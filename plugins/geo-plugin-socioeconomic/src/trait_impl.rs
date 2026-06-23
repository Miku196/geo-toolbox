//! Plugin trait impl — SocioeconomicPlugin
use crate::config::SocioeconomicConfig;
use geo_core::plugin::{Plugin, PluginCategory};
use geo_core::errors::GeoResult;

pub struct SocioeconomicPlugin {
    pub config: SocioeconomicConfig,
}

impl SocioeconomicPlugin {
    pub fn new(config: SocioeconomicConfig) -> Self {
        Self { config }
    }

    pub fn load(_path: &std::path::Path) -> GeoResult<Self> {
        Ok(Self::new(SocioeconomicConfig::default()))
    }
}

impl Plugin for SocioeconomicPlugin {
    type Config = SocioeconomicConfig;

    fn new(config: Self::Config) -> Self {
        Self { config }
    }

    fn name(&self) -> &str { &self.config.plugin.name }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
    fn description(&self) -> &str { &self.config.plugin.description }
    fn category(&self) -> PluginCategory { PluginCategory::Process }
}
