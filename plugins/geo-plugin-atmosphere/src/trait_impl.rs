use geo_core::errors::GeoResult;
use geo_core::plugin::{Plugin, PluginCategory};

use crate::config::AtmosphereConfig;

pub struct AtmospherePlugin {
    pub config: AtmosphereConfig,
}

impl AtmospherePlugin {
    pub fn new(config: AtmosphereConfig) -> Self {
        Self { config }
    }

    pub fn load(_path: &std::path::Path) -> GeoResult<Self> {
        Ok(Self::new(AtmosphereConfig::default()))
    }
}

impl Plugin for AtmospherePlugin {
    type Config = AtmosphereConfig;

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
