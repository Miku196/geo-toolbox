use crate::{EcosystemConfig, EcosystemPlugin};
use geo_core::plugin::{Plugin, PluginCategory};
impl Plugin for EcosystemPlugin {
    type Config = EcosystemConfig;
    fn new(config: EcosystemConfig) -> Self {
        let _ = config;
        Self
    }
    fn name(&self) -> &str {
        "ecosystem-services"
    }
    fn version(&self) -> &str {
        "0.1"
    }
    fn description(&self) -> &str {
        "Ecosystem services assessment"
    }
    fn category(&self) -> PluginCategory {
        PluginCategory::Process
    }
}
