use crate::{SurveyConfig, SurveyPlugin};
use geo_core::errors::GeoResult;
use geo_core::plugin::{Plugin, PluginCategory, ProcessPlugin};

impl Plugin for SurveyPlugin {
    type Config = SurveyConfig;
    fn new(config: SurveyConfig) -> Self {
        Self::new(config)
    }
    fn name(&self) -> &str {
        "survey"
    }
    fn version(&self) -> &str {
        "0.2"
    }
    fn description(&self) -> &str {
        "Surveying: grid earthwork, cross-section, TIN, control network adjustment"
    }
    fn category(&self) -> PluginCategory {
        PluginCategory::Process
    }
}

impl ProcessPlugin for SurveyPlugin {
    fn process_type(&self) -> &str {
        "survey"
    }

    async fn execute(&self, p: serde_json::Value) -> GeoResult<serde_json::Value> {
        let elev: Vec<f64> = p["existing_elevation"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
            .unwrap_or_default();
        let design = p["design_elevation"].as_f64().unwrap_or(0.0);
        let cols = p["grid_cols"].as_u64().unwrap_or(0) as usize;
        let rows = p["grid_rows"].as_u64().unwrap_or(0) as usize;
        let a = self.assess(&elev, design, cols, rows);
        serde_json::to_value(&a).map_err(geo_core::errors::GeoError::Serde)
    }
}
