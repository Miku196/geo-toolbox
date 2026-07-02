use crate::{BiodiversityConfig, BiodiversityPlugin};
use geo_core::errors::{GeoError, GeoResult};
use geo_core::plugin::{Plugin, PluginCategory, ProcessPlugin};

impl Plugin for BiodiversityPlugin {
    type Config = BiodiversityConfig;
    fn new(config: BiodiversityConfig) -> Self {
        Self::new(config)
    }
    fn name(&self) -> &str {
        "biodiversity"
    }
    fn version(&self) -> &str {
        "0.1"
    }
    fn description(&self) -> &str {
        "Biodiversity assessment — SDM, habitat, GAP"
    }
    fn category(&self) -> PluginCategory {
        PluginCategory::Process
    }
}

impl ProcessPlugin for BiodiversityPlugin {
    fn process_type(&self) -> &str {
        "biodiversity"
    }
    async fn execute(&self, p: serde_json::Value) -> GeoResult<serde_json::Value> {
        let action = p["action"].as_str().unwrap_or("diversity");
        match action {
            "diversity" => {
                let abundances: Vec<f64> = p["abundances"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
                    .unwrap_or_default();
                Ok(serde_json::to_value(self.diversity(&abundances)).map_err(GeoError::Serde)?)
            }
            _ => Err(GeoError::Validation(format!("Unknown action: {}", action))),
        }
    }
}
