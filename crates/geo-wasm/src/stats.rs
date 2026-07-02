//! Browser-side geospatial statistics via geo-stats.
//!
//! Zonal statistics, classification (Jenks / quantile / equal-interval),
//! spatial interpolation (IDW), regression (OLS), clustering (k-means),
//! spatial autocorrelation (Moran's I), and hotspot analysis (Getis-Ord Gi*).

use wasm_bindgen::prelude::*;

fn err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

// ── Zonal Statistics ────────────────────────────────────────────

/// Compute zonal statistics for a raster band within an AOI bounding box.
///
/// Parameters:
/// - `data`: flat f64 array (row-major)
/// - `rows`, `cols`: raster dimensions
/// - `nodata`: no-data value
/// - `minLon`, `minLat`, `maxLon`, `maxLat`: raster extent (WGS84)
/// - `aoiMinLon`, `aoiMinLat`, `aoiMaxLon`, `aoiMaxLat`: AOI bounding box
/// - `zoneName`: label for the result
///
/// Returns JSON:
/// ```json
/// {
///   "zone_name": "...",
///   "pixel_count": 1234,
///   "min": 0.1,
///   "max": 0.9,
///   "mean": 0.45,
///   "sum": 555.3,
///   "healthy_ratio": 0.72,
///   "degraded_ratio": 0.05
/// }
/// ```
#[wasm_bindgen(js_name = computeZonalStatsBbox)]
#[allow(clippy::too_many_arguments)]
pub fn compute_zonal_stats_bbox(
    data: Vec<f64>,
    rows: usize,
    cols: usize,
    nodata: f64,
    min_lon: f64,
    min_lat: f64,
    max_lon: f64,
    max_lat: f64,
    aoi_min_lon: f64,
    aoi_min_lat: f64,
    aoi_max_lon: f64,
    aoi_max_lat: f64,
    zone_name: &str,
) -> Result<String, JsValue> {
    compute_zonal_stats_bbox_inner(
        data,
        rows,
        cols,
        nodata,
        min_lon,
        min_lat,
        max_lon,
        max_lat,
        aoi_min_lon,
        aoi_min_lat,
        aoi_max_lon,
        aoi_max_lat,
        zone_name,
    )
    .map_err(err)
}

fn compute_zonal_stats_bbox_inner(
    data: Vec<f64>,
    rows: usize,
    cols: usize,
    nodata: f64,
    min_lon: f64,
    min_lat: f64,
    max_lon: f64,
    max_lat: f64,
    aoi_min_lon: f64,
    aoi_min_lat: f64,
    aoi_max_lon: f64,
    aoi_max_lat: f64,
    zone_name: &str,
) -> geo_core::errors::GeoResult<String> {
    let raster_bbox = geo_core::types::BBox {
        min_x: min_lon,
        min_y: min_lat,
        max_x: max_lon,
        max_y: max_lat,
    };
    let aoi_bbox = geo_core::types::BBox {
        min_x: aoi_min_lon,
        min_y: aoi_min_lat,
        max_x: aoi_max_lon,
        max_y: aoi_max_lat,
    };
    let result =
        geo_stats::zonal_stats(&data, rows, cols, nodata, raster_bbox, &aoi_bbox, zone_name)?;
    serde_json::to_string(&serde_json::json!({
        "zone_name": result.zone_name,
        "pixel_count": result.pixel_count,
        "min": result.min,
        "max": result.max,
        "mean": result.mean,
        "sum": result.sum,
        "healthy_ratio": result.healthy_ratio,
        "degraded_ratio": result.degraded_ratio,
    }))
    .map_err(geo_core::errors::GeoError::Serde)
}

// ── Classification ──────────────────────────────────────────────

