use geo_core::plugin::PluginConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct HealthConfig {
    pub plugin: PluginMeta,
}
#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}
impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            plugin: PluginMeta {
                name: "public-health".into(),
                version: "0.1.0".into(),
                description: "公共卫生环境健康评估".into(),
            },
        }
    }
}
impl PluginConfig for HealthConfig {}
