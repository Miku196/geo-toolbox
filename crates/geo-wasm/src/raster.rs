//! Browser-side raster operations — NDVI, band arithmetic, resampling, zonal stats.
//!
//! All functions receive raw pixel data (plain Vec<f64>) plus dimensions
//! and return JSON strings for JS interop.
//! No RasterBand struct crossing the WASM boundary.
#![allow(non_snake_case)]

use geo_core::errors::GeoResult;
use wasm_bindgen::prelude::*;

// ── NDVI ────────────────────────────────────────────────────────

/// Compute NDVI from red and NIR band data.
///
/// ## Parameters
/// - `red_data`: flat array of red band pixel values (row-major)
/// - `nir_data`: flat array of NIR band pixel values (row-major)
/// - `rows`: number of rows
/// - `cols`: number of columns
///
/// ## Returns
/// JSON string with `ndvi` (band data), `mean_ndvi`, `healthy_ratio`, `degraded_ratio`.
#[wasm_bindgen(js_name = computeNdvi)]
pub fn compute_ndvi(
    red_data: Vec<f64>,
    nir_data: Vec<f64>,
    rows: usize,
    cols: usize,
) -> Result<String, JsValue> {
    compute_ndvi_inner(red_data, nir_data, rows, cols)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

fn compute_ndvi_inner(
    red_data: Vec<f64>,
    nir_data: Vec<f64>,
    rows: usize,
    cols: usize,
) -> GeoResult<String> {
    let red = geo_raster::RasterBand::new("red", rows, cols, red_data, -9999.0);
    let nir = geo_raster::RasterBand::new("nir", rows, cols, nir_data, -9999.0);
    let result = geo_raster::ndvi::compute_ndvi(&red, &nir)
        .map_err(|e| geo_core::errors::GeoError::Other(e.to_string()))?;
    serde_json::to_string(&serde_json::json!({
        "ndvi_data": result.ndvi.data,
        "rows": result.ndvi.rows,
        "cols": result.ndvi.cols,
        "mean_ndvi": result.mean_ndvi,
        "healthy_ratio": result.healthy_ratio,
        "degraded_ratio": result.degraded_ratio,
        "valid_pixels": result.valid_pixels,
    }))
    .map_err(geo_core::errors::GeoError::Serde)
}

/// Compute NDVI difference between two timepoints.
#[wasm_bindgen(js_name = ndviDifference)]
pub fn ndvi_difference(
    prev_ndvi_data: Vec<f64>,
    prev_rows: usize,
    prev_cols: usize,
    curr_ndvi_data: Vec<f64>,
    curr_rows: usize,
    curr_cols: usize,
) -> Result<String, JsValue> {
    ndvi_difference_inner(
        prev_ndvi_data,
        prev_rows,
        prev_cols,
        curr_ndvi_data,
        curr_rows,
        curr_cols,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))
}

fn ndvi_difference_inner(
    prev_ndvi_data: Vec<f64>,
    prev_rows: usize,
    prev_cols: usize,
    curr_ndvi_data: Vec<f64>,
    curr_rows: usize,
    curr_cols: usize,
) -> GeoResult<String> {
    use geo_raster::ndvi::NdviResult;

    let prev_band =
        geo_raster::RasterBand::new("prev", prev_rows, prev_cols, prev_ndvi_data, -9999.0);
    let curr_band =
        geo_raster::RasterBand::new("curr", curr_rows, curr_cols, curr_ndvi_data, -9999.0);

    let prev_result = NdviResult {
        ndvi: prev_band,
        mean_ndvi: None,
        healthy_ratio: None,
        degraded_ratio: None,
        valid_pixels: 0,
    };
    let curr_result = NdviResult {
        ndvi: curr_band,
        mean_ndvi: None,
        healthy_ratio: None,
        degraded_ratio: None,
        valid_pixels: 0,
    };

    let diff = geo_raster::ndvi::ndvi_difference(&prev_result, &curr_result)
        .map_err(|e| geo_core::errors::GeoError::Other(e.to_string()))?;
    serde_json::to_string(&serde_json::json!({
        "diff_data": diff.data,
        "rows": diff.rows,
        "cols": diff.cols,
    }))
    .map_err(geo_core::errors::GeoError::Serde)
}

// ── Band arithmetic ─────────────────────────────────────────────

fn build_band(name: &str, data: Vec<f64>, rows: usize, cols: usize) -> geo_raster::RasterBand {
    geo_raster::RasterBand::new(name, rows, cols, data, -9999.0)
}

fn band_result_to_json_inner(band: &geo_raster::RasterBand) -> GeoResult<String> {
    serde_json::to_string(&serde_json::json!({
        "data": band.data,
        "rows": band.rows,
        "cols": band.cols,
        "nodata": band.nodata,
    }))
    .map_err(geo_core::errors::GeoError::Serde)
}

macro_rules! band_op {
    ($wasm_name:ident, $inner_name:ident, $op:path) => {
        #[wasm_bindgen(js_name = $wasm_name)]
        pub fn $wasm_name(
            a_data: Vec<f64>,
            a_rows: usize,
            a_cols: usize,
            b_data: Vec<f64>,
            b_rows: usize,
            b_cols: usize,
        ) -> Result<String, JsValue> {
            $inner_name(a_data, a_rows, a_cols, b_data, b_rows, b_cols)
                .map_err(|e| JsValue::from_str(&e.to_string()))
        }

        fn $inner_name(
            a_data: Vec<f64>,
            a_rows: usize,
            a_cols: usize,
            b_data: Vec<f64>,
            b_rows: usize,
            b_cols: usize,
        ) -> GeoResult<String> {
            let a = build_band("a", a_data, a_rows, a_cols);
            let b = build_band("b", b_data, b_rows, b_cols);
            let r = $op(&a, &b, "result")
                .map_err(|e| geo_core::errors::GeoError::Other(e.to_string()))?;
            band_result_to_json_inner(&r)
        }
    };
}

