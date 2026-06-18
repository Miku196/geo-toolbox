//! Tool registration — Geohazard plugin.
use crate::config::GeohazardConfig;
use crate::geohazard::GeohazardPlugin;
use crate::rainfall_threshold::{IdCurve, RainfallClass};
use geo_core::plugin::PluginCategory;
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};

fn default_plugin() -> GeohazardPlugin {
    GeohazardPlugin::new(GeohazardConfig::default())
}

pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "geohazard", "Geohazard: FS safety factor, Newmark displacement", PluginCategory::Process, [
        sync "geohazard_fs" => "Factor of Safety (infinite slope model)" ; serde_json::json!({"type":"object","properties":{"slope_deg":{"type":"number"},"soil_depth_m":{"type":"number"},"cohesion_kpa":{"type":"number"},"friction_deg":{"type":"number"},"soil_density_kn_m3":{"type":"number","default":20},"water_table_ratio":{"type":"number","default":0}},"required":["slope_deg","soil_depth_m","cohesion_kpa","friction_deg"]}) => |args| -> ToolResult {
            let p = default_plugin();
            let fs = p.factor_of_safety(args["slope_deg"].as_f64().unwrap_or(0.0), args["soil_depth_m"].as_f64().unwrap_or(1.0), args["cohesion_kpa"].as_f64().unwrap_or(0.0), args["friction_deg"].as_f64().unwrap_or(0.0), args["soil_density_kn_m3"].as_f64().unwrap_or(20.0), args["water_table_ratio"].as_f64().unwrap_or(0.0));
            Ok(serde_json::json!({"fs": fs, "stable": fs >= 1.0}))
        },
        sync "geohazard_newmark" => "Newmark displacement (Jibson 2007)" ; serde_json::json!({"type":"object","properties":{"slope_deg":{"type":"number"},"factor_of_safety":{"type":"number"},"pga_g":{"type":"number"}},"required":["slope_deg","factor_of_safety","pga_g"]}) => |args| -> ToolResult {
            let p = default_plugin();
            let disp = p.newmark_displacement(args["slope_deg"].as_f64().unwrap_or(0.0), args["factor_of_safety"].as_f64().unwrap_or(1.0), args["pga_g"].as_f64().unwrap_or(0.0));
            Ok(serde_json::json!({"displacement_cm": disp, "hazard_level": if disp < 1.0 { "low" } else if disp < 5.0 { "moderate" } else if disp < 15.0 { "high" } else { "very_high" }}))
        },
        sync "geohazard_rainfall_threshold" => "Rainfall intensity-duration threshold" ; serde_json::json!({"type":"object","properties":{"alpha":{"type":"number"},"beta":{"type":"number"},"duration_hours":{"type":"number"}},"required":["alpha","beta","duration_hours"]}) => |args| -> ToolResult {
            let alpha = args["alpha"].as_f64().unwrap_or(10.0);
            let beta = args["beta"].as_f64().unwrap_or(0.5);
            let duration_hours = args["duration_hours"].as_f64().unwrap_or(1.0);
            let curve = IdCurve::new(alpha, beta);
            let intensity = curve.intensity(duration_hours);
            let class = RainfallClass::classify(intensity);
            Ok(serde_json::json!({"intensity_mmh": intensity, "class": class.to_string(), "hazard_weight": class.hazard_weight()}))
        },
        sync "geohazard_debris_flow_runout" => "Debris flow volume-runout empirical model" ; serde_json::json!({"type":"object","properties":{"watershed_area_km2":{"type":"number","default":1.0},"rainfall_24h_mm":{"type":"number","default":100.0},"elevation_drop_m":{"type":"number","default":100.0},"channel_gradient_deg":{"type":"number","default":20.0}},"required":["watershed_area_km2","rainfall_24h_mm","elevation_drop_m","channel_gradient_deg"]}) => |args| -> ToolResult {
            let p = default_plugin();
            let assessment = p.debris_flow_runout_assessment(
                args["watershed_area_km2"].as_f64().unwrap_or(1.0),
                args["rainfall_24h_mm"].as_f64().unwrap_or(100.0),
                args["elevation_drop_m"].as_f64().unwrap_or(100.0),
                args["channel_gradient_deg"].as_f64().unwrap_or(20.0),
            )?;
            Ok(serde_json::to_value(&assessment).map_err(|e| geo_core::errors::GeoError::Serde(e))?)
        },
    ]);
}
