//! 栅格时间序列 — 对多期 RasterBand 做逐像素趋势分析。
//!
//! 典型用法：
//! ```rust,ignore
//! use geo_temporal::raster_ts::RasterTimeSeries;
//!
//! let mut ts = RasterTimeSeries::new();
//! ts.add(2020, ndvi_2020);
//! ts.add(2021, ndvi_2021);
//! ts.add(2025, ndvi_2025);
//!
//! let trend_map = ts.pixelwise_trend()?;  // 每个像素的 MK τ
//! let change = ts.change_detection(2020, 2025, 0.1)?;  // 改善/退化图
//! ```

use geo_core::errors::{GeoError, GeoResult};
use geo_raster::RasterBand;
use std::collections::BTreeMap;

use crate::trend::mann_kendall;

/// 单个时间步。
#[derive(Debug, Clone)]
pub struct TimeStep {
    /// 年份。
    pub year: u16,
    /// 栅格波段。
    pub band: RasterBand,
}

/// 栅格时间序列 — 多期 RasterBand 的逐像素分析。
#[derive(Default)]
pub struct RasterTimeSeries {
    steps: BTreeMap<u16, RasterBand>,
    rows: usize,
    cols: usize,
}


impl RasterTimeSeries {
    /// 创建空序列。
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加一期栅格。
    ///
    /// 所有栅格必须有相同尺寸；不同尺寸会返回错误。
    pub fn add(&mut self, year: u16, band: RasterBand) -> GeoResult<()> {
        if self.steps.is_empty() {
            self.cols = band.cols;
            self.rows = band.rows;
        } else if band.cols != self.cols || band.rows != self.rows {
            return Err(GeoError::Validation(format!(
                "band size mismatch: expected {}×{}, got {}×{}",
                self.cols, self.rows, band.cols, band.rows
            )));
        }
        self.steps.insert(year, band);
        Ok(())
    }

    /// 时间步数量。
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// 逐像素 Mann-Kendall τ。
    ///
    /// 返回一个 RasterBand，每个像素值是 τ ∈ [-1, 1]。
    pub fn pixelwise_trend(&self) -> GeoResult<RasterBand> {
        if self.steps.len() < 4 {
            return Err(GeoError::invalid_input("time_steps", "need at least 4"));
        }

        let years: Vec<u16> = self.steps.keys().copied().collect();
        let n_pixels = self.cols * self.rows;
        let mut tau_values = vec![0.0f64; n_pixels];
        let nodata = f64::NAN;

        for (px, tau_out) in tau_values.iter_mut().enumerate() {
            let values: Vec<f64> = years.iter()
                .map(|y| self.steps[y].data[px])
                .collect();

            // 跳过含 nodata 的像素
            if values.iter().any(|v| v.is_nan() || *v == self.steps[&years[0]].nodata) {
                *tau_out = nodata;
                continue;
            }

            let (tau, _p) = mann_kendall(&values);
            *tau_out = tau;
        }

        Ok(RasterBand::new("mk_tau", self.rows, self.cols, tau_values, nodata))
    }

    /// 逐像素变化检测（两年对比差值）。
    ///
    /// 返回分类图：
    /// -  1 = 显著改善 (diff > threshold)
    /// - -1 = 显著退化 (diff < -threshold)
    /// -  0 = 无显著变化
    pub fn change_detection(
        &self,
        year_baseline: u16,
        year_target: u16,
        threshold: f64,
    ) -> GeoResult<RasterBand> {
        let baseline = self.steps.get(&year_baseline)
            .ok_or_else(|| GeoError::not_found("year", year_baseline.to_string()))?;
        let target = self.steps.get(&year_target)
            .ok_or_else(|| GeoError::Validation(format!("year {year_target} not found")))?;

        let n = self.cols * self.rows;
        let mut classes = vec![0.0f64; n];
        let nodata = f64::NAN;

        for (px, cls) in classes.iter_mut().enumerate() {
            let a = baseline.data[px];
            let b = target.data[px];
            if a.is_nan() || b.is_nan() || a == baseline.nodata || b == target.nodata {
                *cls = nodata;
                continue;
            }
            let diff = b - a;
            if diff > threshold { *cls = 1.0; }
            else if diff < -threshold { *cls = -1.0; }
            else { *cls = 0.0; }
        }

        Ok(RasterBand::new("change", self.rows, self.cols, classes, nodata))
    }