band_op!(bandAdd, band_add_inner, geo_raster::band::band_add);
band_op!(bandSub, band_sub_inner, geo_raster::band::band_sub);
band_op!(bandMul, band_mul_inner, geo_raster::band::band_mul);
band_op!(bandDiv, band_div_inner, geo_raster::band::band_div);

/// Classify each pixel as above/below a threshold. Returns JSON band result.
///
/// Pixels >= threshold become 1.0, pixels < threshold become 0.0.
/// nodata values are preserved.
#[wasm_bindgen(js_name = bandThreshold)]
pub fn band_threshold(
    data: Vec<f64>,
    rows: usize,
    cols: usize,
    threshold: f64,
) -> Result<String, JsValue> {
    band_threshold_inner(data, rows, cols, threshold).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn band_threshold_inner(
    data: Vec<f64>,
    rows: usize,
    cols: usize,
    threshold: f64,
) -> GeoResult<String> {
    let band = build_band("band", data, rows, cols);
    let r = geo_raster::band::band_threshold(&band, threshold, "result");
    band_result_to_json_inner(&r)
}

// ── Resampling ──────────────────────────────────────────────────

/// Nearest-neighbor resampling (fast, blocky). Returns flat Vec<f64> row-major.
///
/// Useful for quick downsampling where precision isn't critical.
#[wasm_bindgen(js_name = resampleNearest)]
pub fn resample_nearest(
    data: Vec<f64>,
    src_rows: usize,
    src_cols: usize,
    dst_rows: usize,
    dst_cols: usize,
    nodata: Option<f64>,
) -> Vec<f64> {
    geo_raster::resample::resample_nearest(&data, src_rows, src_cols, dst_rows, dst_cols, nodata)
}

/// Bicubic resampling (smooth, higher quality). Returns flat Vec<f64> row-major.
///
/// Uses cubic interpolation for smoother results at the cost of computation time.
/// Preferred for continuous data (DEM, temperature, NDVI).
#[wasm_bindgen(js_name = resampleCubic)]
pub fn resample_cubic(
    data: Vec<f64>,
    src_rows: usize,
    src_cols: usize,
    dst_rows: usize,
    dst_cols: usize,
    nodata: Option<f64>,
) -> Vec<f64> {
    geo_raster::resample::resample_cubic(&data, src_rows, src_cols, dst_rows, dst_cols, nodata)
}

// ── Zonal Statistics ────────────────────────────────────────────

/// Compute zonal statistics on raster values grouped by zone IDs.
///
/// ## Parameters
/// - `values`: flat array of raster pixel values
/// - `zones`: flat array of zone IDs (1-indexed, 0 = nodata/ignored)
/// - `num_zones`: number of zones
/// - `nodata`: optional nodata value to filter
///
/// ## Returns
/// JSON object with per-zone stats: `{zones: [{count,min,max,mean,stddev,sum}, ...]}`
#[wasm_bindgen(js_name = computeZonalStats)]
pub fn compute_zonal_stats(
    values: Vec<f64>,
    zones: Vec<u32>,
    num_zones: usize,
    nodata: Option<f64>,
) -> Result<String, JsValue> {
    compute_zonal_stats_inner(values, zones, num_zones, nodata)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

fn compute_zonal_stats_inner(
    values: Vec<f64>,
    zones: Vec<u32>,
    num_zones: usize,
    nodata: Option<f64>,
) -> GeoResult<String> {
    let result = geo_raster::zonal_stats(&values, &zones, num_zones, nodata);
    let zones_json: Vec<serde_json::Value> = result
        .zones
        .iter()
        .map(|z| {
            serde_json::json!({
                "count": z.count,
                "min": z.min,
                "max": z.max,
                "mean": z.mean,
                "stddev": z.stddev,
                "sum": z.sum,
            })
        })
        .collect();
    serde_json::to_string(&serde_json::json!({ "zones": zones_json }))
        .map_err(geo_core::errors::GeoError::Serde)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ndvi() {
        let r = (0..100).map(|i| i as f64 / 100.0).collect::<Vec<_>>();
        let n = r.iter().map(|x| 0.5 + x * 0.5).collect::<Vec<_>>();
        let result = compute_ndvi_inner(r, n, 10, 10).unwrap();
        assert!(result.contains("mean_ndvi"));
        assert!(result.contains("ndvi_data"));
    }

    #[test]
    fn test_band_arithmetic() {
        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![0.5, 0.5, 0.5, 0.5];
        let result = band_add_inner(a.clone(), 2, 2, b.clone(), 2, 2).unwrap();
        assert!(result.contains("\"data\""));
        let sub = band_sub_inner(a.clone(), 2, 2, b.clone(), 2, 2).unwrap();
        assert!(sub.contains("\"data\""));
    }

    #[test]
    fn test_resample() {
        let data = vec![1.0, 2.0, 3.0, 4.0];
        let r = resample_nearest(data, 2, 2, 4, 4, None);
        assert_eq!(r.len(), 16);
    }

    #[test]
    fn test_zonal_stats() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let zones = vec![1, 1, 1, 2, 2, 2];
        let result = compute_zonal_stats_inner(values, zones, 2, None).unwrap();
        assert!(result.contains("\"count\""));
        assert!(result.contains("\"mean\""));
    }
}
