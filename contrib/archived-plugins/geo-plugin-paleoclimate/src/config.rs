use geo_core::plugin::{PluginConfig, PluginMeta};
use serde::{Deserialize, Serialize};

/// 古气候/古地理插件配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaleoclimateConfig {
    pub plugin: PluginMeta,
    #[serde(default = "default_eustatic_lgm")]
    pub eustatic_lgm_m: f64,
    #[serde(default = "default_isostatic_frac")]
    pub isostatic_fraction: f64,
    #[serde(default = "default_d18o_sst_slope")]
    pub d18o_to_sst_slope: f64,
    #[serde(default = "default_ch4_temp_gradient")]
    pub ch4_to_temp_gradient: f64,
}

fn default_eustatic_lgm() -> f64 {
    -125.0
}
fn default_isostatic_frac() -> f64 {
    0.3
}
fn default_d18o_sst_slope() -> f64 {
    -4.5
}
fn default_ch4_temp_gradient() -> f64 {
    0.055
}

impl Default for PaleoclimateConfig {
    fn default() -> Self {
        Self {
            plugin: PluginMeta {
                name: "paleoclimate".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                description:
                    "Paleoclimate: sea level reconstruction, paleocoastline, proxy inversion".into(),
                category: geo_core::plugin::PluginCategory::Process,
                healthy: true,
                extra: serde_json::Value::Null,
            },
            eustatic_lgm_m: default_eustatic_lgm(),
            isostatic_fraction: default_isostatic_frac(),
            d18o_to_sst_slope: default_d18o_sst_slope(),
            ch4_to_temp_gradient: default_ch4_temp_gradient(),
        }
    }
}

impl PluginConfig for PaleoclimateConfig {}
