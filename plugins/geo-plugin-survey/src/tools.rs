//! Tool registration — Survey plugin.
use geo_core::plugin::PluginCategory;
use geo_registry::PluginRegistry;
use geo_registry::registry::{ToolDef, ToolResult};
use crate::{SurveyConfig, SurveyPlugin};
pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta { name: "survey".into(), version: "0.1.0".into(), description: "Surveying: earthwork volume".into(), category: PluginCategory::Process, healthy: true, extra: serde_json::json!({}) });
    registry.register_tool_sync("survey", ToolDef { name: "survey_earthwork".into(), description: "Calculate earthwork volume from [(area_ha, height_diff_m), ...]".into(), input_schema: serde_json::json!({"type":"object","properties":{"polygons":{"type":"array"}},"required":["polygons"]}) }, |args| -> ToolResult {
        let polys: Vec<(f64,f64)> = args["polygons"].as_array().unwrap_or(&vec![]).iter().filter_map(|p|{let a=p.as_array()?;Some((a.get(0)?.as_f64()?,a.get(1)?.as_f64()?))}).collect();
        let config: SurveyConfig = toml::from_str("[plugin]\nname=\"survey\"\nversion=\"0.1\"\ndescription=\"\"\n").map_err(|e| geo_core::GeoError::Validation(e.to_string()))?;
        Ok(serde_json::json!({"volume_m3": SurveyPlugin::new(config).calculate_earthwork(&polys)}))
    });
}
