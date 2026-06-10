use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ForestryConfig {
    pub plugin: PluginMeta,
    #[serde(default)]
    pub carbon: CarbonParams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CarbonParams {
    #[serde(default = "default_source")]
    pub source: String,
    /// 生物量扩展因子 (BEF)
    #[serde(default = "default_bef")]
    pub biomass_expansion_factor: f64,
    /// 根冠比
    #[serde(default = "default_root_shoot")]
    pub root_shoot_ratio: f64,
    /// 含碳率
    #[serde(default = "default_carbon_fraction")]
    pub carbon_fraction: f64,
    /// 木材密度 (t/m³)
    #[serde(default = "default_wood_density")]
    pub wood_density: f64,
    /// CO₂/C 分子量比
    #[serde(default = "default_co2_c_ratio")]
    pub co2_c_ratio: f64,
}

fn default_source() -> String { "IPCC_2019".into() }
fn default_bef() -> f64 { 1.7 }
fn default_root_shoot() -> f64 { 0.25 }
fn default_carbon_fraction() -> f64 { 0.47 }
fn default_wood_density() -> f64 { 0.55 }
fn default_co2_c_ratio() -> f64 { 44.0 / 12.0 }

impl Default for ForestryConfig {
    fn default() -> Self {
        Self {
            plugin: PluginMeta {
                name: "forestry".into(),
                version: "0.1.0".into(),
                description: "林业碳汇计量".into(),
            },
            carbon: CarbonParams {
                source: "IPCC_2019".into(),
                biomass_expansion_factor: 1.7,
                root_shoot_ratio: 0.25,
                carbon_fraction: 0.47,
                wood_density: 0.55,
                co2_c_ratio: 44.0 / 12.0,
            },
        }
    }
}
