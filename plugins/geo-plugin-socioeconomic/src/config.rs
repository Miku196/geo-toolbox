use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocioeconomicConfig {
    pub plugin: PluginMeta,
    #[serde(default)]
    pub population: PopulationConfig,
    #[serde(default)]
    pub landuse: LanduseConfig,
    #[serde(default)]
    pub accessibility: AccessibilityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopulationConfig {
    #[serde(default = "default_cell_area")]
    pub default_cell_area_km2: f64,
    #[serde(default = "default_ntl_cal")]
    pub nightlight_calibration_factor: f64,
    #[serde(default = "default_win")]
    pub wealth_window_size: usize,
}

fn default_cell_area() -> f64 {
    0.01
}
fn default_ntl_cal() -> f64 {
    0.5
}
fn default_win() -> usize {
    3
}

impl Default for PopulationConfig {
    fn default() -> Self {
        Self {
            default_cell_area_km2: default_cell_area(),
            nightlight_calibration_factor: default_ntl_cal(),
            wealth_window_size: default_win(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanduseConfig {
    #[serde(default = "default_iter")]
    pub transition_iterations: usize,
    #[serde(default = "default_nb")]
    pub neighborhood_weight: f64,
    #[serde(default = "default_decay")]
    pub driver_influence_decay: f64,
}

fn default_iter() -> usize {
    10
}
fn default_nb() -> f64 {
    0.3
}
fn default_decay() -> f64 {
    0.5
}

impl Default for LanduseConfig {
    fn default() -> Self {
        Self {
            transition_iterations: default_iter(),
            neighborhood_weight: default_nb(),
            driver_influence_decay: default_decay(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityConfig {
    #[serde(default = "default_max_cost")]
    pub max_travel_cost: f64,
    #[serde(default = "default_decay_param")]
    pub default_decay_parameter: f64,
}

fn default_max_cost() -> f64 {
    120.0
}
fn default_decay_param() -> f64 {
    0.05
}

impl Default for AccessibilityConfig {
    fn default() -> Self {
        Self {
            max_travel_cost: default_max_cost(),
            default_decay_parameter: default_decay_param(),
        }
    }
}

impl Default for SocioeconomicConfig {
    fn default() -> Self {
        Self {
            plugin: PluginMeta {
                name: "socioeconomic".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                description: "社会经济分析：人口空间化、GDP估算、土地变化模拟、可达性".into(),
            },
            population: PopulationConfig::default(),
            landuse: LanduseConfig::default(),
            accessibility: AccessibilityConfig::default(),
        }
    }
}

impl geo_core::plugin::PluginConfig for SocioeconomicConfig {}
