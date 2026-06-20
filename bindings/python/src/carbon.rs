//! Carbon emission calculation engine.
//!
//! Delegates to [`geo_carbon_math::CarbonEngine`].

use pyo3::prelude::*;

/// Carbon emission calculation engine (non-pyclass, called from lib.rs wrapper).
pub struct CarbonEngine {
    pub inner: geo_carbon_math::CarbonEngine,
}

impl CarbonEngine {
    pub fn new() -> Self {
        Self {
            inner: geo_carbon_math::CarbonEngine::new(),
        }
    }

    pub fn calculate(&self, geojson: &str, factors_csv: &str, year: u16) -> PyResult<String> {
        let report = self
            .inner
            .calculate_from_geojson(geojson, factors_csv, year)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        serde_json::to_string_pretty(&report)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    pub fn calculate_with_json_factors(
        &self,
        geojson: &str,
        factors_json: &str,
        year: u16,
    ) -> PyResult<String> {
        let fc: serde_json::Value = serde_json::from_str(geojson)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        let factors: Vec<geo_carbon_math::EmissionFactor> = serde_json::from_str(factors_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        let features_json = fc["features"].as_array().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("GeoJSON has no 'features' array")
        })?;

        let features: Vec<geo_carbon_math::GeoFeature> = features_json
            .iter()
            .filter_map(|f| geo_carbon_math::GeoFeature::from_feature_json(&f.to_string()).ok())
            .collect();

        let report = self
            .inner
            .calculate(&features, &factors, year)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        serde_json::to_string_pretty(&report)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
}
