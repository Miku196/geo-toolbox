//! 碳核算配置（从 rules.toml 加载）。

use serde::Deserialize;

/// 碳核算插件的配置。
#[derive(Debug, Clone, Deserialize)]
pub struct CarbonConfig {
    /// 插件元信息。
    pub plugin: PluginMeta,

    /// 各土地覆盖类型的碳密度参数（tCO₂e/ha/yr）。
    /// 正值 = 排放源，负值 = 碳汇。
    #[serde(default)]
    pub carbon: CarbonDefaults,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CarbonDefaults {
    /// 碳核算方法学来源（如 "IPCC_2019"）。
    #[serde(default = "default_source")]
    pub source: String,

    /// 森林（tCO₂e/ha/yr，默认为 -5.0 = 碳汇）。
    #[serde(default = "default_forest")]
    pub forest: f64,

    /// 草地。
    #[serde(default = "default_grassland")]
    pub grassland: f64,

    /// 湿地。
    #[serde(default = "default_wetland")]
    pub wetland: f64,

    /// 农田。
    #[serde(default = "default_cropland")]
    pub cropland: f64,

    /// 建设用地。
    #[serde(default = "default_built_up")]
    pub built_up: f64,

    /// 水体。
    #[serde(default)]
    pub water: f64,

    /// 裸地。
    #[serde(default)]
    pub bare: f64,
}

fn default_source() -> String { "IPCC_2019".into() }
fn default_forest() -> f64 { -5.0 }
fn default_grassland() -> f64 { -1.2 }
fn default_wetland() -> f64 { -8.5 }
fn default_cropland() -> f64 { 0.5 }
fn default_built_up() -> f64 { 2.0 }

impl CarbonDefaults {
    /// 根据土地覆盖类型名获取碳密度。
    pub fn get_factor(&self, class: &str) -> Option<f64> {
        match class.to_lowercase().as_str() {
            "forest" => Some(self.forest),
            "grassland" => Some(self.grassland),
            "wetland" => Some(self.wetland),
            "cropland" | "crop" | "farmland" => Some(self.cropland),
            "built_up" | "builtup" | "urban" | "construction" => Some(self.built_up),
            "water" => Some(self.water),
            "bare" | "bareland" | "mining" => Some(self.bare),
            _ => None,
        }
    }
}

impl Default for CarbonDefaults {
    fn default() -> Self {
        Self {
            source: "IPCC_2019".into(),
            forest: -5.0,
            grassland: -1.2,
            wetland: -8.5,
            cropland: 0.5,
            built_up: 2.0,
            water: 0.0,
            bare: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_factors() {
        let defaults = CarbonDefaults::default();
        assert_eq!(defaults.get_factor("forest"), Some(-5.0));
        assert_eq!(defaults.get_factor("grAssland"), Some(-1.2));
        assert_eq!(defaults.get_factor("unknown"), None);
    }
}
