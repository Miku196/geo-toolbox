//! Coordinate Reference System engine.
//!
//! Delegates to [`geo_core::crs::CrsRegistry`].

use geo_core::crs::CrsRegistry;
use pyo3::prelude::*;

/// Coordinate Reference System engine (non-pyclass, called from lib.rs wrapper).
pub struct CrsEngine {
    registry: CrsRegistry,
}

impl CrsEngine {
    pub fn new() -> Self {
        Self {
            registry: CrsRegistry::new(),
        }
    }

    /// List all available CRS definitions as a list of dicts.
    pub fn list_all(&self) -> PyResult<Vec<CrsDefPy>> {
        Ok(self
            .registry
            .list()
            .map(|d| CrsDefPy {
                epsg: d.epsg,
                name: d.name.to_string(),
                proj4: d.proj4.to_string(),
                category: format!("{:?}", d.category),
            })
            .collect())
    }

    /// Transform a coordinate pair (x, y) between EPSG codes.
    pub fn transform(&self, from_epsg: u16, to_epsg: u16, x: f64, y: f64) -> PyResult<(f64, f64)> {
        self.registry
            .transform_point(from_epsg, to_epsg, x, y)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Batch transform coordinate pairs.
    pub fn transform_batch(
        &self,
        from_epsg: u16,
        to_epsg: u16,
        coords: Vec<f64>,
    ) -> PyResult<Vec<f64>> {
        if !coords.len().is_multiple_of(2) {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "coords length must be even (x,y pairs)",
            ));
        }
        let mut result = Vec::with_capacity(coords.len());
        for chunk in coords.chunks(2) {
            let (rx, ry) = self.transform(from_epsg, to_epsg, chunk[0], chunk[1])?;
            result.push(rx);
            result.push(ry);
        }
        Ok(result)
    }
}

/// Python-exportable CRS definition.
#[pyclass]
#[derive(Clone)]
pub struct CrsDefPy {
    #[pyo3(get)]
    pub epsg: u16,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub proj4: String,
    #[pyo3(get)]
    pub category: String,
}
