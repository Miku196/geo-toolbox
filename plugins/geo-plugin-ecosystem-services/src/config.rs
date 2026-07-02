use geo_core::plugin::PluginConfig;
use serde::Deserialize;
#[derive(Debug, Clone, Deserialize)]
pub struct EcosystemConfig {
    pub plugin: PluginMeta,
}
#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}
impl Default for EcosystemConfig {
    fn default() -> Self {
        Self {
            plugin: PluginMeta {
                name: "ecosystem-services".into(),
                version: "0.1.0".into(),
                description: "生态系统服务评估".into(),
            },
        }
    }
}
impl PluginConfig for EcosystemConfig {}
