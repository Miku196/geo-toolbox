use serde::Deserialize;
#[derive(Debug, Clone, Deserialize)]
pub struct UrbanConfig { pub plugin: PluginMeta, #[serde(default)] pub density: DensityParams }
#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta { pub name: String, pub version: String, pub description: String }
#[derive(Debug, Clone, Deserialize)]
pub struct DensityParams { #[serde(default = "default_far_max")] pub far_max: f64 }
fn default_far_max() -> f64 { 3.5 }
impl Default for DensityParams { fn default() -> Self { Self { far_max: 3.5 } } }
