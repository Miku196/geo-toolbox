//! Tool registration — Geohazard plugin (async, Arc<Plugin> pattern).
use geo_core::plugin::{Plugin, ProcessPlugin};
use geo_registry::{register_async_tools, PluginRegistry};
pub fn register_tools(registry: &mut PluginRegistry) {
    let config_paths = [std::path::PathBuf::from("plugins/geo-plugin-geohazard/rules.toml"), std::path::PathBuf::from("../../plugins/geo-plugin-geohazard/rules.toml")];
    let plugin = config_paths.iter().find_map(|p| crate::GeohazardPlugin::load_from_file(p).ok()).unwrap_or_else(|| crate::GeohazardPlugin::new(Default::default()));
    registry.register(geo_core::plugin::PluginMeta {
        name: plugin.name().to_string(), version: plugin.version().to_string(), description: plugin.description().to_string(),
        category: plugin.category(), healthy: plugin.is_healthy(), extra: serde_json::json!({}),
    });
    let plugin_arc = std::sync::Arc::new(plugin);
    register_async_tools!(registry, "geohazard", [
        "geohazard_landslide" => "Compute 6-factor landslide susceptibility index" ; serde_json::json!({"type":"object","properties":{"slope_deg":{"type":"number"},"aspect_deg":{"type":"number"},"lithology_index":{"type":"number"},"rainfall_mm":{"type":"number"},"fault_distance_m":{"type":"number"},"ndvi":{"type":"number"}},"required":["slope_deg","lithology_index","rainfall_mm","fault_distance_m","ndvi"]}) => {
            let plugin = std::sync::Arc::clone(&plugin_arc);
            move |args| { let plugin = std::sync::Arc::clone(&plugin); Box::pin(async move { plugin.execute(serde_json::json!({"task":"landslide","slope_deg":args["slope_deg"].as_f64().unwrap_or(15.0),"aspect_deg":args["aspect_deg"].as_f64().unwrap_or(180.0),"lithology_index":args["lithology_index"].as_f64().unwrap_or(0.5),"rainfall_mm":args["rainfall_mm"].as_f64().unwrap_or(100.0),"fault_distance_m":args["fault_distance_m"].as_f64().unwrap_or(500.0),"ndvi":args["ndvi"].as_f64().unwrap_or(0.3),"aoi_name":args.get("aoi_name").and_then(|v|v.as_str()).unwrap_or("default")})).await }) }
        },
        "geohazard_debris_flow" => "Compute debris flow hazard" ; serde_json::json!({"type":"object","properties":{"channel_gradient_deg":{"type":"number"},"material_volume_per_km":{"type":"number"},"rainfall_24h_mm":{"type":"number"}},"required":["channel_gradient_deg","material_volume_per_km","rainfall_24h_mm"]}) => {
            let plugin = std::sync::Arc::clone(&plugin_arc);
            move |args| { let plugin = std::sync::Arc::clone(&plugin); Box::pin(async move { plugin.execute(serde_json::json!({"task":"debris_flow","channel_gradient_deg":args["channel_gradient_deg"].as_f64().unwrap_or(10.0),"material_volume_per_km":args["material_volume_per_km"].as_f64().unwrap_or(500.0),"rainfall_24h_mm":args["rainfall_24h_mm"].as_f64().unwrap_or(30.0)})).await }) }
        },
        "geohazard_risk_map" => "Combined landslide + debris flow risk assessment" ; serde_json::json!({"type":"object","properties":{"aoi_name":{"type":"string"},"slope_deg":{"type":"number"},"aspect_deg":{"type":"number","default":180},"lithology_index":{"type":"number"},"rainfall_mm":{"type":"number"},"fault_distance_m":{"type":"number"},"ndvi":{"type":"number"},"channel_gradient_deg":{"type":"number"},"material_volume_per_km":{"type":"number"},"rainfall_24h_mm":{"type":"number"}},"required":["aoi_name","slope_deg","lithology_index","rainfall_mm","fault_distance_m","ndvi"]}) => {
            let plugin = std::sync::Arc::clone(&plugin_arc);
            move |args| { let plugin = std::sync::Arc::clone(&plugin); Box::pin(async move { plugin.execute(args).await }) }
        },
    ]);
}
