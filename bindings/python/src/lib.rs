//! Python bindings for geo-toolbox.
//!
//! Exposes core GIS operations to Python via PyO3 + maturin.

mod carbon;
mod crs;
mod geohash;
mod ingest;
mod io_util;
mod spatial;
mod tile;

use pyo3::prelude::*;
#[allow(unused_imports)]
use pyo3::types::PyList;

// ── PyClass re-exports ──────────────────────────────────────────

/// Coordinate Reference System engine.
#[pyclass]
struct CrsEngine(crs::CrsEngine);
#[pymethods]
impl CrsEngine {
    #[new]
    fn new() -> Self {
        Self(crs::CrsEngine::new())
    }
    fn list_all(&self) -> PyResult<Vec<crs::CrsDefPy>> {
        self.0.list_all()
    }
    fn transform(&self, from_epsg: u16, to_epsg: u16, x: f64, y: f64) -> PyResult<(f64, f64)> {
        self.0.transform(from_epsg, to_epsg, x, y)
    }
    fn transform_batch(
        &self,
        from_epsg: u16,
        to_epsg: u16,
        coords: Vec<f64>,
    ) -> PyResult<Vec<f64>> {
        self.0.transform_batch(from_epsg, to_epsg, coords)
    }
}

/// Carbon emission calculation engine.
#[pyclass]
struct CarbonEngine(carbon::CarbonEngine);
#[pymethods]
impl CarbonEngine {
    #[new]
    fn new() -> Self {
        Self(carbon::CarbonEngine::new())
    }
    fn calculate(&self, geojson: &str, factors_csv: &str, year: u16) -> PyResult<String> {
        self.0.calculate(geojson, factors_csv, year)
    }
    fn calculate_with_json_factors(
        &self,
        geojson: &str,
        factors_json: &str,
        year: u16,
    ) -> PyResult<String> {
        self.0
            .calculate_with_json_factors(geojson, factors_json, year)
    }
}

/// Mapbox Vector Tile encoder.
#[pyclass]
struct MvtEncoder(tile::MvtEncoder);
#[pymethods]
impl MvtEncoder {
    #[new]
    fn new(extent: u32) -> Self {
        Self(tile::MvtEncoder::new(extent))
    }
    fn add_layer(
        &mut self,
        name: &str,
        features: &Bound<'_, PyAny>,
        tile_x: u32,
        tile_y: u32,
        zoom: u8,
    ) -> PyResult<()> {
        self.0.add_layer(name, features, tile_x, tile_y, zoom)
    }
    fn encode(&self) -> PyResult<Vec<u8>> {
        self.0.encode()
    }
}

// ── PyFunction wrappers ─────────────────────────────────────────

#[pyfunction]
fn geohash_encode(lat: f64, lon: f64, precision: usize) -> PyResult<String> {
    geohash::geohash_encode_impl(lat, lon, precision)
}
#[pyfunction]
fn geohash_decode(hash: &str) -> PyResult<(f64, f64, f64, f64)> {
    geohash::geohash_decode_impl(hash)
}
#[pyfunction]
fn geohash_neighbors(hash: &str) -> PyResult<Vec<String>> {
    geohash::geohash_neighbors_impl(hash)
}

#[pyfunction]
fn parse_nmea(sentence: &str) -> PyResult<std::collections::HashMap<String, String>> {
    ingest::parse_nmea_impl(sentence)
}
#[pyfunction]
fn parse_nmea_batch(
    sentences: Vec<String>,
) -> PyResult<Vec<std::collections::HashMap<String, String>>> {
    ingest::parse_nmea_batch_impl(sentences)
}
#[pyfunction]
fn validate_coord(lat: f64, lon: f64) -> PyResult<(bool, String)> {
    Ok(ingest::validate_coord_impl(lat, lon))
}
#[pyfunction]
fn validate_gps_fix(quality: u8) -> PyResult<(bool, String)> {
    Ok(ingest::validate_gps_fix_impl(quality))
}
#[pyfunction]
fn validate_sensor_reading(
    value: f64,
    sensor_type: &str,
    min_val: f64,
    max_val: f64,
) -> PyResult<(bool, String)> {
    Ok(ingest::validate_sensor_reading_impl(
        value,
        sensor_type,
        min_val,
        max_val,
    ))
}

