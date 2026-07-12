use geo_core::errors::GeoResult;
use geo_core::plugin::{Plugin, PluginCategory};

use crate::config::GeologyConfig;

pub struct GeologyPlugin {
    pub config: GeologyConfig,
}

impl GeologyPlugin {
    pub fn new(config: GeologyConfig) -> Self {
        Self { config }
    }
    pub fn load(_path: &std::path::Path) -> GeoResult<Self> {
        Ok(Self::new(GeologyConfig::default()))
    }
}

impl Plugin for GeologyPlugin {
    type Config = GeologyConfig;
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
