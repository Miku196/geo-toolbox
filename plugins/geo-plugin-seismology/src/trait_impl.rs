use crate::config::SeismologyConfig;
use geo_core::errors::GeoResult;
use geo_core::plugin::{Plugin, PluginCategory};

pub struct SeismologyPlugin {
    pub config: SeismologyConfig,
}

impl SeismologyPlugin {
    pub fn new(config: SeismologyConfig) -> Self {
        Self { config }
    }

    pub fn load(_path: &std::path::Path) -> GeoResult<Self> {
        Ok(Self::new(SeismologyConfig::default()))
    }
}

impl Plugin for SeismologyPlugin {
    type Config = SeismologyConfig;

    fn new(config: Self::Config) -> Self {
        Self { config }
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
