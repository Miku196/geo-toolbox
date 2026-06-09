//! NDVI（归一化植被指数）计算。
//!
//! NDVI = (NIR - RED) / (NIR + RED)
//!
//! 值域 [-1, 1]，正值表示植被覆盖，负值表示水体/裸地/云。

use crate::band;
use crate::grid::RasterBand;
use geo_core::errors::GeoResult;
use serde::{Deserialize, Serialize};

/// NDVI 计算结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NdviResult {
    /// 输出的 NDVI 波段。
    pub ndvi: RasterBand,
    /// NDVI 均值。
    pub mean_ndvi: Option<f64>,
    /// 健康植被比例（NDVI ≥ healthy_threshold 的像素占比）。
    pub healthy_ratio: Option<f64>,
    /// 退化植被比例（NDVI ≤ degraded_threshold 的像素占比）。
    pub degraded_ratio: Option<f64>,
    /// 有效像素数。
    pub valid_pixels: usize,
}

/// 从红波段（RED）和近红外波段（NIR）计算 NDVI。
///
/// 自动跳过两组波段中任一为 nodata 的像素。
pub fn compute_ndvi(red: &RasterBand, nir: &RasterBand) -> GeoResult<NdviResult> {
    let sum = band::band_add(nir, red, "NIR+RED")?;
    let diff = band::band_sub(nir, red, "NIR-RED")?;
    let ndvi = band::band_div(&diff, &sum, "NDVI")?;

    let mean_ndvi = ndvi.mean();
    let valid_pixels = ndvi.valid_count();

    let healthy_ratio = if valid_pixels > 0 {
        let healthy_count = ndvi.data.iter()
            .filter(|v| !v.is_nan() && **v != ndvi.nodata && **v >= 0.5)
            .count();
        Some(healthy_count as f64 / valid_pixels as f64)
    } else {
        None
    };

    let degraded_ratio = if valid_pixels > 0 {
        let degraded_count = ndvi.data.iter()
            .filter(|v| !v.is_nan() && **v != ndvi.nodata && **v <= 0.2)
            .count();
        Some(degraded_count as f64 / valid_pixels as f64)
    } else {
        None
    };

    Ok(NdviResult {
        ndvi,
        mean_ndvi,
        healthy_ratio,
        degraded_ratio,
        valid_pixels,
    })
}

/// 计算两期 NDVI 的差值（后减前）。
///
/// 正值表示植被增加，负值表示植被退化。
pub fn ndvi_difference(previous: &NdviResult, current: &NdviResult) -> GeoResult<RasterBand> {
    band::band_sub(&current.ndvi, &previous.ndvi, "NDVI_diff")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_band(data: Vec<f64>) -> RasterBand {
        RasterBand::new("band", 1, data.len(), data, -999.0)
    }

    #[test]
    fn test_ndvi_compute() {
        // Typical values: RED=0.05, NIR=0.50 → NDVI ≈ 0.818
        let red = make_band(vec![0.05, 0.10, 0.40, -999.0]);
        let nir = make_band(vec![0.50, 0.15, 0.35, 0.60]);

        let result = compute_ndvi(&red, &nir).unwrap();
        // (0.50-0.05)/(0.50+0.05) = 0.45/0.55 ≈ 0.818
        assert!((result.ndvi.get(0, 0) - 0.818).abs() < 0.01);
        // (0.15-0.10)/(0.15+0.10) = 0.05/0.25 = 0.2
        assert!((result.ndvi.get(0, 1) - 0.2).abs() < 0.01);
        // (0.35-0.40)/(0.35+0.40) = -0.05/0.75 ≈ -0.067
        assert!((result.ndvi.get(0, 2) + 0.067).abs() < 0.01);
        // Pixel 3: RED = nodata → output nodata
        assert_eq!(result.ndvi.get(0, 3), -999.0);

        assert_eq!(result.valid_pixels, 3);
        assert!(result.healthy_ratio.is_some());
        assert_eq!(result.healthy_ratio.unwrap(), 1.0 / 3.0); // only pixel 0 ≥ 0.5
    }

    #[test]
    fn test_ndvi_difference() {
        let red1 = make_band(vec![0.10, 0.20]);
        let nir1 = make_band(vec![0.40, 0.30]);
        let prev = compute_ndvi(&red1, &nir1).unwrap();

        let red2 = make_band(vec![0.05, 0.25]);
        let nir2 = make_band(vec![0.50, 0.35]);
        let curr = compute_ndvi(&red2, &nir2).unwrap();

        let diff = ndvi_difference(&prev, &curr).unwrap();
        // prev[0] = (0.30)/(0.50)=0.6, curr[0] = (0.45)/(0.55)≈0.818, diff ≈ +0.218
        assert!(diff.get(0, 0) > 0.0, "植被应该增加");
        // prev[1] = (0.10)/(0.50)=0.2, curr[1] = (0.10)/(0.60)≈0.167, diff ≈ -0.033
        assert!(diff.get(0, 1) < 0.0, "植被应该退化");
    }
}
