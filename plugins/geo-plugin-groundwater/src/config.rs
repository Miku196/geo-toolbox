use geo_core::plugin::PluginConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct GroundwaterConfig {
    pub plugin: PluginMeta,
    #[serde(default)]
    pub aquifer: AquiferParams,
    #[serde(default)]
    pub recharge: RechargeParams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AquiferParams {
    /// Default specific yield (dimensionless, 0.01–0.40)
    #[serde(default = "default_specific_yield")]
    pub specific_yield: f64,
    /// Default hydraulic conductivity (m/day)
    #[serde(default = "default_hydraulic_conductivity")]
    pub hydraulic_conductivity: f64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RechargeParams {
    /// Default recharge coefficient (% of precipitation)
    #[serde(default = "default_recharge_coeff")]
    pub recharge_coeff: f64,
}

fn default_specific_yield() -> f64 {
    0.15
}
fn default_hydraulic_conductivity() -> f64 {
    10.0
}
fn default_recharge_coeff() -> f64 {
    0.15
}

impl Default for GroundwaterConfig {
    fn default() -> Self {
        Self {
            plugin: PluginMeta {
                name: "groundwater".into(),
                version: "0.1.0".into(),
                description: "地下水资源评估".into(),
            },
            aquifer: AquiferParams::default(),
            recharge: RechargeParams::default(),
        }
    }
}

impl PluginConfig for GroundwaterConfig {}
