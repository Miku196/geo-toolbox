use geo_core::errors::GeoResult;
use geo_core::plugin::{Plugin, PluginCategory};

use crate::config::PaleoclimateConfig;

pub struct PaleoclimatePlugin {
    pub config: PaleoclimateConfig,
}

impl PaleoclimatePlugin {
    pub fn new(config: PaleoclimateConfig) -> Self {
        Self { config }
    }
    pub fn load(_path: &std::path::Path) -> GeoResult<Self> {
        Ok(Self::new(PaleoclimateConfig::default()))
    }
}

impl Plugin for PaleoclimatePlugin {
    type Config = PaleoclimateConfig;
    fn new(config: Self::Config) -> Self {
        Self::new(config)
    }
    fn name(&self) -> &str {
        &self.config.plugin.name
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        &self.config.plugin.description
    }
    fn category(&self) -> PluginCategory {
        PluginCategory::Process
    }
}
