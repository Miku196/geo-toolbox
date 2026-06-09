//! 生态修复配置（rules.toml）。

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct EcologyConfig {
    pub plugin: PluginMeta,

    /// NDVI 阈值定义。
    #[serde(default)]
    pub ndvi: NdviThresholds,

    /// 碳密度参数（与 geo-plugin-carbon 的 rules.toml 结构相同，但独立维护）。
    #[serde(default)]
    pub carbon: CarbonParams,

    /// 报告模板路径。
    #[serde(default)]
    pub report: ReportConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NdviThresholds {
    /// 健康植被 NDVI 下限（≥ 此值 = 恢复良好）。
    #[serde(default = "default_healthy_min")]
    pub healthy_min: f64,

    /// 退化植被 NDVI 上限（≤ 此值 = 退化）。
    #[serde(default = "default_degraded_max")]
    pub degraded_max: f64,

    /// 显著改善的 NDVI 差值阈值（> 此值 = 恢复）。
    #[serde(default = "default_improvement_threshold")]
    pub improvement_threshold: f64,

    /// 显著退化的 NDVI 差值阈值（< 此值 = 退化）。
    #[serde(default = "default_degradation_threshold")]
    pub degradation_threshold: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CarbonParams {
    /// 方法学来源。
    #[serde(default = "default_source")]
    pub source: String,

    /// 森林碳汇 tCO₂e/ha/yr。
    #[serde(default = "default_forest")]
    pub forest: f64,

    /// 草地碳汇。
    #[serde(default = "default_grassland")]
    pub grassland: f64,

    /// 湿地。
    #[serde(default = "default_wetland")]
    pub wetland: f64,

    /// 农田。
    #[serde(default = "default_cropland")]
    pub cropland: f64,

    /// 建设用地排放。
    #[serde(default = "default_built_up")]
    pub built_up: f64,

    /// 裸地/矿区（恢复前）。
    #[serde(default)]
    pub bare: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReportConfig {
    /// 报告模板文件名（相对于 templates/ 目录）。
    #[serde(default = "default_template")]
    pub template: String,

    /// 输出格式：markdown | html。
    #[serde(default = "default_format")]
    pub format: String,
}

// ── 默认值 ──

fn default_healthy_min() -> f64 { 0.5 }
fn default_degraded_max() -> f64 { 0.2 }
fn default_improvement_threshold() -> f64 { 0.1 }
fn default_degradation_threshold() -> f64 { -0.1 }

fn default_source() -> String { "IPCC_2019".into() }
fn default_forest() -> f64 { -5.0 }
fn default_grassland() -> f64 { -1.2 }
fn default_wetland() -> f64 { -8.5 }
fn default_cropland() -> f64 { 0.5 }
fn default_built_up() -> f64 { 2.0 }

fn default_template() -> String { "restoration-report.md.tera".into() }
fn default_format() -> String { "markdown".into() }

impl Default for EcologyConfig {
    fn default() -> Self {
        toml::from_str(include_str!("../rules.toml"))
            .expect("Default ecology rules.toml is valid")
    }
}

impl CarbonParams {
    /// 根据土地覆盖类型获取碳密度。
    pub fn get_factor(&self, class: &str) -> Option<f64> {
        match class.to_lowercase().as_str() {
            "forest" => Some(self.forest),
            "grassland" => Some(self.grassland),
            "wetland" => Some(self.wetland),
            "cropland" | "crop" | "farmland" => Some(self.cropland),
            "built_up" | "builtup" | "urban" | "construction" => Some(self.built_up),
            "water" => Some(0.0),
            "bare" | "bareland" | "mining" => Some(self.bare),
            _ => None,
        }
    }
}

impl Default for NdviThresholds {
    fn default() -> Self {
        Self {
            healthy_min: default_healthy_min(),
            degraded_max: default_degraded_max(),
            improvement_threshold: default_improvement_threshold(),
            degradation_threshold: default_degradation_threshold(),
        }
    }
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
            bare: 0.0,
        }
    }
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self {
            template: default_template(),
            format: default_format(),
        }
    }
}

impl NdviThresholds {
    /// NDVI 值是否代表健康植被。
    pub fn is_healthy(&self, ndvi: f64) -> bool {
        ndvi >= self.healthy_min
    }

    /// NDVI 值是否代表退化植被。
    pub fn is_degraded(&self, ndvi: f64) -> bool {
        ndvi <= self.degraded_max
    }

    /// NDVI 变化是否代表显著改善。
    pub fn is_improvement(&self, diff: f64) -> bool {
        diff > self.improvement_threshold
    }

    /// NDVI 变化是否代表显著退化。
    pub fn is_degradation(&self, diff: f64) -> bool {
        diff < self.degradation_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = EcologyConfig::default();
        assert_eq!(config.ndvi.healthy_min, 0.5);
        assert_eq!(config.carbon.forest, -5.0);
    }
}
