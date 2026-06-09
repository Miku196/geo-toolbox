use serde::Deserialize;
#[derive(Debug, Clone, Deserialize)]
pub struct AgriConfig { pub plugin: PluginMeta, #[serde(default)] pub yield_params: YieldParams }
#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta { pub name: String, pub version: String, pub description: String }
#[derive(Debug, Clone, Deserialize)]
pub struct YieldParams {
    #[serde(default = "default_crop_coefficient")] pub crop_coefficient: f64,
    #[serde(default = "default_ndvi_weight")] pub ndvi_weight: f64,
}
fn default_crop_coefficient() -> f64 { 0.8 }
fn default_ndvi_weight() -> f64 { 1.2 }
impl Default for YieldParams { fn default() -> Self { Self { crop_coefficient: 0.8, ndvi_weight: 1.2 } } }
