//! 分区统计（Zonal Statistics）。
//!
//! 在给定的多边形区域内对栅格数据进行统计：
//! - 计数、均值、最小值、最大值、总和
//! - 按阈值分类的面积占比
//!
//! 当前实现基于边界框快速近似（简化版）：

use geo_core::errors::GeoResult;
use geo_core::types::BBox;
use serde::{Deserialize, Serialize};

/// 单个分区的统计结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZonalResult {
    /// 区域名称/标识。
    pub zone_name: String,
    /// 区域内有效像素数。
    pub pixel_count: usize,
    /// 区域内栅格均值。
    pub mean: f64,
    /// 最小值。
    pub min: f64,
    /// 最大值。
    pub max: f64,
    /// 总和。
    pub sum: f64,
    /// 像素值 ≥ healthy_threshold 的比例。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub healthy_ratio: Option<f64>,
    /// 像素值 ≤ degraded_threshold 的比例。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub degraded_ratio: Option<f64>,
}

/// 分区统计配置。
#[derive(Debug, Clone)]
pub struct ZonalStats<'a> {
    /// 栅格数据（展平为一维 f64 数组）。
    pub data: &'a [f64],
    /// 栅格行数。
    pub rows: usize,
    /// 栅格列数。
    pub cols: usize,
    /// 无数据值。
    pub nodata: f64,
    /// 每个像素对应的地理范围（用于判断像素是否在 AOI 内）。
    /// bbox 覆盖整个栅格范围。
    pub bbox: BBox,
}

impl<'a> ZonalStats<'a> {
    /// 在指定边界框内计算统计。
    ///
    /// 使用简化方法：遍历所有像素，判断像素中心点是否在 AOI 的 bbox 内。
    /// 适合矩形 AOI 或作为快速近似。精确分区统计需要多边形-像素相交检测。
    pub fn compute(&self, aoi_bbox: &BBox, zone_name: &str) -> GeoResult<ZonalResult> {
        let pixel_w = self.bbox.width() / self.cols as f64;
        let pixel_h = self.bbox.height() / self.rows as f64;

        let mut count = 0usize;
        let mut min_val = f64::MAX;
        let mut max_val = f64::MIN;
        let mut sum = 0.0f64;

        for row in 0..self.rows {
            let y = self.bbox.max_y - (row as f64 + 0.5) * pixel_h;
            if y < aoi_bbox.min_y || y > aoi_bbox.max_y {
                continue;
            }
            for col in 0..self.cols {
                let x = self.bbox.min_x + (col as f64 + 0.5) * pixel_w;
                if x < aoi_bbox.min_x || x > aoi_bbox.max_x {
                    continue;
                }
                let v = self.data[row * self.cols + col];
                if v == self.nodata || v.is_nan() {
                    continue;
                }
                count += 1;
                sum += v;
                if v < min_val {
                    min_val = v;
                }
                if v > max_val {
                    max_val = v;
                }
            }
        }

        if count == 0 {
            // 即使无像素也返回零值（AOI 可能太小或数据缺失）
            return Ok(ZonalResult {
                zone_name: zone_name.to_string(),
                pixel_count: 0,
                mean: 0.0,
                min: 0.0,
                max: 0.0,
                sum: 0.0,
                healthy_ratio: Some(0.0),
                degraded_ratio: Some(0.0),
            });
        }

        let mean = sum / count as f64;

        // 阈值分类
        let healthy_count = self
            .data
            .iter()
            .filter(|v| **v != self.nodata && !v.is_nan() && **v >= 0.5)
            .count();
        let degraded_count = self
            .data
            .iter()
            .filter(|v| **v != self.nodata && !v.is_nan() && **v <= 0.2)
            .count();
        let total_valid = self
            .data
            .iter()
            .filter(|v| **v != self.nodata && !v.is_nan())
            .count();

        let healthy_ratio = if total_valid > 0 {
            Some(healthy_count as f64 / total_valid as f64)
        } else {
            None
        };

        let degraded_ratio = if total_valid > 0 {
            Some(degraded_count as f64 / total_valid as f64)
        } else {
            None
        };

        Ok(ZonalResult {
            zone_name: zone_name.to_string(),
            pixel_count: count,
            mean,
            min: min_val,
            max: max_val,
            sum,
            healthy_ratio,
            degraded_ratio,
        })
    }
}

