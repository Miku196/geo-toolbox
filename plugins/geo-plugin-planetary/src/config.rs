use geo_core::plugin::{PluginConfig, PluginMeta};
use serde::{Deserialize, Serialize};

/// 行星/天文插件配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanetaryConfig {
    pub plugin: PluginMeta,
    #[serde(default = "default_solar_constant")]
    pub solar_constant_wm2: f64,
    #[serde(default = "default_lunar_frame")]
    pub lunar_frame: String,
    #[serde(default = "default_mars_frame")]
    pub mars_frame: String,
}

fn default_solar_constant() -> f64 {
    1361.0
}
fn default_lunar_frame() -> String {
    "MeanEarthPole".into()
}
fn default_mars_frame() -> String {
    "Mars2000".into()
}

impl Default for PlanetaryConfig {
    fn default() -> Self {
        Self {
            plugin: PluginMeta {
                name: "planetary".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                description: "Planetary astronomy: coordinate transforms, solar position, extraterrestrial radiation".into(),
                category: geo_core::plugin::PluginCategory::Process,
                healthy: true,
                extra: serde_json::Value::Null,
            },
            solar_constant_wm2: default_solar_constant(),
            lunar_frame: default_lunar_frame(),
            mars_frame: default_mars_frame(),
        }
    }
}

impl PluginConfig for PlanetaryConfig {}