/// Jenks natural breaks classification.
/// Returns JSON: `{"breaks":[0.0, 0.3, 0.7, 1.0], "classes":[0,0,1,2,2,...], "gvf":0.92, "k":3}`
#[wasm_bindgen(js_name = jenksBreaks)]
pub fn jenks_breaks(data: Vec<f64>, k: usize) -> Result<String, JsValue> {
    let result = geo_stats::jenks(&data, k).ok_or_else(|| {
        JsValue::from_str("Jenks classification failed — insufficient data or k too large")
    })?;
    serde_json::to_string(&serde_json::json!({
        "breaks": result.breaks,
        "classes": result.classes,
        "gvf": result.gvf,
        "k": result.k,
    }))
    .map_err(err)
}

/// Quantile breaks classification.
/// Returns JSON: `{"breaks": [0.1, 0.4, 0.8, 1.0]}`
#[wasm_bindgen(js_name = quantileBreaks)]
pub fn quantile_breaks(data: Vec<f64>, k: usize) -> Result<String, JsValue> {
    let breaks = geo_stats::quantile_breaks(&data, k)
        .ok_or_else(|| JsValue::from_str("Quantile classification failed"))?;
    serde_json::to_string(&breaks).map_err(err)
}

/// Equal-interval breaks classification.
/// Returns JSON: `{"breaks": [0.0, 0.25, 0.5, 0.75, 1.0]}`
#[wasm_bindgen(js_name = equalIntervalBreaks)]
pub fn equal_interval_breaks(data: Vec<f64>, k: usize) -> Result<String, JsValue> {
    let breaks = geo_stats::equal_interval_breaks(&data, k)
        .ok_or_else(|| JsValue::from_str("Equal-interval classification failed"))?;
    serde_json::to_string(&breaks).map_err(err)
}

// ── Inverse Distance Weighting (IDW) ────────────────────────────

/// IDW interpolation at a single target point.
/// Returns the interpolated value as a plain number (JSON f64).
#[wasm_bindgen(js_name = idwPoint)]
pub fn idw_point(
    target_x: f64,
    target_y: f64,
    src_x: Vec<f64>,
    src_y: Vec<f64>,
    src_values: Vec<f64>,
    power: f64,
    max_radius: f64,
    min_neighbors: usize,
) -> Result<JsValue, JsValue> {
    let val = idw_point_inner(
        target_x,
        target_y,
        &src_x,
        &src_y,
        &src_values,
        power,
        max_radius,
        min_neighbors,
    )
    .ok_or_else(|| JsValue::from_str("IDW interpolation failed — no neighbors within radius"))?;
    Ok(JsValue::from_f64(val))
}

fn idw_point_inner(
    target_x: f64,
    target_y: f64,
    src_x: &[f64],
    src_y: &[f64],
    src_values: &[f64],
    power: f64,
    max_radius: f64,
    min_neighbors: usize,
) -> Option<f64> {
    geo_stats::idw_point(
        target_x,
        target_y,
        src_x,
        src_y,
        src_values,
        power,
        max_radius,
        min_neighbors,
    )
}

/// IDW interpolation over a regular grid defined by a bounding box.
/// Returns JSON: `{"values": [...], "point_count": 42, "power": 2.0, "max_radius": 1000.0}`
#[wasm_bindgen(js_name = idwGrid)]
pub fn idw_grid(
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    ncols: usize,
    nrows: usize,
    src_x: Vec<f64>,
    src_y: Vec<f64>,
    src_values: Vec<f64>,
    power: f64,
    max_radius: f64,
    min_neighbors: usize,
) -> Result<String, JsValue> {
    let bbox = geo_core::types::BBox {
        min_x,
        min_y,
        max_x,
        max_y,
    };
    let (values, result) = geo_stats::idw_grid(
        &bbox,
        ncols,
        nrows,
        &src_x,
        &src_y,
        &src_values,
        power,
        max_radius,
        min_neighbors,
    );
    serde_json::to_string(&serde_json::json!({
        "values": values,
        "point_count": result.point_count,
        "power": result.power,
        "max_radius": result.max_radius,
    }))
    .map_err(err)
}

// ── Regression (Simple Linear: y = a + bx) ──────────────────────

