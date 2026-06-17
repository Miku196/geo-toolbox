use geo_core::plugin::PluginConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct UrbanConfig {
    pub plugin: PluginMeta,
    #[serde(default)]
    pub density: DensityParams,
    #[serde(default)]
    pub land_use: LandUseParams,
    #[serde(default)]
    pub solar: SolarParams,
    #[serde(default)]
    pub uhi: UhiParams,
    #[serde(default)]
    pub vegetation: VegetationParams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}

/// 容积率 / 密度参数。
#[derive(Debug, Clone, Deserialize)]
pub struct DensityParams {
    #[serde(default = "default_far_max")]
    pub far_max: f64,
    #[serde(default = "default_density_max")]
    pub density_max: f64,
    #[serde(default = "default_height_per_floor")]
    pub height_per_floor_m: f64,
}

/// 用地分类参数（NLCD 不透水率阈值）。
#[derive(Debug, Clone, Deserialize)]
pub struct LandUseParams {
    /// 水体 NDVI 上限。
    #[serde(default = "default_water_ndvi_max")]
    pub water_ndvi_max: f64,
    /// 绿地不透水率上限。
    #[serde(default = "default_green_impervious_max")]
    pub green_impervious_max: f64,
    /// 低强度不透水率上限。
    #[serde(default = "default_low_impervious_max")]
    pub low_impervious_max: f64,
    /// 中强度不透水率上限。
    #[serde(default = "default_medium_impervious_max")]
    pub medium_impervious_max: f64,
}

/// 日照分析参数。
#[derive(Debug, Clone, Deserialize)]
pub struct SolarParams {
    /// 冬至日太阳高度角（度）。
    #[serde(default = "default_winter_altitude")]
    pub winter_sun_altitude_deg: f64,
    /// 夏至日太阳高度角（度）。
    #[serde(default = "default_summer_altitude")]
    pub summer_sun_altitude_deg: f64,
    /// 冬至日太阳方位角（度）。
    #[serde(default = "default_winter_azimuth")]
    pub winter_sun_azimuth_deg: f64,
}

/// 热岛效应参数。
#[derive(Debug, Clone, Deserialize)]
pub struct UhiParams {
    #[serde(default = "default_impervious_weight")]
    pub impervious_weight: f64,
    #[serde(default = "default_density_weight")]
    pub density_weight: f64,
    #[serde(default = "default_green_weight")]
    pub green_weight: f64,
    /// UHI 高风险阈值。
    #[serde(default = "default_uhi_high")]
    pub high_threshold: f64,
    /// UHI 中风险阈值。
    #[serde(default = "default_uhi_medium")]
    pub medium_threshold: f64,
}

/// 绿地参数。
#[derive(Debug, Clone, Deserialize)]
pub struct VegetationParams {
    #[serde(default = "default_min_green_ratio")]
    pub min_green_ratio: f64,
    #[serde(default = "default_min_green_per_capita_m2")]
    pub min_green_per_capita_m2: f64,
}

// ── defaults ──
fn default_far_max() -> f64 {
    3.5
}
fn default_density_max() -> f64 {
    0.4
}
fn default_height_per_floor() -> f64 {
    3.0
}

fn default_water_ndvi_max() -> f64 {
    -0.1
}
fn default_green_impervious_max() -> f64 {
    0.20
}
fn default_low_impervious_max() -> f64 {
    0.50
}
fn default_medium_impervious_max() -> f64 {
    0.80
}

fn default_winter_altitude() -> f64 {
    26.5
}
fn default_summer_altitude() -> f64 {
    73.5
}
fn default_winter_azimuth() -> f64 {
    135.0
}

fn default_impervious_weight() -> f64 {
    0.6
}
fn default_density_weight() -> f64 {
    0.3
}
fn default_green_weight() -> f64 {
    0.1
}
fn default_uhi_high() -> f64 {
    0.7
}
fn default_uhi_medium() -> f64 {
    0.4
}

fn default_min_green_ratio() -> f64 {
    0.30
}
fn default_min_green_per_capita_m2() -> f64 {
    9.0
}

// ── Default impls ──
impl Default for PluginMeta {
    fn default() -> Self {
        Self {
            name: "urban".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            description: "城市规划：容积率、用地分类、日照、热岛、通风廊道".into(),
        }
    }
}

impl Default for UrbanConfig {
    fn default() -> Self {
        Self {
            plugin: PluginMeta::default(),
            density: DensityParams::default(),
            land_use: LandUseParams::default(),
            solar: SolarParams::default(),
            uhi: UhiParams::default(),
            vegetation: VegetationParams::default(),
        }
    }
}

impl PluginConfig for UrbanConfig {}

impl Default for DensityParams {
    fn default() -> Self {
        Self {
            far_max: default_far_max(),
            density_max: default_density_max(),
            height_per_floor_m: default_height_per_floor(),
        }
    }
}
impl Default for LandUseParams {
    fn default() -> Self {
        Self {
            water_ndvi_max: default_water_ndvi_max(),
            green_impervious_max: default_green_impervious_max(),
            low_impervious_max: default_low_impervious_max(),
            medium_impervious_max: default_medium_impervious_max(),
        }
    }
}
impl Default for SolarParams {
    fn default() -> Self {
        Self {
            winter_sun_altitude_deg: default_winter_altitude(),
            summer_sun_altitude_deg: default_summer_altitude(),
            winter_sun_azimuth_deg: default_winter_azimuth(),
        }
    }
}
impl Default for UhiParams {
    fn default() -> Self {
        Self {
            impervious_weight: default_impervious_weight(),
            density_weight: default_density_weight(),
            green_weight: default_green_weight(),
            high_threshold: default_uhi_high(),
            medium_threshold: default_uhi_medium(),
        }
    }
}
impl Default for VegetationParams {
    fn default() -> Self {
        Self {
            min_green_ratio: default_min_green_ratio(),
            min_green_per_capita_m2: default_min_green_per_capita_m2(),
        }
    }
}
