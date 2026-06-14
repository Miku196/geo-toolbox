//! 生态修复配置（rules.toml）。

use serde::Deserialize;

/// 生态修复插件的顶级配置。
#[derive(Debug, Clone, Deserialize)]
pub struct EcologyConfig {
    pub plugin: PluginMeta,

    /// NDVI 阈值定义。
    #[serde(default)]
    pub ndvi: NdviThresholds,

    /// 碳密度参数（共享 `geo_carbon_math::CarbonParams`）。
    #[serde(default)]
    pub carbon: geo_carbon_math::CarbonParams,

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
pub struct ReportConfig {
    /// 报告模板文件名（相对于 templates/ 目录）。
    #[serde(default = "default_template")]
    pub template: String,

    /// 输出格式：markdown | html。
    #[serde(default = "default_format")]
    pub format: String,
}

// ── 默认值 ──

fn default_healthy_min() -> f64 {
    0.5
}
fn default_degraded_max() -> f64 {
    0.2
}
fn default_improvement_threshold() -> f64 {
    0.1
}
fn default_degradation_threshold() -> f64 {
    -0.1
}

fn default_template() -> String {
    "restoration-report.md.tera".into()
}
fn default_format() -> String {
    "markdown".into()
}

geo_core::default_from_rules!(EcologyConfig, "ecology");

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
