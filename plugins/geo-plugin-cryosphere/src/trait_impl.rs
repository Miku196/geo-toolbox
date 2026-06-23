//! Plugin trait impl — CryospherePlugin
use crate::config::CryosphereConfig;
use geo_core::errors::GeoResult;
use geo_core::plugin::{Plugin, PluginCategory};

pub struct CryospherePlugin {
    pub config: CryosphereConfig,
}

impl CryospherePlugin {
    pub fn new(config: CryosphereConfig) -> Self { Self { config } }
    pub fn load(_path: &std::path::Path) -> GeoResult<Self> {
        Ok(Self::new(CryosphereConfig::default()))
    }
}

impl Plugin for CryospherePlugin {
    type Config = CryosphereConfig;
    fn new(config: Self::Config) -> Self { Self { config } }
    fn name(&self) -> &str { &self.config.plugin.name }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
    fn description(&self) -> &str { &self.config.plugin.description }
    fn category(&self) -> PluginCategory { PluginCategory::Process }
}
