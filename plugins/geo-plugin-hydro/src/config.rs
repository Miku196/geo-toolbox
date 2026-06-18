use geo_core::plugin::PluginConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HydroConfig {
    pub plugin: PluginMeta,
    #[serde(default)]
    pub flood: FloodParams,
    #[serde(default)]
    pub runoff: RunoffParams,
    #[serde(default)]
    pub catchment: CatchmentParams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}

/// 洪水参数。
#[derive(Debug, Clone, Deserialize)]
pub struct FloodParams {
    #[serde(default = "default_return_period")]
    pub return_period_years: u32,
    #[serde(default = "default_safety_factor")]
    pub safety_factor: f64,
    /// 曼宁粗糙系数（用于洪水演进简算）。
    #[serde(default = "default_manning_n")]
    pub manning_n: f64,
}

/// 径流参数（推理公式法）。
#[derive(Debug, Clone, Deserialize)]
pub struct RunoffParams {
    #[serde(default = "default_impervious_c")]
    pub impervious_c: f64,
    #[serde(default = "default_grass_c")]
    pub grass_c: f64,
    #[serde(default = "default_forest_c")]
    pub forest_c: f64,
}

/// 集水区参数。
#[derive(Debug, Clone, Deserialize)]
pub struct CatchmentParams {
    /// D8 坡度阈值（低于此值视为平地）。
    #[serde(default = "default_slope_threshold")]
    pub slope_threshold: f64,
}

fn default_return_period() -> u32 {
    100
}
fn default_safety_factor() -> f64 {
    1.2
}
fn default_manning_n() -> f64 {
    0.035
}
fn default_impervious_c() -> f64 {
    0.9
}
fn default_grass_c() -> f64 {
    0.25
}
fn default_forest_c() -> f64 {
    0.15
}
fn default_slope_threshold() -> f64 {
    0.01
}

impl Default for PluginMeta {
    fn default() -> Self {
        Self {
            name: "hydro".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            description: "水文分析：D8 汇流、径流、淹没分析、集水区提取".into(),
        }
    }
}

impl PluginConfig for HydroConfig {}

impl Default for FloodParams {
    fn default() -> Self {
        Self {
            return_period_years: default_return_period(),
            safety_factor: default_safety_factor(),
            manning_n: default_manning_n(),
        }
    }
}
impl Default for RunoffParams {
    fn default() -> Self {
        Self {
            impervious_c: default_impervious_c(),
            grass_c: default_grass_c(),
            forest_c: default_forest_c(),
        }
    }
}
impl Default for CatchmentParams {
    fn default() -> Self {
        Self {
            slope_threshold: default_slope_threshold(),
        }
    }
}
