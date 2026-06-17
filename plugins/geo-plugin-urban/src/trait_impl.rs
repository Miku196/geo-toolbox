use crate::urban::SolarResult;
use crate::{UrbanConfig, UrbanPlugin};
use geo_core::errors::GeoResult;
use geo_core::plugin::{Plugin, PluginCategory, ProcessPlugin};

impl Plugin for UrbanPlugin {
    fn name(&self) -> &str {
        "urban"
    }
    fn version(&self) -> &str {
        "0.2"
    }
    fn description(&self) -> &str {
        "Urban planning: FAR, land use classification, solar analysis, UHI, ventilation corridor"
    }
    fn category(&self) -> PluginCategory {
        PluginCategory::Process
    }
    type Config = UrbanConfig;
    fn new(config: UrbanConfig) -> Self {
        Self::new(config)
    }
}

impl ProcessPlugin for UrbanPlugin {
    fn process_type(&self) -> &str {
        "urban"
    }

    async fn execute(&self, p: serde_json::Value) -> GeoResult<serde_json::Value> {
        let tfa = p["total_floor_area_m2"].as_f64().unwrap_or(0.0);
        let bf = p["building_footprint_m2"].as_f64().unwrap_or(0.0);
        let sa = p["site_area_m2"].as_f64().unwrap_or(0.0);
        let ga = p["green_area_m2"].as_f64().unwrap_or(0.0);
        let pop = p["population"].as_u64().unwrap_or(0);
        let imp = p["impervious_ratio"].as_f64().unwrap_or(0.0);

        let ndvi_raw = p["ndvi_values"].as_array();
        let imp_raw = p["impervious_values"].as_array();
        let ndvi: Vec<Option<f64>> =
            ndvi_raw.map_or(vec![], |a| a.iter().map(|v| v.as_f64()).collect());
        let impervious: Vec<Option<f64>> =
            imp_raw.map_or(vec![], |a| a.iter().map(|v| v.as_f64()).collect());

        let assessment = self.assess(tfa, bf, sa, ga, pop, imp, &ndvi, &impervious);
        Ok(serde_json::to_value(&assessment).map_err(|e| geo_core::errors::GeoError::Serde(e))?)
    }
}

/// Helper: parse SolarResult from JSON value (used by tools)
pub fn solar_to_json(sr: &SolarResult) -> serde_json::Value {
    serde_json::json!({
        "winter_shadow_m": sr.winter_shadow_m,
        "summer_shadow_m": sr.summer_shadow_m,
        "shadow_azimuth_deg": sr.shadow_azimuth_deg,
        "compliant": sr.compliant,
    })
}
