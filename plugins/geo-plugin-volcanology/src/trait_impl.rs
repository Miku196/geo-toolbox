use geo_core::errors::GeoResult;
use geo_core::plugin::{Plugin, PluginCategory};

use crate::config::VolcanologyConfig;

pub struct VolcanologyPlugin {
    pub config: VolcanologyConfig,
}

impl VolcanologyPlugin {
    pub fn new(config: VolcanologyConfig) -> Self {
        Self { config }
    }
    pub fn load(_path: &std::path::Path) -> GeoResult<Self> {
        Ok(Self::new(VolcanologyConfig::default()))
    }
}

impl Plugin for VolcanologyPlugin {
    type Config = VolcanologyConfig;
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
