use geo_core::errors::GeoResult;
use geo_core::plugin::{Plugin, PluginCategory};

use crate::config::PlanetaryConfig;

pub struct PlanetaryPlugin {
    pub config: PlanetaryConfig,
}

impl PlanetaryPlugin {
    pub fn new(config: PlanetaryConfig) -> Self {
        Self { config }
    }
    pub fn load(_path: &std::path::Path) -> GeoResult<Self> {
        Ok(Self::new(PlanetaryConfig::default()))
    }
}

impl Plugin for PlanetaryPlugin {
    type Config = PlanetaryConfig;
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
