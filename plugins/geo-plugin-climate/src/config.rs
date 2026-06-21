use geo_core::plugin::{PluginConfig, PluginMeta};
use serde::{Deserialize, Serialize};

/// Climate plugin configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClimateConfig {
    pub plugin: PluginMeta,
    pub gcm_model: String,
    pub base_period_start: u16,
    pub base_period_end: u16,
    pub projection_period_start: u16,
    pub projection_period_end: u16,
}

impl Default for ClimateConfig {
    fn default() -> Self {
        Self {
            plugin: PluginMeta {
                name: "climate".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                description: "Climate & meteorology plugin".into(),
                category: geo_core::plugin::PluginCategory::Process,
                healthy: true,
                extra: serde_json::Value::Null,
            },
            gcm_model: "MRI-AGCM3.2".into(),
            base_period_start: 1981,
            base_period_end: 2010,
            projection_period_start: 2041,
            projection_period_end: 2070,
        }
    }
}

impl PluginConfig for ClimateConfig {}
