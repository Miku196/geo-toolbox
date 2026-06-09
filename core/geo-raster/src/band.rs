//! 波段逐像素运算。
//!
//! 支持加/减/乘/除/阈值判定，自动跳过 nodata 像素。

use crate::grid::RasterBand;
use geo_core::errors::{GeoError, GeoResult};

/// 两波段逐像素求和。
///
/// 要求行列数一致。结果波段以 `out_name` 命名。
/// nodata 像素会被跳过（结果中也标记为 nodata）。
pub fn band_add(a: &RasterBand, b: &RasterBand, out_name: &str) -> GeoResult<RasterBand> {
    check_same_size(a, b)?;
    let mut out = a.clone();
    out.name = out_name.to_string();
    for i in 0..out.data.len() {
        if a.data[i] != a.nodata && !a.data[i].is_nan()
            && b.data[i] != b.nodata && !b.data[i].is_nan()
        {
            out.data[i] = a.data[i] + b.data[i];
        } else {
            out.data[i] = out.nodata;
        }
    }
    Ok(out)
}

/// 两波段逐像素相减。
pub fn band_sub(a: &RasterBand, b: &RasterBand, out_name: &str) -> GeoResult<RasterBand> {
    check_same_size(a, b)?;
    let mut out = a.clone();
    out.name = out_name.to_string();
    for i in 0..out.data.len() {
        if a.data[i] != a.nodata && !a.data[i].is_nan()
            && b.data[i] != b.nodata && !b.data[i].is_nan()
        {
            out.data[i] = a.data[i] - b.data[i];
        } else {
            out.data[i] = out.nodata;
        }
    }
    Ok(out)
}

/// 两波段逐像素相除（返回比值，0/0 → nodata）。
pub fn band_div(a: &RasterBand, b: &RasterBand, out_name: &str) -> GeoResult<RasterBand> {
    check_same_size(a, b)?;
    let mut out = a.clone();
    out.name = out_name.to_string();
    for i in 0..out.data.len() {
        if a.data[i] != a.nodata && !a.data[i].is_nan()
            && b.data[i] != b.nodata && !b.data[i].is_nan()
            && b.data[i] != 0.0
        {
            out.data[i] = a.data[i] / b.data[i];
        } else {
            out.data[i] = out.nodata;
        }
    }
    Ok(out)
}

/// 阈值二值化：value ≥ threshold → 1.0，否则 → 0.0。
pub fn band_threshold(band: &RasterBand, threshold: f64, out_name: &str) -> RasterBand {
    let mut out = band.clone();
    out.name = out_name.to_string();
    for i in 0..out.data.len() {
        if band.data[i] != band.nodata && !band.data[i].is_nan() {
            out.data[i] = if band.data[i] >= threshold { 1.0 } else { 0.0 };
        } else {
            out.data[i] = out.nodata;
        }
    }
    out
}

fn check_same_size(a: &RasterBand, b: &RasterBand) -> GeoResult<()> {
    if a.rows != b.rows || a.cols != b.cols {
        return Err(GeoError::Validation(format!(
            "Raster size mismatch: {}x{} vs {}x{}",
            a.rows, a.cols, b.rows, b.cols
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_band(name: &str, data: Vec<f64>) -> RasterBand {
        RasterBand::new(name, 1, data.len(), data, -999.0)
    }

    #[test]
    fn test_band_add() {
        let a = make_band("a", vec![1.0, 2.0, 3.0]);
        let b = make_band("b", vec![4.0, 5.0, 6.0]);
        let result = band_add(&a, &b, "sum").unwrap();
        assert_eq!(result.get(0, 0), 5.0);
        assert_eq!(result.get(0, 2), 9.0);
    }

    #[test]
    fn test_band_sub() {
        let a = make_band("a", vec![10.0, 20.0]);
        let b = make_band("b", vec![3.0, 5.0]);
        let result = band_sub(&a, &b, "diff").unwrap();
        assert_eq!(result.get(0, 0), 7.0);
    }

    #[test]
    fn test_band_div_nodata() {
        let a = make_band("a", vec![1.0, 2.0]);
        let b = make_band("b", vec![0.0, -999.0]);
        let result = band_div(&a, &b, "ratio").unwrap();
        assert_eq!(result.get(0, 0), -999.0); // 0/0 → nodata
        assert_eq!(result.get(0, 1), -999.0); // nodata in b → nodata
    }

    #[test]
    fn test_threshold() {
        let a = make_band("a", vec![0.3, 0.6, 0.5]);
        let t = band_threshold(&a, 0.5, "threshold");
        assert_eq!(t.get(0, 0), 0.0);
        assert_eq!(t.get(0, 1), 1.0);
        assert_eq!(t.get(0, 2), 1.0);
    }
}
