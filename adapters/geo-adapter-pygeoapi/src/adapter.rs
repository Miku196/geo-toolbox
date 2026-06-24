//! PyGeoAdapter — main Python↔Rust geometry bridge.
//!
//! Uses WKB (Well-Known Binary) as zero-copy interchange format between
//! Rust geo-types and Python shapely geometries. For raster data, uses
//! numpy ndarray flat bytes with shape/dtype metadata.

#[cfg(feature = "python")]
// use tracing::info;
use crate::geometry::{geometry_to_shapely, shapely_to_geometry};
use crate::raster::{geo_raster_to_numpy, numpy_to_geo_raster};
use geo_core::errors::GeoResult;
use geo_core::plugin::{Plugin, PluginCategory};
#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
use pyo3::types::PyModule;

/// PyGeoAdapter bridges Rust geo-types with Python geospatial libraries.
///
/// # Zero-copy guarantees
///
/// - **WKB bytes**: geometry ↔ shapely conversion uses WKB as shared byte buffer;
///   no per-coordinate copy.
/// - **numpy arrays**: raster ↔ numpy conversion maps flat `&[f64/f32]` slices;
///   shape/strides metadata is separate.
pub struct PyGeoAdapter {
    name: String,
    version: String,
    description: String,
}

impl PyGeoAdapter {
    pub fn new() -> Self {
        Self {
            name: "pygeoapi".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            description: "PyO3 FFI adapter — zero-copy geometry & raster interchange".into(),
        }
    }

    /// Convert a Rust GeoFeature (as WKB bytes) into a Python shapely geometry object.
    ///
    /// Returns the WKB bytes that can be loaded via `shapely.from_wkb()` on the Python side.
    pub fn feature_to_shapely_bytes<'a>(&self, wkb: &'a [u8]) -> &'a [u8] {
        geometry_to_shapely(wkb)
    }

    /// Convert a Python shapely geometry (as WKB bytes) into Rust geo-types.
    ///
    /// Accepts WKB bytes produced by `shapely.to_wkb()`.
    pub fn shapely_bytes_to_feature(&self, wkb: &[u8]) -> GeoResult<Vec<u8>> {
        shapely_to_geometry(wkb)
    }

    /// Convert a numpy 2D/3D array (flat bytes) into a geo-raster Band.
    pub fn numpy_to_raster(
        &self,
        flat_data: &[u8],
        rows: usize,
        cols: usize,
        bands: usize,
        dtype: &str,
    ) -> GeoResult<Vec<f64>> {
        numpy_to_geo_raster(flat_data, rows, cols, bands, dtype)
    }

    /// Convert a geo-raster Band into a numpy-compatible flat f64 buffer.
    pub fn raster_to_numpy(&self, band_data: &[f64], rows: usize, cols: usize) -> Vec<f64> {
        geo_raster_to_numpy(band_data, rows, cols)
    }
}

impl Default for PyGeoAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for PyGeoAdapter {
    type Config = geo_core::plugin::EmptyConfig;

    fn new(_: Self::Config) -> Self {
        Self::new()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::Adapter
    }
}

#[cfg(feature = "python")]
#[pymodule]
fn _pygeoapi(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    /// Convert WKB bytes (shapely geometry) to Rust feature bytes.
    #[pyfn(m)]
    fn shapely_to_feature(wkb: &[u8]) -> PyResult<Vec<u8>> {
        shapely_to_geometry(wkb).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("shapely_to_feature: {e}"))
        })
    }

    /// Convert Rust feature WKB bytes to shapely-compatible WKB.
    #[pyfn(m)]
    fn feature_to_shapely(wkb: &[u8]) -> PyResult<Vec<u8>> {
        Ok(geometry_to_shapely(wkb).to_vec())
    }

    /// Convert numpy flat array to raster band (returns f64 Vec).
    #[pyfn(m)]
    fn numpy_to_raster_band(
        flat_data: Vec<u8>,
        rows: usize,
        cols: usize,
        bands: usize,
        dtype: &str,
    ) -> PyResult<Vec<f64>> {
        numpy_to_geo_raster(&flat_data, rows, cols, bands, dtype).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("numpy_to_raster_band: {e}"))
        })
    }

    /// Convert raster band (f64 slice) to numpy-compatible flat array.
    #[pyfn(m)]
    fn raster_band_to_numpy(band_data: Vec<f64>, rows: usize, cols: usize) -> PyResult<Vec<f64>> {
        Ok(geo_raster_to_numpy(&band_data, rows, cols))
    }

    info!("geo-adapter-pygeoapi Python module initialized");
    Ok(())
}
