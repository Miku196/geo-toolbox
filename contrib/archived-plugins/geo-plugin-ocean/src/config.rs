use geo_core::plugin::PluginConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct OceanConfig {
    pub plugin: PluginMeta,
    #[serde(default)]
    pub coral: CoralParams,
    #[serde(default)]
    pub upwelling: UpwellingParams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CoralParams {
    #[serde(default = "default_dhw_threshold")]
    pub dhw_bleaching_threshold: f64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpwellingParams {
    #[serde(default = "default_ekman_min")]
    pub ekman_transport_min: f64,
}

fn default_dhw_threshold() -> f64 {
    4.0
}
fn default_ekman_min() -> f64 {
    0.5
}

impl Default for OceanConfig {
    fn default() -> Self {
        Self {
            plugin: PluginMeta {
                name: "ocean".into(),
                version: "0.1.0".into(),
                description: "海洋学分析".into(),
            },
            coral: CoralParams::default(),
            upwelling: UpwellingParams::default(),
        }
    }
}
impl PluginConfig for OceanConfig {}
