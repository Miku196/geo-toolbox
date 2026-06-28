use geo_core::errors::GeoResult;
use wasm_bindgen::prelude::*;

/// Carbon emission calculation engine.
///
/// Delegates to [`geo_carbon_math::CarbonEngine`] for all computation.
#[wasm_bindgen]
pub struct CarbonEngine {
    inner: geo_carbon_math::CarbonEngine,
}

// ── Non-WASM methods (testable on native) ────────────────────────

impl CarbonEngine {
    fn calculate_inner(
        &self,
        geojson_str: &str,
        factors_csv: &str,
        year: u16,
    ) -> GeoResult<String> {
        let report = self
            .inner
            .calculate_from_geojson(geojson_str, factors_csv, year)
            .map_err(|e| geo_core::errors::GeoError::Other(e.to_string()))?;
        serde_json::to_string_pretty(&report).map_err(geo_core::errors::GeoError::Serde)
    }

    fn calculate_with_json_factors_inner(
        &self,
        geojson_str: &str,
        factors_json: &str,
        year: u16,
    ) -> GeoResult<String> {
        // Parse JSON factors and convert to CSV format for calculate_from_geojson
        let factors: Vec<serde_json::Value> = serde_json::from_str(factors_json).map_err(|e| {
            geo_core::errors::GeoError::Validation(format!("Invalid factors JSON: {e}"))
        })?;
        let mut csv = String::from("category,factor_value,source\n");
        for f in &factors {
            let cat = f["category"].as_str().unwrap_or("unknown");
            let val = f["factor_value"].as_f64().unwrap_or(0.0);
            let src = f["source"].as_str().unwrap_or("IPCC Tier 1");
            csv.push_str(&format!("{cat},{val},{src}\n"));
        }
        let report = self
            .inner
            .calculate_from_geojson(geojson_str, &csv, year)
            .map_err(geo_core::errors::GeoError::Other)?;
        serde_json::to_string_pretty(&report).map_err(geo_core::errors::GeoError::Serde)
    }
}

// ── WASM bindings ────────────────────────────────────────────────

#[wasm_bindgen]
impl CarbonEngine {
    /// Create a new carbon engine.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: geo_carbon_math::CarbonEngine::new(),
        }
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
    /// JSON string representing [`geo_carbon_math::CarbonReport`].
    #[wasm_bindgen(js_name = calculate)]
    pub fn calculate(
        &self,
        geojson_str: &str,
        factors_csv: &str,
        year: u16,
    ) -> Result<String, JsValue> {
        self.calculate_inner(geojson_str, factors_csv, year)
            .map_err(|e| JsValue::from_str(&e.to_string()))
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
        self.calculate_with_json_factors_inner(geojson_str, factors_json, year)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

impl Default for CarbonEngine {
    fn default() -> Self {
        Self::new()
    }
}
