use geo_core::plugin::PluginConfig;
use serde::Deserialize;

/// 地震插件配置。
#[derive(Debug, Clone, Deserialize)]
pub struct SeismologyConfig {
    pub plugin: PluginMeta,
    #[serde(default)]
    pub ground_motion: GroundMotionConfig,
    #[serde(default)]
    pub psha: PshaConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}

/// 地震动参数配置（GB 18306-2015 经验系数）。
#[derive(Debug, Clone, Deserialize)]
pub struct GroundMotionConfig {
    /// 默认场地类别 (I0, I1, II, III, IV)
    #[serde(default = "default_site_class")]
    pub default_site_class: String,
    /// PGA 衰减系数 a (g)
    #[serde(default = "default_pga_a")]
    pub pga_coeff_a: f64,
    /// PGA 衰减系数 b
    #[serde(default = "default_pga_b")]
    pub pga_coeff_b: f64,
    /// PGA 衰减系数 c
    #[serde(default = "default_pga_c")]
    pub pga_coeff_c: f64,
    /// II 类场地 PGA 放大因子
    #[serde(default = "default_site_ii")]
    pub pga_site_factor_ii: f64,
    /// III 类场地 PGA 放大因子
    #[serde(default = "default_site_iii")]
    pub pga_site_factor_iii: f64,
    /// IV 类场地 PGA 放大因子
    #[serde(default = "default_site_iv")]
    pub pga_site_factor_iv: f64,
    /// 反应谱阻尼比
    #[serde(default = "default_damping")]
    pub response_damping: f64,
}

fn default_site_class() -> String {
    "II".into()
}
fn default_pga_a() -> f64 {
    0.35
}
fn default_pga_b() -> f64 {
    0.05
}
fn default_pga_c() -> f64 {
    0.01
}
fn default_site_ii() -> f64 {
    1.0
}
fn default_site_iii() -> f64 {
    1.35
}
fn default_site_iv() -> f64 {
    1.8
}
fn default_damping() -> f64 {
    0.05
}

impl Default for GroundMotionConfig {
    fn default() -> Self {
        Self {
            default_site_class: default_site_class(),
            pga_coeff_a: default_pga_a(),
            pga_coeff_b: default_pga_b(),
            pga_coeff_c: default_pga_c(),
            pga_site_factor_ii: default_site_ii(),
            pga_site_factor_iii: default_site_iii(),
            pga_site_factor_iv: default_site_iv(),
            response_damping: default_damping(),
        }
    }
}

/// PSHA 配置。
#[derive(Debug, Clone, Deserialize)]
pub struct PshaConfig {
    /// 年均超越概率重现期列表
    #[serde(default = "default_return_periods")]
    pub return_periods: Vec<f64>,
    /// 最小震级
    #[serde(default = "default_min_mag")]
    pub min_magnitude: f64,
    /// 最大距离 (km)
    #[serde(default = "default_max_dist")]
    pub max_distance_km: f64,
}

fn default_return_periods() -> Vec<f64> {
    vec![50.0, 100.0, 475.0, 975.0, 2475.0]
}
fn default_min_mag() -> f64 {
    4.5
}
fn default_max_dist() -> f64 {
    300.0
}

impl Default for PshaConfig {
    fn default() -> Self {
        Self {
            return_periods: default_return_periods(),
            min_magnitude: default_min_mag(),
            max_distance_km: default_max_dist(),
        }
    }
}

impl Default for SeismologyConfig {
    fn default() -> Self {
        Self {
            plugin: PluginMeta {
                name: "seismology".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                description: "地震动参数预测、概率地震危险性分析、地震目录工具".into(),
            },
            ground_motion: GroundMotionConfig::default(),
            psha: PshaConfig::default(),
        }
    }
}

impl PluginConfig for SeismologyConfig {}
