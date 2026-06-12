//! 碳密度配置参数 — 与土地覆盖类型对应的排放因子。
//!
//! 各 Plugin 通过 rules.toml 反序列化此结构体，
//! 然后喂入 `CarbonEngine` 生成 `EmissionFactor` 列表。

use serde::Deserialize;

/// 碳密度参数配置。
///
/// 每个土地覆盖类型对应一个排放因子（tCO₂e/ha/yr）。
/// 正值 = 排放源，负值 = 碳汇。
#[derive(Debug, Clone, Deserialize)]
pub struct CarbonParams {
    /// 方法学来源（如 "IPCC_2019"）。
    #[serde(default = "default_source")]
    pub source: String,

    /// 森林碳汇 tCO₂e/ha/yr。
    #[serde(default = "default_forest")]
    pub forest: f64,

    /// 草地碳汇。
    #[serde(default = "default_grassland")]
    pub grassland: f64,

    /// 湿地碳汇。
    #[serde(default = "default_wetland")]
    pub wetland: f64,

    /// 农田排放。
    #[serde(default = "default_cropland")]
    pub cropland: f64,

    /// 建设用地排放。
    #[serde(default = "default_built_up")]
    pub built_up: f64,

    /// 水体（通常为 0）。
    #[serde(default)]
    pub water: f64,

    /// 裸地/矿区。
    #[serde(default)]
    pub bare: f64,
}

fn default_source() -> String {
    "IPCC_2019".into()
}
fn default_forest() -> f64 {
    -5.0
}
fn default_grassland() -> f64 {
    -1.2
}
fn default_wetland() -> f64 {
    -8.5
}
fn default_cropland() -> f64 {
    0.5
}
fn default_built_up() -> f64 {
    2.0
}

impl Default for CarbonParams {
    fn default() -> Self {
        Self {
            source: default_source(),
            forest: default_forest(),
            grassland: default_grassland(),
            wetland: default_wetland(),
            cropland: default_cropland(),
            built_up: default_built_up(),
            water: 0.0,
            bare: 0.0,
        }
    }
}

impl CarbonParams {
    /// 根据土地覆盖类型名获取碳密度（tCO₂e/ha/yr）。
    ///
    /// 支持别名：cropland/crop/farmland, built_up/builtup/urban/construction,
    /// bare/bareland/mining。
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_factors() {
        let defaults = CarbonParams::default();
        assert_eq!(defaults.get_factor("forest"), Some(-5.0));
        assert_eq!(defaults.get_factor("grAssland"), Some(-1.2));
        assert_eq!(defaults.get_factor("unknown"), None);
    }

    #[test]
    fn test_aliases() {
        let defaults = CarbonParams::default();
        assert_eq!(defaults.get_factor("crop"), Some(0.5));
        assert_eq!(defaults.get_factor("farmland"), Some(0.5));
        assert_eq!(defaults.get_factor("urban"), Some(2.0));
        assert_eq!(defaults.get_factor("builtup"), Some(2.0));
        assert_eq!(defaults.get_factor("mining"), Some(0.0));
    }
}
