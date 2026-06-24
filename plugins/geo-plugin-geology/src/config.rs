use geo_core::plugin::{PluginConfig, PluginMeta};
use serde::{Deserialize, Serialize};

/// 地质/构造插件配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeologyConfig {
    pub plugin: PluginMeta,
    #[serde(default = "default_layer_thickness")]
    pub default_layer_thickness_m: f64,
    #[serde(default = "default_interpolation")]
    pub interpolation_method: String,
    #[serde(default = "default_dip")]
    pub default_dip_degrees: f64,
    #[serde(default = "default_strike")]
    pub default_strike_degrees: f64,
}

fn default_layer_thickness() -> f64 {
    50.0
}
fn default_interpolation() -> String {
    "linear".into()
}
fn default_dip() -> f64 {
    45.0
}
fn default_strike() -> f64 {
    0.0
}

impl Default for GeologyConfig {
    fn default() -> Self {
        Self {
            plugin: PluginMeta {
                name: "geology".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                description: "Geology: stratigraphic 3D modeling, fault/fold geometry, lithology classification".into(),
                category: geo_core::plugin::PluginCategory::Process,
                healthy: true,
                extra: serde_json::Value::Null,
            },
            default_layer_thickness_m: default_layer_thickness(),
            interpolation_method: default_interpolation(),
            default_dip_degrees: default_dip(),
            default_strike_degrees: default_strike(),
        }
    }
}

impl PluginConfig for GeologyConfig {}
