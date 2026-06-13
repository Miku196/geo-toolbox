//! 二维栅格数据基类。
//!
//! 轻量级内存栅格，支持从 GeoJSON 格式的 2D 数组构造、
//! 基本像素访问和统计计算。

use serde::{Deserialize, Serialize};
use std::fmt;

/// 内存中的单波段栅格数据。
///
/// 行优先存储（row-major），data[row * cols + col]。
#[derive(Clone, Serialize, Deserialize)]
pub struct RasterBand {
    /// 波段名称（如 "B4", "B8", "NDVI"）。
    pub name: String,
    /// 行数。
    pub rows: usize,
    /// 列数。
    pub cols: usize,
    /// 无数据值。
    pub nodata: f64,
    /// 像素值，行优先。
    pub data: Vec<f64>,
}

impl RasterBand {
    /// 从 Vec 创建栅格。
    pub fn new(
        name: impl Into<String>,
        rows: usize,
        cols: usize,
        data: Vec<f64>,
        nodata: f64,
    ) -> Self {
        Self {
            name: name.into(),
            rows,
            cols,
            nodata,
            data,
        }
    }

    /// 创建全零栅格。
    pub fn zeros(name: impl Into<String>, rows: usize, cols: usize) -> Self {
        Self {
            name: name.into(),
            rows,
            cols,
            data: vec![0.0; rows * cols],
            nodata: f64::NAN,
        }
    }

    /// 获取像素值。
    #[inline]
    pub fn get(&self, row: usize, col: usize) -> f64 {
        self.data[row * self.cols + col]
    }

    /// 设置像素值。
    #[inline]
    pub fn set(&mut self, row: usize, col: usize, value: f64) {
        self.data[row * self.cols + col] = value;
    }

    /// 是否为有效值。
    #[inline]
    pub fn is_valid(&self, row: usize, col: usize) -> bool {
        let v = self.get(row, col);
        !v.is_nan() && (v != self.nodata)
    }

    /// 总像素数。
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// 有效像素数。
    pub fn valid_count(&self) -> usize {
        self.data
            .iter()
            .filter(|v| !v.is_nan() && **v != self.nodata)
            .count()
    }

    /// 最小值。
    pub fn min(&self) -> Option<f64> {
        self.data
            .iter()
            .filter(|v| !v.is_nan() && **v != self.nodata)
            .cloned()
            .fold(None, |acc: Option<f64>, x| {
                Some(acc.map_or(x, |a| a.min(x)))
            })
    }

    /// 最大值。
    pub fn max(&self) -> Option<f64> {
        self.data
            .iter()
            .filter(|v| !v.is_nan() && **v != self.nodata)
            .cloned()
            .fold(None, |acc: Option<f64>, x| {
                Some(acc.map_or(x, |a| a.max(x)))
            })
    }

    /// 平均值。
    pub fn mean(&self) -> Option<f64> {
        let (sum, count) = self
            .data
            .iter()
            .filter(|v| !v.is_nan() && **v != self.nodata)
            .fold((0.0, 0usize), |(s, c), v| (s + v, c + 1));
        if count > 0 {
            Some(sum / count as f64)
        } else {
            None
        }
    }

    /// 标准差。
    pub fn stddev(&self) -> Option<f64> {
        let mean = self.mean()?;
        let valid: Vec<f64> = self
            .data
            .iter()
            .filter(|v| !v.is_nan() && **v != self.nodata)
            .cloned()
            .collect();
        if valid.len() < 2 {
            return Some(0.0);
        }
        let variance =
            valid.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (valid.len() - 1) as f64;
        Some(variance.sqrt())
    }
}

impl fmt::Debug for RasterBand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RasterBand")
            .field("name", &self.name)
            .field("rows", &self.rows)
            .field("cols", &self.cols)
            .field("valid_pixels", &self.valid_count())
            .field("min", &self.min())
            .field("max", &self.max())
            .field("mean", &self.mean())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raster_basics() {
        let band = RasterBand::new("test", 2, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], -999.0);
        assert_eq!(band.get(0, 0), 1.0);
        assert_eq!(band.get(1, 2), 6.0);
        assert_eq!(band.valid_count(), 6);
    }

    #[test]
    fn test_raster_stats() {
        let band = RasterBand::new("test", 2, 2, vec![1.0, 2.0, 3.0, 4.0], f64::NAN);
        assert_eq!(band.min(), Some(1.0));
        assert_eq!(band.max(), Some(4.0));
        assert_eq!(band.mean(), Some(2.5));
    }

    #[test]
    fn test_raster_nodata_skip() {
        let band = RasterBand::new("test", 1, 4, vec![1.0, -999.0, 3.0, -999.0], -999.0);
        assert_eq!(band.valid_count(), 2);
        assert_eq!(band.min(), Some(1.0));
        assert_eq!(band.max(), Some(3.0));
    }

    #[test]
    fn test_raster_nan_skip() {
        let band = RasterBand::new("test", 1, 3, vec![1.0, f64::NAN, 3.0], -999.0);
        assert_eq!(band.valid_count(), 2);
    }
}
