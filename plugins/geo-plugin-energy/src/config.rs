//! 能源插件配置。

use geo_core::plugin::PluginConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct EnergyConfig {
    pub plugin: PluginMeta,
    #[serde(default)]
    pub solar: SolarConfig,
    #[serde(default)]
    pub wind: WindConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SolarConfig {
    #[serde(default = "default_slope_max")]
    pub slope_max_deg: f64,
    #[serde(default = "default_radiation_min")]
    pub radiation_min_kwh: f64,
    #[serde(default = "default_aspect_south")]
    pub aspect_south_weight: f64,
}

fn default_slope_max() -> f64 {
    25.0
}
fn default_radiation_min() -> f64 {
    1500.0
}
fn default_aspect_south() -> f64 {
    1.2
}

#[derive(Debug, Clone, Deserialize)]
pub struct WindConfig {
    #[serde(default = "default_wind_speed_min")]
    pub wind_speed_min_ms: f64,
    #[serde(default = "default_slope_max_wind")]
    pub slope_max_deg: f64,
}

fn default_wind_speed_min() -> f64 {
    5.5
}
fn default_slope_max_wind() -> f64 {
    15.0
}

impl Default for SolarConfig {
    fn default() -> Self {
        Self {
            slope_max_deg: 25.0,
            radiation_min_kwh: 1500.0,
            aspect_south_weight: 1.2,
        }
    }
}

impl Default for WindConfig {
    fn default() -> Self {
        Self {
            wind_speed_min_ms: 5.5,
            slope_max_deg: 15.0,
        }
    }
}

impl Default for EnergyConfig {
    fn default() -> Self {
        Self {
            plugin: PluginMeta {
                name: "energy".into(),
                version: "0.1.0".into(),
                description: "新能源选址评估".into(),
            },
            solar: SolarConfig::default(),
            wind: WindConfig::default(),
        }
    }
}
