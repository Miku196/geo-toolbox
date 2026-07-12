use geo_core::plugin::PluginConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct BiodiversityConfig {
    pub plugin: PluginMeta,
    #[serde(default)]
    pub sdm: SdmParams,
    #[serde(default)]
    pub connectivity: ConnectivityParams,
    #[serde(default)]
    pub gap: GapParams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SdmParams {
    /// Minimum suitability threshold (0-1)
    #[serde(default = "default_suitability_threshold")]
    pub suitability_threshold: f64,
    /// Number of presence points required for valid model
    #[serde(default = "default_min_presence_points")]
    pub min_presence_points: usize,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConnectivityParams {
    /// Edge effect depth in map units
    #[serde(default = "default_edge_depth")]
    pub edge_depth: f64,
    /// Minimum patch area for core habitat (map units²)
    #[serde(default = "default_min_core_area")]
    pub min_core_area: f64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GapParams {
    /// Target percentage of range to be protected
    #[serde(default = "default_target_pct")]
    pub target_pct: f64,
}

fn default_suitability_threshold() -> f64 {
    0.5
}
fn default_min_presence_points() -> usize {
    5
}
fn default_edge_depth() -> f64 {
    100.0
}
fn default_min_core_area() -> f64 {
    10000.0
}
fn default_target_pct() -> f64 {
    17.0
}

impl Default for BiodiversityConfig {
    fn default() -> Self {
        Self {
            plugin: PluginMeta {
                name: "biodiversity".into(),
                version: "0.1.0".into(),
                description: "生物多样性评估".into(),
            },
            sdm: SdmParams::default(),
            connectivity: ConnectivityParams::default(),
            gap: GapParams::default(),
        }
    }
}

impl PluginConfig for BiodiversityConfig {}
