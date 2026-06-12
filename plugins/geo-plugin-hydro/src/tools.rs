//! Tool registration — Hydro plugin.
use geo_core::plugin::PluginCategory;
use geo_registry::PluginRegistry;
use geo_registry::registry::{ToolDef, ToolResult};
use crate::{HydroConfig, HydroPlugin};
fn mk() -> HydroConfig { toml::from_str("[plugin]\nname=\"hydro\"\nversion=\"0.1\"\ndescription=\"\"\n").unwrap() }
pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta { name: "hydro".into(), version: "0.1.0".into(), description: "Hydrology: inundation, runoff".into(), category: PluginCategory::Process, healthy: true, extra: serde_json::json!({}) });
    registry.register_tool_sync("hydro", ToolDef { name: "hydro_inundation".into(), description: "Estimate inundation area from catchment + rainfall".into(), input_schema: serde_json::json!({"type":"object","properties":{"catchment_area_ha":{"type":"number"},"rainfall_mm":{"type":"number"}},"required":["catchment_area_ha","rainfall_mm"]}) }, |args| -> ToolResult {
        Ok(serde_json::json!({"inundation_area_ha": HydroPlugin::new(mk()).estimate_inundation_area(args["catchment_area_ha"].as_f64().unwrap_or(0.0), args["rainfall_mm"].as_f64().unwrap_or(0.0))}))
    });
    registry.register_tool_sync("hydro", ToolDef { name: "hydro_runoff".into(), description: "Compute runoff coefficient from impervious ratio".into(), input_schema: serde_json::json!({"type":"object","properties":{"impervious_ratio":{"type":"number"}},"required":["impervious_ratio"]}) }, |args| -> ToolResult {
        Ok(serde_json::json!({"runoff_coefficient": HydroPlugin::new(mk()).runoff_coefficient(args["impervious_ratio"].as_f64().unwrap_or(0.0))}))
    });
}