/// Ordinary Least Squares (OLS) simple linear regression (y = a + bx).
///
/// Returns JSON:
/// ```json
/// {
///   "intercept": 3.0,
///   "slope": 2.0,
///   "r_squared": 1.0,
///   "rmse": 0.0,
///   "n": 5,
///   "intercept_se": 0.0,
///   "slope_se": 0.0
/// }
/// ```
#[wasm_bindgen(js_name = olsRegression)]
pub fn ols_regression(x: Vec<f64>, y: Vec<f64>) -> Result<String, JsValue> {
    let result = geo_stats::ols_regression(&x, &y)
        .ok_or_else(|| JsValue::from_str("OLS regression failed — check dimensions"))?;
    serde_json::to_string(&serde_json::json!({
        "intercept": result.intercept,
        "slope": result.slope,
        "r_squared": result.r_squared,
        "rmse": result.rmse,
        "n": result.n,
        "intercept_se": result.intercept_se,
        "slope_se": result.slope_se,
    }))
    .map_err(err)
}

/// Predict a single y value from a fitted OLS model (y = intercept + slope * x).
/// Returns the predicted y value.
#[wasm_bindgen(js_name = olsPredict)]
pub fn ols_predict(x: f64, intercept: f64, slope: f64) -> f64 {
    let model = geo_stats::OlsResult {
        slope,
        intercept,
        r_squared: 0.0,
        rmse: 0.0,
        n: 0,
        slope_se: 0.0,
        intercept_se: 0.0,
    };
    geo_stats::predict(x, &model)
}

/// Batch prediction from a fitted OLS model (y = intercept + slope * x).
/// Returns JSON array of predicted values.
#[wasm_bindgen(js_name = olsPredictBatch)]
pub fn ols_predict_batch(x: Vec<f64>, intercept: f64, slope: f64) -> Result<String, JsValue> {
    let model = geo_stats::OlsResult {
        slope,
        intercept,
        r_squared: 0.0,
        rmse: 0.0,
        n: 0,
        slope_se: 0.0,
        intercept_se: 0.0,
    };
    let preds = geo_stats::predict_batch(&x, &model);
    serde_json::to_string(&preds).map_err(err)
}

/// Compute residuals (observed - predicted) using a fitted OLS model.
/// Returns JSON array of residuals.
#[wasm_bindgen(js_name = olsResiduals)]
pub fn ols_residuals(
    x: Vec<f64>,
    y_observed: Vec<f64>,
    intercept: f64,
    slope: f64,
) -> Result<String, JsValue> {
    let model = geo_stats::OlsResult {
        slope,
        intercept,
        r_squared: 0.0,
        rmse: 0.0,
        n: 0,
        slope_se: 0.0,
        intercept_se: 0.0,
    };
    let res = geo_stats::residuals(&x, &y_observed, &model)
        .ok_or_else(|| JsValue::from_str("Residuals computation failed"))?;
    serde_json::to_string(&res).map_err(err)
}

// ── K-Means ─────────────────────────────────────────────────────

/// K-means clustering on multi-dimensional data.
/// `data` is a flat array: [dim0_row0, dim1_row0, dim0_row1, dim1_row1, ...]
/// `n_dims` is the number of features per point.
///
/// Returns JSON:
/// ```json
/// {
///   "centroids": [[c0_d0, c0_d1], [c1_d0, c1_d1], ...],
///   "labels": [0, 1, 0, 2, ...],
///   "iterations": 15,
///   "inertia": 12.3,
///   "converged": true
/// }
/// ```
#[wasm_bindgen(js_name = kmeans)]
pub fn kmeans(
    data: Vec<f64>,
    n_dims: usize,
    k: usize,
    max_iters: usize,
    seed: Option<u64>,
) -> Result<String, JsValue> {
    // Convert flat array to Vec<Vec<f64>>
    let n_points = data.len() / n_dims;
    let mut points: Vec<Vec<f64>> = Vec::with_capacity(n_points);
    for i in 0..n_points {
        let start = i * n_dims;
        points.push(data[start..start + n_dims].to_vec());
    }
    let result = geo_stats::kmeans(&points, k, max_iters, seed)
        .ok_or_else(|| JsValue::from_str("K-means clustering failed"))?;
    serde_json::to_string(&serde_json::json!({
        "centroids": result.centroids,
        "labels": result.labels,
        "iterations": result.iterations,
        "inertia": result.inertia,
        "converged": result.converged,
    }))
    .map_err(err)
}

