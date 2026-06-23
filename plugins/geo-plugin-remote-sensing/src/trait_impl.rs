//! Plugin trait impl — RemoteSensingPlugin
use crate::config::RemoteSensingConfig;
use geo_core::errors::GeoResult;
use geo_core::plugin::{Plugin, PluginCategory};

pub struct RemoteSensingPlugin {
    pub config: RemoteSensingConfig,
}

impl RemoteSensingPlugin {
    pub fn new(config: RemoteSensingConfig) -> Self {
        Self { config }
    }

    pub fn load(_path: &std::path::Path) -> GeoResult<Self> {
        Ok(Self::new(RemoteSensingConfig::default()))
    }
}

impl Plugin for RemoteSensingPlugin {
    type Config = RemoteSensingConfig;

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
