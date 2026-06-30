//! Python bindings for geo-stats core tools.

use pyo3::prelude::*;

/// Zonal statistics for raster data within bounding boxes.
#[pyfunction]
pub fn zonal_stats(
    zones: Vec<(String, f64, f64, f64, f64)>, // (name, min_x, min_y, max_x, max_y)
    raster_data: Vec<f64>,
    raster_cols: usize,
    raster_min_x: f64,
    raster_min_y: f64,
    raster_max_x: f64,
    raster_max_y: f64,
    nodata: Option<f64>,
) -> PyResult<Vec<(String, u64, f64, f64, f64, f64)>> {
    let nd = nodata.unwrap_or(-999.0);
    let rb = geo_core::types::BBox {
        min_x: raster_min_x,
        min_y: raster_min_y,
        max_x: raster_max_x,
        max_y: raster_max_y,
    };
    let data = &raster_data;
    let mut results = Vec::new();
    for (name, min_x, min_y, max_x, max_y) in zones {
        let zb = geo_core::types::BBox {
            min_x,
            min_y,
            max_x,
            max_y,
        };
        let zr = geo_stats::zonal_stats(
            data,
            data.len() / raster_cols.max(1),
            raster_cols,
            nd,
            rb,
            &zb,
            &name,
        )
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        results.push((name, zr.pixel_count as u64, zr.mean, zr.min, zr.max, zr.sum));
    }
    Ok(results)
}

/// Moran's I spatial autocorrelation.
#[pyfunction]
#[pyo3(signature = (values, nrows, ncols, rook=true))]
pub fn morans_i(
    values: Vec<f64>,
    nrows: usize,
    ncols: usize,
    rook: bool,
) -> PyResult<(f64, f64, f64, f64)> {
    let weights = if rook {
        geo_stats::moran::rook_weights(nrows, ncols)
    } else {
        geo_stats::moran::queen_weights(nrows, ncols)
    };
    match geo_stats::moran::morans_i(&values, &weights) {
        Some(r) => Ok((r.i, r.expected_i, r.z_score, r.p_value)),
        None => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Moran's I failed",
        )),
    }
}

/// Getis-Ord Gi* hotspot analysis.
#[pyfunction]
#[pyo3(signature = (values, nrows, ncols, confidence=0.05))]
pub fn gistar(
    values: Vec<f64>,
    nrows: usize,
    ncols: usize,
    confidence: f64,
) -> PyResult<Vec<(usize, f64, f64, bool, bool)>> {
    let weights = geo_stats::hotspot::queen_weights_self(nrows, ncols);
    let hotspots = geo_stats::hotspot::gistar(&values, &weights, confidence)
        .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Gi* failed"))?;
    Ok(hotspots
        .iter()
        .map(|h| (h.index, h.z_score, h.p_value, h.is_hotspot, h.is_coldspot))
        .collect())
}

/// IDW spatial interpolation (regular grid).
#[pyfunction]
#[pyo3(signature = (x_src, y_src, values_src, ncols, nrows, min_x, min_y, max_x, max_y, power=2.0, max_radius=0.0, min_neighbors=1))]
pub fn idw_grid(
    x_src: Vec<f64>,
    y_src: Vec<f64>,
    values_src: Vec<f64>,
    ncols: usize,
    nrows: usize,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    power: f64,
    max_radius: f64,
    min_neighbors: usize,
) -> PyResult<(Vec<f64>, usize, usize)> {
    let bbox = geo_core::types::BBox {
        min_x,
        min_y,
        max_x,
        max_y,
    };
    let (grid, _meta) = geo_stats::idw::idw_grid(
        &bbox,
        ncols,
        nrows,
        &x_src,
        &y_src,
        &values_src,
        power,
        max_radius,
        min_neighbors,
    );
    Ok((grid, ncols, nrows))
}

/// Jenks Natural Breaks classification.
#[pyfunction]
pub fn jenks_classify(values: Vec<f64>, k: usize) -> PyResult<(Vec<f64>, f64, Vec<usize>)> {
    match geo_stats::classify::jenks(&values, k) {
        Some(r) => Ok((r.breaks, r.gvf, r.classes)),
        None => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Jenks classification failed",
        )),
    }
}
