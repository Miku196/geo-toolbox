#![allow(non_snake_case)]

use geo_core::plugin::{PluginConfig, PluginMeta};
use serde::{Deserialize, Serialize};

/// 火山插件配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolcanologyConfig {
    pub plugin: PluginMeta,
    #[serde(default = "default_plume_height")]
    pub default_plume_height_m: f64,
    #[serde(default = "default_particle_density")]
    pub particle_density_kgm3: f64,
    #[serde(default = "default_grain_size_mm")]
    pub grain_size_mm: f64,
    #[serde(default = "default_effusion_rate")]
    pub effusion_rate_m3s: f64,
    #[serde(default = "default_viscosity")]
    #[allow(non_snake_case)]
    pub lava_viscosity_Pa_s: f64,
    #[serde(default = "default_slope")]
    pub default_slope_degrees: f64,
}

fn default_plume_height() -> f64 {
    5000.0
}
fn default_particle_density() -> f64 {
    2500.0
}
fn default_grain_size_mm() -> f64 {
    0.5
}
fn default_effusion_rate() -> f64 {
    500.0
}
fn default_viscosity() -> f64 {
    5000.0
}
fn default_slope() -> f64 {
    5.0
}

impl Default for VolcanologyConfig {
    fn default() -> Self {
        Self {
            plugin: PluginMeta {
                name: "volcanology".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                description: "Volcanology: ash dispersion, lava flow path, hazard zoning".into(),
                category: geo_core::plugin::PluginCategory::Process,
                healthy: true,
                extra: serde_json::Value::Null,
            },
            default_plume_height_m: default_plume_height(),
            particle_density_kgm3: default_particle_density(),
            grain_size_mm: default_grain_size_mm(),
            effusion_rate_m3s: default_effusion_rate(),
            lava_viscosity_Pa_s: default_viscosity(),
            default_slope_degrees: default_slope(),
        }
    }
}

impl PluginConfig for VolcanologyConfig {}
