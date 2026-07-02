use crate::{HealthConfig, HealthPlugin};
use geo_core::plugin::{Plugin, PluginCategory};
impl Plugin for HealthPlugin {
    type Config = HealthConfig;
    fn new(config: HealthConfig) -> Self {
        let _ = config;
        Self
    }
    fn name(&self) -> &str {
        "public-health"
    }
    fn version(&self) -> &str {
        "0.1"
    }
    fn description(&self) -> &str {
        "Environmental public health"
    }
    fn category(&self) -> PluginCategory {
        PluginCategory::Process
    }
}
