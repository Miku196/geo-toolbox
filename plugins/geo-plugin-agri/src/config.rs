use serde::Deserialize;

/// Agriculture plugin top-level configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct AgriConfig {
    pub plugin: PluginMeta,

    /// Per-crop yield parameters.
    #[serde(default)]
    pub crops: CropsConfig,

    /// Soil rating thresholds.
    #[serde(default)]
    pub soil: SoilParams,

    /// Irrigation calculation parameters.
    #[serde(default)]
    pub irrigation: IrrigationParams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}

/// ── Crop-specific parameters ──

#[derive(Debug, Clone, Deserialize)]
pub struct CropsConfig {
    #[serde(default)]
    pub wheat: CropParams,
    #[serde(default)]
    pub corn: CropParams,
    #[serde(default)]
    pub rice: CropParams,
    #[serde(default)]
    pub soybean: CropParams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CropParams {
    /// Extinction coefficient for NDVI→LAI conversion.
    #[serde(default = "default_k")]
    pub k: f64,

    /// Light use efficiency (gC/MJ).
    #[serde(default = "default_lue")]
    pub light_use_efficiency: f64,

    /// Harvest index (fraction of biomass that is yield).
    #[serde(default = "default_hi")]
    pub harvest_index: f64,

    /// Crop coefficient Kc for irrigation (FAO-56).
    #[serde(default = "default_kc")]
    pub kc: f64,

    /// NDVI weight multiplier for simplified yield estimation.
    #[serde(default = "default_ndvi_weight")]
    pub ndvi_weight: f64,

    /// Area-unit yield coefficient (kg/ha baseline).
    #[serde(default = "default_baseline_yield")]
    pub baseline_yield_kg_ha: f64,
}

fn default_k() -> f64 {
    0.5
}
fn default_lue() -> f64 {
    1.8
}
fn default_hi() -> f64 {
    0.45
}
fn default_kc() -> f64 {
    0.85
}
fn default_ndvi_weight() -> f64 {
    1.2
}
fn default_baseline_yield() -> f64 {
    6000.0
}

impl Default for CropParams {
    fn default() -> Self {
        Self {
            k: default_k(),
            light_use_efficiency: default_lue(),
            harvest_index: default_hi(),
            kc: default_kc(),
            ndvi_weight: default_ndvi_weight(),
            baseline_yield_kg_ha: default_baseline_yield(),
        }
    }
}

impl Default for CropsConfig {
    fn default() -> Self {
        Self {
            wheat: CropParams {
                k: 0.55,
                light_use_efficiency: 1.8,
                harvest_index: 0.42,
                kc: 0.85,
                ndvi_weight: 1.1,
                baseline_yield_kg_ha: 5500.0,
            },
            corn: CropParams {
                k: 0.65,
                light_use_efficiency: 2.2,
                harvest_index: 0.48,
                kc: 0.75,
                ndvi_weight: 1.3,
                baseline_yield_kg_ha: 7500.0,
            },
            rice: CropParams {
                k: 0.50,
                light_use_efficiency: 1.6,
                harvest_index: 0.50,
                kc: 1.05,
                ndvi_weight: 1.0,
                baseline_yield_kg_ha: 6000.0,
            },
            soybean: CropParams {
                k: 0.60,
                light_use_efficiency: 1.4,
                harvest_index: 0.35,
                kc: 0.80,
                ndvi_weight: 0.9,
                baseline_yield_kg_ha: 3000.0,
            },
        }
    }
}

impl CropsConfig {
    pub fn get(&self, crop_type: &str) -> Option<&CropParams> {
        match crop_type.to_lowercase().as_str() {
            "wheat" | "小麦" => Some(&self.wheat),
            "corn" | "maize" | "玉米" => Some(&self.corn),
            "rice" | "水稻" => Some(&self.rice),
            "soybean" | "大豆" => Some(&self.soybean),
            _ => None,
        }
    }
}

/// ── Soil rating parameters ──

#[derive(Debug, Clone, Deserialize)]
pub struct SoilParams {
    /// Minimum organic matter (%) for "good" rating.
    #[serde(default = "default_om_good")]
    pub om_good: f64,
    /// Minimum organic matter (%) for "moderate" rating.
    #[serde(default = "default_om_moderate")]
    pub om_moderate: f64,