#[pyfunction]
fn parse_csv_to_json(csv_text: &str) -> PyResult<Vec<std::collections::HashMap<String, String>>> {
    io_util::parse_csv_to_json_impl(csv_text)
}
#[pyfunction]
fn generate_geojson(features: &Bound<'_, PyList>) -> PyResult<String> {
    io_util::generate_geojson_impl(features)
}
#[pyfunction]
#[pyo3(signature = (columns, rows, sheet_name=None))]
fn generate_excel(
    columns: Vec<String>,
    rows: Vec<Vec<PyObject>>,
    sheet_name: Option<String>,
) -> PyResult<Vec<u8>> {
    io_util::generate_excel_impl(columns, rows, sheet_name)
}
#[pyfunction]
fn generate_carbon_report_md(
    report: &Bound<'_, PyAny>,
    aoi_name: &str,
    auditor: &str,
) -> PyResult<String> {
    io_util::generate_carbon_report_md_impl(report, aoi_name, auditor)
}

#[pyfunction]
fn compute_area_sqm(geojson_geom: &str) -> PyResult<f64> {
    spatial::compute_area_sqm_impl(geojson_geom)
}
#[pyfunction]
fn compute_bbox(geojson_geom: &str) -> PyResult<(f64, f64, f64, f64)> {
    spatial::compute_bbox_impl(geojson_geom)
}
#[pyfunction]
fn compute_centroid(geojson_geom: &str) -> PyResult<(f64, f64)> {
    spatial::compute_centroid_impl(geojson_geom)
}
#[pyfunction]
fn simplify_geometry(geojson_geom: &str, epsilon: f64) -> PyResult<String> {
    spatial::simplify_geometry_impl(geojson_geom, epsilon)
}
#[pyfunction]
fn convex_hull(geojson_geom: &str) -> PyResult<String> {
    spatial::convex_hull_impl(geojson_geom)
}

#[pyfunction]
fn latlon_to_tile(lat: f64, lon: f64, zoom: u8) -> PyResult<(u32, u32, u8)> {
    Ok(tile::latlon_to_tile_impl(lat, lon, zoom))
}
#[pyfunction]
fn tile_to_latlon(x: u32, y: u32, zoom: u8) -> PyResult<(f64, f64, f64, f64)> {
    Ok(tile::tile_to_latlon_impl(x, y, zoom))
}
#[pyfunction]
fn tile_url(source: &str, x: u32, y: u32, zoom: u8) -> PyResult<String> {
    tile::tile_url_impl(source, x, y, zoom)
}

// ── Module registration ─────────────────────────────────────────

/// Top-level Python module registration.
#[pymodule]
fn _geo_toolbox(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<CrsEngine>()?;
    m.add_class::<CarbonEngine>()?;
    m.add_class::<MvtEncoder>()?;

    m.add_function(wrap_pyfunction!(latlon_to_tile, m)?)?;
    m.add_function(wrap_pyfunction!(tile_to_latlon, m)?)?;
    m.add_function(wrap_pyfunction!(tile_url, m)?)?;
    m.add_function(wrap_pyfunction!(compute_area_sqm, m)?)?;
    m.add_function(wrap_pyfunction!(compute_bbox, m)?)?;
    m.add_function(wrap_pyfunction!(compute_centroid, m)?)?;
    m.add_function(wrap_pyfunction!(simplify_geometry, m)?)?;
    m.add_function(wrap_pyfunction!(convex_hull, m)?)?;
    m.add_function(wrap_pyfunction!(parse_csv_to_json, m)?)?;
    m.add_function(wrap_pyfunction!(generate_geojson, m)?)?;
    m.add_function(wrap_pyfunction!(generate_excel, m)?)?;
    m.add_function(wrap_pyfunction!(generate_carbon_report_md, m)?)?;
    m.add_function(wrap_pyfunction!(parse_nmea, m)?)?;
    m.add_function(wrap_pyfunction!(parse_nmea_batch, m)?)?;
    m.add_function(wrap_pyfunction!(validate_coord, m)?)?;
    m.add_function(wrap_pyfunction!(validate_gps_fix, m)?)?;
    m.add_function(wrap_pyfunction!(validate_sensor_reading, m)?)?;
    m.add_function(wrap_pyfunction!(geohash_encode, m)?)?;
    m.add_function(wrap_pyfunction!(geohash_decode, m)?)?;
    m.add_function(wrap_pyfunction!(geohash_neighbors, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    Ok(())
}
