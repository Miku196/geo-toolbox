use crate::{GroundwaterConfig, GroundwaterPlugin};
use geo_core::errors::{GeoError, GeoResult};
use geo_core::plugin::{Plugin, PluginCategory, ProcessPlugin};

impl Plugin for GroundwaterPlugin {
    type Config = GroundwaterConfig;
    fn new(config: GroundwaterConfig) -> Self {
        Self::new(config)
    }
    fn name(&self) -> &str {
        "groundwater"
    }
    fn version(&self) -> &str {
        "0.1"
    }
    fn description(&self) -> &str {
        "Groundwater resources — pumping test, recharge, trend"
    }
    fn category(&self) -> PluginCategory {
        PluginCategory::Process
    }
}

impl ProcessPlugin for GroundwaterPlugin {
    fn process_type(&self) -> &str {
        "groundwater"
    }
    async fn execute(&self, p: serde_json::Value) -> GeoResult<serde_json::Value> {
        let action = p["action"].as_str().unwrap_or("recharge");
        match action {
            "recharge" => {
                let precip = p["precipitation_mm"].as_f64().unwrap_or(0.0);
                let et = p["evapotranspiration_mm"].as_f64().unwrap_or(0.0);
                let rc = p["runoff_coefficient"].as_f64().unwrap_or(0.1);
                let area = p["area_km2"].as_f64();
                Ok(serde_json::to_value(self.recharge(precip, et, rc, area))
                    .map_err(GeoError::Serde)?)
            }
            _ => Err(GeoError::Validation(format!("Unknown action: {}", action))),
        }
    }
}