    /// pH lower bound for optimal range.
    #[serde(default = "default_ph_lo")]
    pub ph_lo: f64,
    /// pH upper bound for optimal range.
    #[serde(default = "default_ph_hi")]
    pub ph_hi: f64,

    /// Available nitrogen threshold (mg/kg) for "good".
    #[serde(default = "default_n_good")]
    pub n_good: f64,
    /// Available phosphorus threshold (mg/kg) for "good".
    #[serde(default = "default_p_good")]
    pub p_good: f64,
    /// Available potassium threshold (mg/kg) for "good".
    #[serde(default = "default_k_good")]
    pub k_good: f64,

    /// Soil texture suitability weights (loam = 1.0, sand = 0.6, clay = 0.8).
    #[serde(default = "default_texture_weights")]
    pub texture_weights: std::collections::HashMap<String, f64>,
}

fn default_om_good() -> f64 {
    3.0
}
fn default_om_moderate() -> f64 {
    1.5
}
fn default_ph_lo() -> f64 {
    5.5
}
fn default_ph_hi() -> f64 {
    8.0
}
fn default_n_good() -> f64 {
    120.0
}
fn default_p_good() -> f64 {
    20.0
}
fn default_k_good() -> f64 {
    150.0
}

fn default_texture_weights() -> std::collections::HashMap<String, f64> {
    let mut m = std::collections::HashMap::new();
    m.insert("loam".into(), 1.0);
    m.insert("clay_loam".into(), 0.95);
    m.insert("sandy_loam".into(), 0.85);
    m.insert("silt_loam".into(), 0.90);
    m.insert("clay".into(), 0.75);
    m.insert("sand".into(), 0.55);
    m.insert("silt".into(), 0.70);
    m
}

impl Default for SoilParams {
    fn default() -> Self {
        Self {
            om_good: default_om_good(),
            om_moderate: default_om_moderate(),
            ph_lo: default_ph_lo(),
            ph_hi: default_ph_hi(),
            n_good: default_n_good(),
            p_good: default_p_good(),
            k_good: default_k_good(),
            texture_weights: default_texture_weights(),
        }
    }
}

/// ── Irrigation parameters ──

#[derive(Debug, Clone, Deserialize)]
pub struct IrrigationParams {
    /// Application efficiency (fraction).
    #[serde(default = "default_application_efficiency")]
    pub application_efficiency: f64,

    /// Management allowed depletion (fraction).
    #[serde(default = "default_mad")]
    pub mad: f64,

    /// Soil available water capacity (mm/m).
    #[serde(default = "default_awc")]
    pub awc_mm_per_m: f64,

    /// Rooting depth (m).
    #[serde(default = "default_root_depth")]
    pub root_depth_m: f64,
}

fn default_application_efficiency() -> f64 {
    0.75
}
fn default_mad() -> f64 {
    0.50
}
fn default_awc() -> f64 {
    120.0
}
fn default_root_depth() -> f64 {
    1.0
}

impl Default for IrrigationParams {
    fn default() -> Self {
        Self {
            application_efficiency: default_application_efficiency(),
            mad: default_mad(),
            awc_mm_per_m: default_awc(),
            root_depth_m: default_root_depth(),
        }
    }
}

impl Default for AgriConfig {
    fn default() -> Self {
        toml::from_str(include_str!("../rules.toml")).expect("Default agri rules.toml is valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = AgriConfig::default();
        assert_eq!(cfg.plugin.name, "agri");
        assert_eq!(cfg.crops.wheat.harvest_index, 0.42);
        assert_eq!(cfg.soil.ph_lo, 5.5);
        assert_eq!(cfg.irrigation.application_efficiency, 0.75);
    }

    #[test]
    fn test_get_crop_params() {
        let cfg = CropsConfig::default();
        assert!(cfg.get("wheat").is_some());
        assert!(cfg.get("玉米").is_some());
        assert_eq!(cfg.get("corn").unwrap().kc, 0.75);
        assert!(cfg.get("unknown").is_none());
    }
}
