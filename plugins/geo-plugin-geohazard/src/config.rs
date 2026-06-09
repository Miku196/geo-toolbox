use serde::Deserialize;
#[derive(Debug, Clone, Deserialize)]
pub struct GeohazardConfig { pub plugin: PluginMeta, #[serde(default)] pub landslide: LandslideParams }
#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta { pub name: String, pub version: String, pub description: String }
#[derive(Debug, Clone, Deserialize)]
pub struct LandslideParams {
    #[serde(default = "default_slope_weight")] pub slope_weight: f64,
    #[serde(default = "default_lithology_weight")] pub lithology_weight: f64,
    #[serde(default = "default_rainfall_weight")] pub rainfall_weight: f64,
}
fn default_slope_weight() -> f64 { 0.4 }
fn default_lithology_weight() -> f64 { 0.35 }
fn default_rainfall_weight() -> f64 { 0.25 }
impl Default for LandslideParams { fn default() -> Self { Self { slope_weight: 0.4, lithology_weight: 0.35, rainfall_weight: 0.25 } } }