    /// 逐年统计（每年均值 + 有效像素数）。
    pub fn yearly_stats(&self) -> Vec<YearlyStats> {
        self.steps.iter().map(|(&year, band)| {
            let valid: Vec<f64> = band.data.iter()
                .filter(|&&v| !v.is_nan() && v != band.nodata)
                .copied()
                .collect();
            let mean = if valid.is_empty() { None } else {
                Some(valid.iter().sum::<f64>() / valid.len() as f64)
            };
            YearlyStats { year, mean_ndvi: mean, valid_pixels: valid.len() }
        }).collect()
    }

    /// 输出像素级线性回归斜率（°NDVI/yr）。
    pub fn pixelwise_slope(&self) -> GeoResult<RasterBand> {
        let years: Vec<f64> = self.steps.keys().map(|&y| y as f64).collect();
        let n = self.cols * self.rows;
        let mut slopes = vec![0.0f64; n];
        let nodata = f64::NAN;

        let x_mean = years.iter().sum::<f64>() / years.len() as f64;
        let xx_var: f64 = years.iter().map(|&x| (x - x_mean).powi(2)).sum();

        for (px, slope) in slopes.iter_mut().enumerate() {
            let values: Vec<f64> = self.steps.values().map(|b| b.data[px]).collect();
            if values.iter().any(|v| v.is_nan()) {
                *slope = nodata;
                continue;
            }
            let y_mean = values.iter().sum::<f64>() / values.len() as f64;
            let xy_cov: f64 = years.iter().zip(&values).map(|(&x, &y)| (x - x_mean) * (y - y_mean)).sum();
            *slope = xy_cov / xx_var.max(1e-10);
        }

        Ok(RasterBand::new("slope", self.rows, self.cols, slopes, nodata))
    }
}

/// 年度统计。
#[derive(Debug, Clone, serde::Serialize)]
pub struct YearlyStats {
    /// 年份。
    pub year: u16,
    /// 平均 NDVI。
    pub mean_ndvi: Option<f64>,
    /// 有效像素数。
    pub valid_pixels: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_band(data: Vec<f64>) -> RasterBand {
        let w = data.len();
        RasterBand::new("test", w, 1, data, -999.0)
    }

    #[test]
    fn test_pixelwise_trend() {
        let mut ts = RasterTimeSeries::new();
        // 4 年，4 像素，逐年递增
        ts.add(2020, make_band(vec![0.30, 0.40, 0.50, 0.60])).unwrap();
        ts.add(2021, make_band(vec![0.35, 0.42, 0.52, 0.61])).unwrap();
        ts.add(2022, make_band(vec![0.40, 0.44, 0.54, 0.62])).unwrap();
        ts.add(2023, make_band(vec![0.45, 0.46, 0.56, 0.63])).unwrap();

        let tau_band = ts.pixelwise_trend().unwrap();
        // 所有像素都应正趋势
        for v in &tau_band.data {
            assert!(*v > 0.0, "expected positive τ, got {v}");
        }
    }

    #[test]
    fn test_change_detection() {
        let mut ts = RasterTimeSeries::new();
        ts.add(2020, make_band(vec![0.30, 0.40, 0.50])).unwrap();
        ts.add(2025, make_band(vec![0.15, 0.42, 0.70])).unwrap();

        let change = ts.change_detection(2020, 2025, 0.1).unwrap();
        // 0:退化(-0.15)  1:无变化(+0.02)  2:改善(+0.20)
        assert_eq!(change.data[0], -1.0); // 退化
        assert_eq!(change.data[1], 0.0);  // 无变化
        assert_eq!(change.data[2], 1.0);  // 改善
    }

    #[test]
    fn test_yearly_stats() {
        let mut ts = RasterTimeSeries::new();
        ts.add(2020, make_band(vec![0.3, -999.0, 0.5])).unwrap();
        ts.add(2021, make_band(vec![0.4, 0.6, -999.0])).unwrap();

        let stats = ts.yearly_stats();
        assert_eq!(stats.len(), 2);
        assert_eq!(stats[0].valid_pixels, 2); // nodata 被排除
        assert_eq!(stats[1].valid_pixels, 2);
    }

    #[test]
    fn test_pixelwise_slope() {
        let mut ts = RasterTimeSeries::new();
        ts.add(2020, make_band(vec![0.30, 0.50])).unwrap();
        ts.add(2021, make_band(vec![0.35, 0.45])).unwrap();
        ts.add(2022, make_band(vec![0.40, 0.40])).unwrap();

        let slope = ts.pixelwise_slope().unwrap();
        assert!(slope.data[0] > 0.0); // 上升
        assert!(slope.data[1] < 0.0); // 下降
    }
}
