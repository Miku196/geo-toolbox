use serde::{Deserialize, Serialize};

/// 遥感插件配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSensingConfig {
    pub plugin: PluginMeta,
    #[serde(default)]
    pub radiometric: RadiometricConfig,
    #[serde(default)]
    pub insar: InsarConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadiometricConfig {
    /// TOA 辐射定标增益 (per band)
    #[serde(default = "default_gain")]
    pub toa_radiance_gain: Vec<f64>,
    /// TOA 辐射定标偏移 (per band)
    #[serde(default = "default_bias")]
    pub toa_radiance_bias: Vec<f64>,
    /// 太阳高度角 (度)
    #[serde(default = "default_sun_elev")]
    pub sun_elevation_deg: f64,
    /// 日地距离 (AU)
    #[serde(default = "default_sed")]
    pub sun_earth_distance_au: f64,
    /// 暗目标百分位数
    #[serde(default = "default_dark_pct")]
    pub dark_object_pct: f64,
    /// 云检测 NDVI 阈值
    #[serde(default = "default_cloud_ndvi")]
    pub cloud_ndvi_threshold: f64,
}

fn default_gain() -> Vec<f64> {
    vec![0.1, 0.08, 0.06]
}
fn default_bias() -> Vec<f64> {
    vec![-0.5, -0.3, -0.2]
}
fn default_sun_elev() -> f64 {
    50.0
}
fn default_sed() -> f64 {
    1.0
}
fn default_dark_pct() -> f64 {
    0.01
}
fn default_cloud_ndvi() -> f64 {
    0.2
}

impl Default for RadiometricConfig {
    fn default() -> Self {
        Self {
            toa_radiance_gain: default_gain(),
            toa_radiance_bias: default_bias(),
            sun_elevation_deg: default_sun_elev(),
            sun_earth_distance_au: default_sed(),
            dark_object_pct: default_dark_pct(),
            cloud_ndvi_threshold: default_cloud_ndvi(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsarConfig {
    /// 相干性计算窗口大小
    #[serde(default = "default_coherence_window")]
    pub coherence_window: usize,
    /// 相位噪声标准差
    #[serde(default = "default_phase_sigma")]
    pub phase_sigma: f64,
    /// 雷达波长 (cm)
    #[serde(default = "default_wavelength")]
    pub wavelength_cm: f64,
    /// 解缠容差
    #[serde(default = "default_unwrap_tol")]
    pub unwrap_tolerance: f64,
}

fn default_coherence_window() -> usize {
    5
}
fn default_phase_sigma() -> f64 {
    0.3
}
fn default_wavelength() -> f64 {
    5.6
}
fn default_unwrap_tol() -> f64 {
    0.5
}

impl Default for InsarConfig {
    fn default() -> Self {
        Self {
            coherence_window: default_coherence_window(),
            phase_sigma: default_phase_sigma(),
            wavelength_cm: default_wavelength(),
            unwrap_tolerance: default_unwrap_tol(),
        }
    }
}

impl Default for RemoteSensingConfig {
    fn default() -> Self {
        Self {
            plugin: PluginMeta {
                name: "remote-sensing".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                description: "遥感影像辐射校正、大气校正、InSAR 形变监测".into(),
            },
            radiometric: RadiometricConfig::default(),
            insar: InsarConfig::default(),
        }
    }
}

impl geo_core::plugin::PluginConfig for RemoteSensingConfig {}
