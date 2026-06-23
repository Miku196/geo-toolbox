use geo_core::plugin::{PluginConfig, PluginMeta};
use serde::{Deserialize, Serialize};

/// 大气插件配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtmosphereConfig {
    pub plugin: PluginMeta,
    /// 默认地表粗糙度 (m)
    #[serde(default = "default_roughness")]
    pub roughness_default: f64,
    /// 科里奥利参数 (rad/s)
    #[serde(default = "default_coriolis")]
    pub coriolis_default: f64,
    /// AOD550→PM2.5 转化系数
    #[serde(default = "default_aod_ratio")]
    pub aod550_pm25_ratio: f64,
    /// 相对湿度校正
    #[serde(default = "default_rh_corr")]
    pub rh_correction_factor: f64,
}

fn default_roughness() -> f64 {
    0.1
}
fn default_coriolis() -> f64 {
    1.0e-4
}
fn default_aod_ratio() -> f64 {
    0.55
}
fn default_rh_corr() -> f64 {
    0.85
}

impl Default for AtmosphereConfig {
    fn default() -> Self {
        Self {
            plugin: PluginMeta {
                name: "atmosphere".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                description: "Atmospheric science: boundary layer, Gaussian dispersion, AOD→PM2.5"
                    .into(),
                category: geo_core::plugin::PluginCategory::Process,
                healthy: true,
                extra: serde_json::Value::Null,
            },
            roughness_default: default_roughness(),
            coriolis_default: default_coriolis(),
            aod550_pm25_ratio: default_aod_ratio(),
            rh_correction_factor: default_rh_corr(),
        }
    }
}

impl PluginConfig for AtmosphereConfig {}