/// 便捷函数：对栅格在 AOI bbox 内做分区统计。
pub fn zonal_stats(
    data: &[f64],
    rows: usize,
    cols: usize,
    nodata: f64,
    raster_bbox: BBox,
    aoi_bbox: &BBox,
    zone_name: &str,
) -> GeoResult<ZonalResult> {
    let stats = ZonalStats {
        data,
        rows,
        cols,
        nodata,
        bbox: raster_bbox,
    };
    stats.compute(aoi_bbox, zone_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zonal_stats_basic() {
        // 3x3 栅格，全覆盖
        let data = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];
        let raster_bbox = BBox::new(103.0, 30.0, 104.0, 31.0);
        let aoi_bbox = BBox::new(103.0, 30.0, 104.0, 31.0); // 全覆盖

        let result = zonal_stats(&data, 3, 3, -999.0, raster_bbox, &aoi_bbox, "full").unwrap();

        assert_eq!(result.pixel_count, 9);
        assert!((result.mean - 0.5).abs() < 0.01);
        assert_eq!(result.min, 0.1);
        assert_eq!(result.max, 0.9);
    }

    #[test]
    fn test_zonal_stats_partial() {
        // 只覆盖左上角 2x2
        let data = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];
        let raster_bbox = BBox::new(103.0, 30.0, 104.0, 31.0);
        let aoi_bbox = BBox::new(103.0, 30.33, 103.67, 31.0);

        let result = zonal_stats(&data, 3, 3, -999.0, raster_bbox, &aoi_bbox, "partial").unwrap();

        assert!(result.pixel_count > 0 && result.pixel_count < 9);
    }

    #[test]
    fn test_zonal_stats_nodata() {
        let data = vec![0.5, -999.0, 0.8, 0.3, -999.0, -999.0, 0.7, 0.6, 0.9];
        let raster_bbox = BBox::new(0.0, 0.0, 1.0, 1.0);
        let aoi_bbox = BBox::new(0.0, 0.0, 1.0, 1.0);

        let result =
            zonal_stats(&data, 3, 3, -999.0, raster_bbox, &aoi_bbox, "with_nodata").unwrap();

        assert_eq!(result.pixel_count, 6);
        assert_eq!(result.min, 0.3);
        assert_eq!(result.max, 0.9);
    }

    #[test]
    fn test_zonal_stats_empty() {
        let data = vec![-999.0, -999.0, -999.0, -999.0];
        let raster_bbox = BBox::new(0.0, 0.0, 1.0, 1.0);
        let aoi_bbox = BBox::new(0.0, 0.0, 1.0, 1.0);

        let result = zonal_stats(&data, 2, 2, -999.0, raster_bbox, &aoi_bbox, "empty").unwrap();
        assert_eq!(result.pixel_count, 0);
        assert_eq!(result.mean, 0.0);
    }

    #[test]
    fn test_zonalstats_compute_direct() {
        let data = vec![0.1, 0.3, 0.6, 0.8, 0.5, 0.4, 0.3, 0.2, 0.6];
        let bbox = BBox::new(0.0, 0.0, 3.0, 3.0);
        let zs = ZonalStats {
            data: &data,
            rows: 3,
            cols: 3,
            nodata: -999.0,
            bbox,
        };
        let result = zs.compute(&zs.bbox.clone(), "full").unwrap();
        assert_eq!(result.pixel_count, 9);
        assert_eq!(result.zone_name, "full");
        assert!((result.mean - 0.422222).abs() < 0.01);
        assert!((result.min - 0.1).abs() < 0.01);
        assert!((result.max - 0.8).abs() < 0.01);
        assert!((result.sum - 3.8).abs() < 0.01);
        // healthy_ratio and degraded_ratio check
        assert!(result.healthy_ratio.is_some());
        assert!(result.degraded_ratio.is_some());
    }
}