/// K-means clustering on 2D spatial data (x, y coordinates).
/// Returns same JSON structure as `kmeans`.
#[wasm_bindgen(js_name = kmeans2d)]
pub fn kmeans_2d(
    x: Vec<f64>,
    y: Vec<f64>,
    k: usize,
    max_iters: usize,
    seed: Option<u64>,
) -> Result<String, JsValue> {
    let result = geo_stats::kmeans_2d(&x, &y, k, max_iters, seed)
        .ok_or_else(|| JsValue::from_str("K-means 2D clustering failed"))?;
    serde_json::to_string(&serde_json::json!({
        "centroids": result.centroids,
        "labels": result.labels,
        "iterations": result.iterations,
        "inertia": result.inertia,
        "converged": result.converged,
    }))
    .map_err(err)
}

// ── Moran's I ───────────────────────────────────────────────────

/// Compute Moran's I spatial autocorrelation statistic.
/// `weights` is a flat (n * n) row-major spatial weights matrix.
/// `n` is inferred from values.len().
///
/// Returns JSON:
/// ```json
/// {
///   "i": 0.45,
///   "expected_i": -0.01,
///   "variance_i": 0.002,
///   "z_score": 3.2,
///   "p_value": 0.001,
///   "n": 100,
///   "weight_sum": 400.0
/// }
/// ```
#[wasm_bindgen(js_name = moransI)]
pub fn morans_i(values: Vec<f64>, weights: Vec<f64>) -> Result<String, JsValue> {
    let result = geo_stats::morans_i(&values, &weights)
        .ok_or_else(|| JsValue::from_str("Moran's I computation failed"))?;
    serde_json::to_string(&serde_json::json!({
        "i": result.i,
        "expected_i": result.expected_i,
        "variance_i": result.variance_i,
        "z_score": result.z_score,
        "p_value": result.p_value,
        "n": result.n,
        "weight_sum": result.weight_sum,
    }))
    .map_err(err)
}

/// Generate queen-contiguity spatial weights for a regular grid (nrows × ncols).
/// Returns a flat Vec<f64> of length n*n (row-major), where n = nrows * ncols.
#[wasm_bindgen(js_name = queenWeights)]
pub fn queen_weights(nrows: usize, ncols: usize) -> Result<String, JsValue> {
    let w = geo_stats::queen_weights(nrows, ncols);
    serde_json::to_string(&w).map_err(err)
}

/// Generate rook-contiguity spatial weights for a regular grid (nrows × ncols).
#[wasm_bindgen(js_name = rookWeights)]
pub fn rook_weights(nrows: usize, ncols: usize) -> Result<String, JsValue> {
    let w = geo_stats::rook_weights(nrows, ncols);
    serde_json::to_string(&w).map_err(err)
}

// ── Hotspot Analysis (Getis-Ord Gi*) ────────────────────────────

/// Getis-Ord Gi* hotspot analysis.
/// `values`: attribute values at each location
/// `weights`: flat (n * n) row-major spatial weights matrix
/// `confidence`: significance level (e.g., 0.95 for 95%)
///
/// Returns JSON array of per-location results:
/// ```json
/// [{
///   "index": 0,
///   "z_score": 2.5,
///   "p_value": 0.012,
///   "is_hotspot": true,
///   "is_coldspot": false,
///   "local_sum": 45.0,
///   "neighbor_count": 8
/// }, ...]
/// ```
#[wasm_bindgen(js_name = gistarHotspot)]
pub fn gistar_hotspot(
    values: Vec<f64>,
    weights: Vec<f64>,
    confidence: f64,
) -> Result<String, JsValue> {
    let results = geo_stats::gistar(&values, &weights, confidence)
        .ok_or_else(|| JsValue::from_str("Gi* hotspot analysis failed"))?;
    let json_results: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "index": r.index,
                "z_score": r.z_score,
                "p_value": r.p_value,
                "is_hotspot": r.is_hotspot,
                "is_coldspot": r.is_coldspot,
                "local_sum": r.local_sum,
                "neighbor_count": r.neighbor_count,
            })
        })
        .collect();
    serde_json::to_string(&json_results).map_err(err)
}

