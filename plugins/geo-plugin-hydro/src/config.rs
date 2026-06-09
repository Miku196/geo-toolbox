use serde::Deserialize;
#[derive(Debug, Clone, Deserialize)]
pub struct HydroConfig { pub plugin: PluginMeta, #[serde(default)] pub flood: FloodParams }
#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta { pub name: String, pub version: String, pub description: String }
#[derive(Debug, Clone, Deserialize)]
pub struct FloodParams {
    #[serde(default = "default_return_period")] pub return_period_years: u32,
    #[serde(default)] pub safety_factor: f64,
}
fn default_return_period() -> u32 { 100 }
impl Default for FloodParams { fn default() -> Self { Self { return_period_years: 100, safety_factor: 1.2 } } }
