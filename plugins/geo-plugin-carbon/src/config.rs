//! 碳核算配置（从 rules.toml 加载）。
//!
//! 碳密度参数使用 `geo_carbon_math::CarbonParams`，避免与 ecology 等插件重复定义。

use geo_core::plugin::PluginConfig;
use serde::Deserialize;

/// 碳核算插件的顶级配置。
#[derive(Debug, Clone, Deserialize)]
pub struct CarbonConfig {
    /// 插件元信息。
    pub plugin: PluginMeta,

    /// 各土地覆盖类型的碳密度参数（共享定义）。
    #[serde(default)]
    pub carbon: geo_carbon_math::CarbonParams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}

impl Default for CarbonConfig {
    fn default() -> Self {
        Self {
            plugin: PluginMeta {
                name: "carbon".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                description: "IPCC Tier 1 carbon accounting plugin".into(),
            },
            carbon: geo_carbon_math::CarbonParams::default(),
        }
    }
}

impl PluginConfig for CarbonConfig {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_factors() {
        let config: CarbonConfig = toml::from_str(
            "[plugin]\nname = \"carbon\"\nversion = \"0.1.0\"\ndescription = \"test\"\n",
        )
        .unwrap();
        assert_eq!(config.carbon.get_factor("forest"), Some(-5.0));
        assert_eq!(config.carbon.get_factor("unknown"), None);
    }
}
