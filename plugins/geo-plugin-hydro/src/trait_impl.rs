use crate::{HydroConfig, HydroPlugin};
use geo_core::errors::GeoResult;
use geo_core::plugin::{Plugin, PluginCategory, ProcessPlugin};

impl Plugin for HydroPlugin {
    fn name(&self) -> &str {
        "hydro"
    }
    fn version(&self) -> &str {
        "0.2"
    }
    fn description(&self) -> &str {
        "Hydrology: D8 flow accumulation, runoff, inundation analysis"
    }
    fn category(&self) -> PluginCategory {
        PluginCategory::Process
    }
    type Config = HydroConfig;
    fn new(config: HydroConfig) -> Self {
        Self::new(config)
    }
}

impl ProcessPlugin for HydroPlugin {
    fn process_type(&self) -> &str {
        "hydro"
    }

    async fn execute(&self, p: serde_json::Value) -> GeoResult<serde_json::Value> {
        let dem: Vec<f64> = p["dem"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
            .unwrap_or_default();
        let rows = p["rows"].as_u64().unwrap_or(0) as usize;
        let cols = p["cols"].as_u64().unwrap_or(0) as usize;
        let cell = p["cell_size_m"].as_f64().unwrap_or(10.0);
        let imp = p["impervious_ratio"].as_f64().unwrap_or(0.0);
        let rain = p["rainfall_mmh"].as_f64().unwrap_or(50.0);
        let a = self.assess(&dem, rows, cols, cell, imp, rain);
        Ok(serde_json::to_value(&a).map_err(|e| geo_core::errors::GeoError::Serde(e))?)
    }
}
