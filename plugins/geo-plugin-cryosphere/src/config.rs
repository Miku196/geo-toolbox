use geo_core::plugin::{PluginConfig, PluginMeta};
use serde::{Deserialize, Serialize};

/// 冰雪/冰川插件配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryosphereConfig {
    pub plugin: PluginMeta,
    /// 融雪度日因子 (mm/°C/day)
    #[serde(default = "default_ddf")]
    pub degree_day_factor: f64,
    /// 雨雪分界温度 (°C)
    #[serde(default = "default_rain_snow")]
    pub rain_snow_threshold: f64,
    /// 冰密度 (kg/m³)
    #[serde(default = "default_ice_density")]
    pub ice_density: f64,
}

fn default_ddf() -> f64 { 3.5 }
fn default_rain_snow() -> f64 { 1.0 }
fn default_ice_density() -> f64 { 917.0 }

impl Default for CryosphereConfig {
    fn default() -> Self {
        Self {
            plugin: PluginMeta {
                name: "cryosphere".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                description: "Cryosphere: snowmelt, glacier mass balance, permafrost".into(),
                category: geo_core::plugin::PluginCategory::Process,
                healthy: true,
                extra: serde_json::Value::Null,
            },
            degree_day_factor: default_ddf(),
            rain_snow_threshold: default_rain_snow(),
            ice_density: default_ice_density(),
        }
    }
}

impl PluginConfig for CryosphereConfig {}
