use crate::{OceanConfig, OceanPlugin};
use geo_core::plugin::{Plugin, PluginCategory};
impl Plugin for OceanPlugin {
    type Config = OceanConfig;
    fn new(config: OceanConfig) -> Self {
        Self::new(config)
    }
    fn name(&self) -> &str {
        "ocean"
    }
    fn version(&self) -> &str {
        "0.1"
    }
    fn description(&self) -> &str {
        "Oceanography — currents, bleaching, upwelling"
    }
    fn category(&self) -> PluginCategory {
        PluginCategory::Process
    }
}
