//! Tool registration — Urban plugin.
use geo_core::plugin::PluginCategory;
use geo_registry::PluginRegistry;
use geo_registry::registry::{ToolDef, ToolResult};
use crate::{UrbanConfig, UrbanPlugin};
fn mk() -> UrbanConfig { toml::from_str("[plugin]\nname=\"urban\"\nversion=\"0.1\"\ndescription=\"\"\n").unwrap() }
pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta { name: "urban".into(), version: "0.1.0".into(), description: "Urban planning: FAR, density, compliance".into(), category: PluginCategory::Process, healthy: true, extra: serde_json::json!({}) });
    registry.register_tool_sync("urban", ToolDef { name: "urban_far".into(), description: "Compute FAR from total floor area and site area".into(), input_schema: serde_json::json!({"type":"object","properties":{"total_floor_area_m2":{"type":"number"},"site_area_m2":{"type":"number"}},"required":["total_floor_area_m2","site_area_m2"]}) }, |args| -> ToolResult {
        let p = UrbanPlugin::new(mk());
        let tfa = args["total_floor_area_m2"].as_f64().unwrap_or(0.0);
        let site = args["site_area_m2"].as_f64().unwrap_or(0.0);
        let far = p.far(tfa, site); let den = p.building_density(tfa, site); let (fo,do_) = p.check_compliance(far, den);
        Ok(serde_json::json!({"far":far,"building_density":den,"far_compliant":fo,"density_compliant":do_}))
    });
}
