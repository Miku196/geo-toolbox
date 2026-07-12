use geo_registry::register_plugin;
use geo_registry::registry::ToolResult;
use geo_registry::PluginRegistry;

pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "ecosystem-services", "Ecosystem services — water yield, sediment, habitat, carbon", PluginCategory::Process, [
        sync "eco_water_yield" => "Water yield (Budyko curve)" ; serde_json::json!({"type":"object","properties":{"precip_mm":{"type":"number"},"pet_mm":{"type":"number"},"omega":{"type":"number","default":2.6}},"required":["precip_mm","pet_mm"]}) => |args| -> ToolResult {
            let p=args["precip_mm"].as_f64().unwrap_or(800.0);let pet=args["pet_mm"].as_f64().unwrap_or(1200.0);let w=args["omega"].as_f64().unwrap_or(2.6);
            serde_json::to_value(crate::EcosystemPlugin.water_yield(p,pet,w)).map_err(geo_core::GeoError::Serde)
        },
        sync "eco_sediment" => "Sediment retention (SDR model)" ; serde_json::json!({"type":"object","properties":{"soil_loss_t_ha_yr":{"type":"number"},"upstream_area_ha":{"type":"number"},"land_cover_roughness":{"type":"number","default":0.6}},"required":["soil_loss_t_ha_yr","upstream_area_ha"]}) => |args| -> ToolResult {
            let sl=args["soil_loss_t_ha_yr"].as_f64().unwrap_or(10.0);let ua=args["upstream_area_ha"].as_f64().unwrap_or(100.0);let lcr=args["land_cover_roughness"].as_f64().unwrap_or(0.6);
            serde_json::to_value(crate::EcosystemPlugin.sediment(sl,ua,lcr)).map_err(geo_core::GeoError::Serde)
        },
        sync "eco_habitat" => "Habitat quality (InVEST-style)" ; serde_json::json!({"type":"object","properties":{"habitat_suitability":{"type":"number"},"threat_distances":{"type":"array","items":{"type":"object","properties":{"distance":{"type":"number"},"max_distance":{"type":"number"}},"required":["distance","max_distance"]}},"threat_weights":{"type":"array","items":{"type":"number"}}},"required":["habitat_suitability","threat_distances","threat_weights"]}) => |args| -> ToolResult {
            let hs=args["habitat_suitability"].as_f64().unwrap_or(0.8);
            let td:Vec<(f64,f64)>=args["threat_distances"].as_array().map(|a|a.iter().map(|v|(v["distance"].as_f64().unwrap_or(500.0),v["max_distance"].as_f64().unwrap_or(1000.0))).collect()).unwrap_or_default();
            let tw:Vec<f64>=args["threat_weights"].as_array().map(|a|a.iter().filter_map(|v|v.as_f64()).collect()).unwrap_or_default();
            serde_json::to_value(crate::EcosystemPlugin.habitat(hs,&td,&tw)).map_err(geo_core::GeoError::Serde)
        },
        sync "eco_carbon" => "Carbon storage (4-pool IPCC)" ; serde_json::json!({"type":"object","properties":{"agb_tc_ha":{"type":"number"},"bgb_tc_ha":{"type":"number"},"soc_tc_ha":{"type":"number"},"dom_tc_ha":{"type":"number","default":5.0}},"required":["agb_tc_ha","bgb_tc_ha","soc_tc_ha"]}) => |args| -> ToolResult {
            let a=args["agb_tc_ha"].as_f64().unwrap_or(80.0);let b=args["bgb_tc_ha"].as_f64().unwrap_or(20.0);let s=args["soc_tc_ha"].as_f64().unwrap_or(60.0);let d=args["dom_tc_ha"].as_f64().unwrap_or(5.0);
            serde_json::to_value(crate::EcosystemPlugin.carbon(a,b,s,d)).map_err(geo_core::GeoError::Serde)
        },
        sync "eco_nutrient" => "Nutrient retention" ; serde_json::json!({"type":"object","properties":{"n_load_kg_ha_yr":{"type":"number"},"buffer_width_m":{"type":"number"},"slope_pct":{"type":"number"}},"required":["n_load_kg_ha_yr","buffer_width_m","slope_pct"]}) => |args| -> ToolResult {
            let n=args["n_load_kg_ha_yr"].as_f64().unwrap_or(50.0);let bw=args["buffer_width_m"].as_f64().unwrap_or(30.0);let sp=args["slope_pct"].as_f64().unwrap_or(5.0);
            serde_json::to_value(crate::EcosystemPlugin.nutrient(n,bw,sp)).map_err(geo_core::GeoError::Serde)
        }
    ]);
}
