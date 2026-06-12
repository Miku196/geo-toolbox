//! Tool registration — Geohazard plugin.
use geo_core::plugin::PluginCategory;
use geo_registry::PluginRegistry;
use geo_registry::registry::{ToolDef, ToolResult};
use crate::{GeohazardConfig, GeohazardPlugin};
fn mk() -> GeohazardConfig { toml::from_str("[plugin]\nname=\"geohazard\"\nversion=\"0.1\"\ndescription=\"\"\n").unwrap() }
pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta { name: "geohazard".into(), version: "0.1.0".into(), description: "Landslide susceptibility + risk level".into(), category: PluginCategory::Process, healthy: true, extra: serde_json::json!({}) });
    registry.register_tool_sync("geohazard", ToolDef { name: "geohazard_landslide".into(), description: "Compute landslide susceptibility from normalized factors [0,1]".into(), input_schema: serde_json::json!({"type":"object","properties":{"slope_norm":{"type":"number"},"lithology_norm":{"type":"number"},"rainfall_norm":{"type":"number"}},"required":["slope_norm","lithology_norm","rainfall_norm"]}) }, |args| -> ToolResult {
        let p = GeohazardPlugin::new(mk());
        let s = p.landslide_susceptibility(args["slope_norm"].as_f64().unwrap_or(0.0), args["lithology_norm"].as_f64().unwrap_or(0.0), args["rainfall_norm"].as_f64().unwrap_or(0.0));
        Ok(serde_json::json!({"susceptibility":s,"risk_level":p.risk_level(s)}))
    });
}
