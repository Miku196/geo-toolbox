//! Tool registration — Agri plugin.
use geo_core::plugin::PluginCategory;
use geo_registry::PluginRegistry;
use geo_registry::registry::{ToolDef, ToolResult};
use crate::{AgriConfig, AgriPlugin};
fn mk() -> AgriConfig { toml::from_str("[plugin]\nname=\"agri\"\nversion=\"0.1\"\ndescription=\"\"\n").unwrap() }
pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta { name: "agri".into(), version: "0.1.0".into(), description: "Agriculture: yield estimation, soil rating".into(), category: PluginCategory::Process, healthy: true, extra: serde_json::json!({}) });
    registry.register_tool_sync("agri", ToolDef { name: "agri_yield".into(), description: "Estimate crop yield from area, NDVI, baseline".into(), input_schema: serde_json::json!({"type":"object","properties":{"area_ha":{"type":"number"},"ndvi_mean":{"type":"number"},"baseline_yield_kg_ha":{"type":"number"}},"required":["area_ha","ndvi_mean","baseline_yield_kg_ha"]}) }, |args| -> ToolResult {
        Ok(serde_json::json!({"estimated_yield_kg": AgriPlugin::new(mk()).estimate_yield(args["area_ha"].as_f64().unwrap_or(0.0), args["ndvi_mean"].as_f64().unwrap_or(0.0), args["baseline_yield_kg_ha"].as_f64().unwrap_or(5000.0))}))
    });
    registry.register_tool_sync("agri", ToolDef { name: "agri_soil".into(), description: "Rate soil quality from organic matter and pH".into(), input_schema: serde_json::json!({"type":"object","properties":{"organic_matter_pct":{"type":"number"},"ph":{"type":"number"}},"required":["organic_matter_pct","ph"]}) }, |args| -> ToolResult {
        Ok(serde_json::json!({"rating": AgriPlugin::new(mk()).soil_rating(args["organic_matter_pct"].as_f64().unwrap_or(0.0), args["ph"].as_f64().unwrap_or(7.0))}))
    });
}
