use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SurveyConfig {
    pub plugin: PluginMeta,
    #[serde(default)]
    pub adjustment: AdjustmentParams,
    #[serde(default)]
    pub earthwork: EarthworkParams,
    #[serde(default)]
    pub contour: ContourParams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}

/// 控制网平差参数。
#[derive(Debug, Clone, Deserialize)]
pub struct AdjustmentParams {
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    #[serde(default = "default_convergence")]
    pub convergence_threshold: f64,
    /// 单位权中误差阈值（m）。
    #[serde(default = "default_max_error")]
    pub max_error_m: f64,
}

/// 土方量计算参数。
#[derive(Debug, Clone, Deserialize)]
pub struct EarthworkParams {
    /// 方格网尺寸（m）。
    #[serde(default = "default_grid_size")]
    pub grid_size_m: f64,
    /// 填方膨胀系数（松土压实后体积缩小 1/loose_factor）。
    #[serde(default = "default_loose_factor")]
    pub loose_factor: f64,
    /// 挖方系数。
    #[serde(default = "default_cut_factor")]
    pub cut_factor: f64,
}

/// 等高线参数。
#[derive(Debug, Clone, Deserialize)]
pub struct ContourParams {
    #[serde(default = "default_contour_interval")]
    pub interval_m: f64,
}

fn default_max_iterations() -> u32 {
    50
}
fn default_convergence() -> f64 {
    0.001
}
fn default_max_error() -> f64 {
    0.05
}
fn default_grid_size() -> f64 {
    10.0
}
fn default_loose_factor() -> f64 {
    1.15
}
fn default_cut_factor() -> f64 {
    1.0
}
fn default_contour_interval() -> f64 {
    1.0
}

impl Default for AdjustmentParams {
    fn default() -> Self {
        Self {
            max_iterations: default_max_iterations(),
            convergence_threshold: default_convergence(),
            max_error_m: default_max_error(),
        }
    }
}
impl Default for EarthworkParams {
    fn default() -> Self {
        Self {
            grid_size_m: default_grid_size(),
            loose_factor: default_loose_factor(),
            cut_factor: default_cut_factor(),
        }
    }
}
impl Default for ContourParams {
    fn default() -> Self {
        Self {
            interval_m: default_contour_interval(),
        }
    }
}
