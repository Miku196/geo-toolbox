//! Pure-Rust carbon emission calculation engine (IPCC Tier 1).
//!
//! Thin WASM wrapper around [`geo_carbon_core`], providing
//! browser-friendly JSON-in/JSON-out APIs.
//!
//! All computation happens in WASM memory — no data leaves the browser.

use wasm_bindgen::prelude::*;

/// Carbon emission calculation engine.
///
/// Delegates to [`geo_carbon_core::CarbonEngine`] for all computation.
#[wasm_bindgen]
pub struct CarbonEngine {
    inner: geo_carbon_core::CarbonEngine,
}

#[wasm_bindgen]
impl CarbonEngine {
    /// Create a new carbon engine.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { inner: geo_carbon_core::CarbonEngine::new() }
    }

    /// Calculate carbon emissions from GeoJSON features and emission factors.
    ///
    /// ## Parameters
    ///
    /// - `geojson_str`: GeoJSON FeatureCollection string with Polygon/MultiPolygon features.
    ///   Each feature must have `properties.class` (string) indicating landcover type.
    /// - `factors_csv`: CSV string with columns: `category,factor_value[,source]`.
    /// - `year`: Target year for the calculation.
    ///
    /// ## Returns
    ///
    /// JSON string representing [`geo_carbon_core::CarbonReport`].
    #[wasm_bindgen(js_name = calculate)]
    pub fn calculate(
        &self,
        geojson_str: &str,
        factors_csv: &str,
        year: u16,
    ) -> Result<String, JsValue> {
        let report = self.inner.calculate_from_geojson(geojson_str, factors_csv, year)
            .map_err(|e| JsValue::from_str(&e))?;

        serde_json::to_string_pretty(&report)
            .map_err(|e| JsValue::from_str(&format!("Serialization: {e}")))
    }

    /// Calculate with JSON factors (alternative to CSV).
    ///
    /// - `factors_json`: JSON array of `{category, factor_value, source?}` objects.
    #[wasm_bindgen(js_name = calculateWithJsonFactors)]
    pub fn calculate_with_json_factors(
        &self,
        geojson_str: &str,
        factors_json: &str,
        year: u16,
    ) -> Result<String, JsValue> {
        let fc: serde_json::Value = serde_json::from_str(geojson_str)
            .map_err(|e| JsValue::from_str(&format!("Invalid GeoJSON: {e}")))?;

        let factors: Vec<geo_carbon_core::EmissionFactor> = serde_json::from_str(factors_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid factors JSON: {e}")))?;

        let features_json = fc["features"].as_array()
            .ok_or_else(|| JsValue::from_str("GeoJSON has no 'features' array"))?;

        let features: Vec<geo_carbon_core::GeoFeature> = features_json
            .iter()
            .filter_map(|f| {
                geo_carbon_core::GeoFeature::from_feature_json(&f.to_string()).ok()
            })
            .collect();

        let report = self.inner.calculate(&features, &factors, year)
            .map_err(|e| JsValue::from_str(&e))?;

        serde_json::to_string_pretty(&report)
            .map_err(|e| JsValue::from_str(&format!("Serialization: {e}")))
    }
}

impl Default for CarbonEngine {
    fn default() -> Self { Self::new() }
}