/// Generate queen-contiguity weights with self-inclusion for Gi* (regular grid).
#[wasm_bindgen(js_name = queenWeightsSelf)]
pub fn queen_weights_self(nrows: usize, ncols: usize) -> Result<String, JsValue> {
    let w = geo_stats::queen_weights_self(nrows, ncols);
    serde_json::to_string(&w).map_err(err)
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zonal_stats() {
        let data = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];
        let r = compute_zonal_stats_bbox_inner(
            data,
            3,
            3,
            -999.0,
            103.0,
            30.0,
            104.0,
            31.0,
            103.0,
            30.0,
            104.0,
            31.0,
            "test_zone",
        )
        .unwrap();
        assert!(r.contains("test_zone"));
        assert!(r.contains("\"pixel_count\""));
    }

    #[test]
    fn test_jenks() {
        let data: Vec<f64> = (0..100).map(|i| (i as f64).sin() * 50.0 + 50.0).collect();
        let r = jenks_breaks(data, 3).unwrap();
        assert!(r.contains("\"breaks\""));
        assert!(r.contains("\"gvf\""));
    }

    #[test]
    fn test_quantile() {
        let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let r = quantile_breaks(data, 4).unwrap();
        assert!(r.contains('['));
    }

    #[test]
    fn test_equal_interval() {
        let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let r = equal_interval_breaks(data, 5).unwrap();
        assert!(r.contains('['));
    }

    #[test]
    fn test_idw_point() {
        let src_x = vec![0.0, 10.0, 10.0, 0.0];
        let src_y = vec![0.0, 0.0, 10.0, 10.0];
        let src_vals = vec![1.0, 2.0, 3.0, 4.0];
        let val = idw_point_inner(5.0, 5.0, &src_x, &src_y, &src_vals, 2.0, 100.0, 2).unwrap();
        assert!(val > 1.0 && val < 4.0);
    }

    #[test]
    fn test_ols() {
        // y = 3 + 2*x
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![5.0, 7.0, 9.0, 11.0, 13.0];
        let r = ols_regression(x, y).unwrap();
        assert!(r.contains("\"r_squared\""));
        assert!(r.contains("\"slope\""));
    }

    #[test]
    fn test_kmeans_2d() {
        let x: Vec<f64> = vec![0.0, 1.0, 10.0, 11.0, 5.0];
        let y: Vec<f64> = vec![0.0, 1.0, 10.0, 11.0, 5.0];
        let r = kmeans_2d(x, y, 2, 100, Some(42)).unwrap();
        assert!(r.contains("\"labels\""));
        assert!(r.contains("\"centroids\""));
    }

    #[test]
    fn test_moran() {
        // Simple positive autocorrelation
        let values = vec![1.0, 1.0, 1.0, 10.0, 10.0, 10.0, 1.0, 1.0, 1.0];
        let w = geo_stats::queen_weights(3, 3);
        let r = morans_i(values, w).unwrap();
        assert!(r.contains("\"i\""));
        assert!(r.contains("\"z_score\""));
    }

    #[test]
    fn test_gistar() {
        let values = vec![1.0, 2.0, 3.0, 10.0, 20.0, 30.0, 1.0, 2.0, 3.0];
        let w = geo_stats::queen_weights_self(3, 3);
        let r = gistar_hotspot(values, w, 0.95).unwrap();
        assert!(r.contains("\"z_score\""));
        assert!(r.contains("\"is_hotspot\""));
    }
}
