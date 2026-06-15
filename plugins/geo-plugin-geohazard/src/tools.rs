//! Tool registration — Geohazard plugin.
use crate::config::GeohazardConfig;
use crate::geohazard::GeohazardPlugin;
use geo_core::plugin::PluginCategory;
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};

fn default_plugin() -> GeohazardPlugin {
    GeohazardPlugin::new(GeohazardConfig::default())
}

pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "geohazard", "Geohazard: FS safety factor, Newmark displacement", PluginCategory::Process, [
        sync "geohazard_fs" => "Factor of Safety (infinite slope model)" ; serde_json::json!({"type":"object","properties":{"slope_deg":{"type":"number"},"cohesion_kpa":{"type":"number"},"friction_deg":{"type":"number"},"soil_density":{"type":"number"},"water_depth_m":{"type":"number","default":0}},"required":["slope_deg","cohesion_kpa","friction_deg","soil_density"]}) => |args| -> ToolResult {
            let p = default_plugin();
            let fs = p.factor_of_safety(args["slope_deg"].as_f64().unwrap_or(0.0), args["cohesion_kpa"].as_f64().unwrap_or(0.0), args["friction_deg"].as_f64().unwrap_or(0.0), args["soil_density"].as_f64().unwrap_or(2000.0), args["water_depth_m"].as_f64().unwrap_or(0.0));
            Ok(serde_json::json!({"fs": fs, "stable": fs >= 1.0}))
        },
        sync "geohazard_newmark" => "Newmark displacement (Jibson 2007)" ; serde_json::json!({"type":"object","properties":{"pga_g":{"type":"number"},"ky_g":{"type":"number"}},"required":["pga_g","ky_g"]}) => |args| -> ToolResult {
            let p = default_plugin();
            let disp = p.newmark_displacement(args["pga_g"].as_f64().unwrap_or(0.0), args["ky_g"].as_f64().unwrap_or(0.0));
            Ok(serde_json::json!({"displacement_cm": disp, "hazard_level": if disp < 1.0 { "low" } else if disp < 5.0 { "moderate" } else if disp < 15.0 { "high" } else { "very_high" }}))
        },
    ]);
}
