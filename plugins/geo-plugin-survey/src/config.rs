//! 测绘配置。

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SurveyConfig {
    pub plugin: PluginMeta,
    #[serde(default)]
    pub adjustment: AdjustmentParams,
    #[serde(default)]
    pub earthwork: EarthworkParams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdjustmentParams {
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    #[serde(default = "default_convergence")]
    pub convergence_threshold: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EarthworkParams {
    #[serde(default = "default_grid_size")]
    pub grid_size_m: f64,
}

fn default_max_iterations() -> u32 { 50 }
fn default_convergence() -> f64 { 0.001 }
fn default_grid_size() -> f64 { 10.0 }

impl Default for AdjustmentParams { fn default() -> Self { Self { max_iterations: 50, convergence_threshold: 0.001 } } }
impl Default for EarthworkParams { fn default() -> Self { Self { grid_size_m: 10.0 } } }
