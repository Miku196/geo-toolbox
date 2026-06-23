//! Tool registration — Cryosphere plugin
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};

pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "cryosphere", "Cryosphere: snowmelt, glacier mass balance, permafrost", PluginCategory::Process, [
        sync "cryo_swe_simulate" => "Snow Water Equivalent simulation from precipitation + temperature" ; serde_json::json!({"type":"object","properties":{"precip_mm":{"type":"array","items":{"type":"number"}},"temp_c":{"type":"array","items":{"type":"number"}},"dd_factor":{"type":"number","default":3.0},"rain_snow_c":{"type":"number","default":0.0}},"required":["precip_mm","temp_c"]}) => |args| -> ToolResult {
            let p: Vec<f64> = args["precip_mm"].as_array().unwrap_or(&vec![]).iter().filter_map(|v| v.as_f64()).collect();
            let t: Vec<f64> = args["temp_c"].as_array().unwrap_or(&vec![]).iter().filter_map(|v| v.as_f64()).collect();
            let ddf = args["dd_factor"].as_f64().unwrap_or(3.0);
            let rst = args["rain_snow_c"].as_f64().unwrap_or(0.0);
            let r = crate::swe::simulate_swe(&p, &t, ddf, rst);
            serde_json::to_value(r).map_err(geo_core::errors::GeoError::Serde)
        },
        sync "cryo_glacier_balance" => "Glacier mass balance: accumulation, ablation, net" ; serde_json::json!({"type":"object","properties":{"accumulation_mwe":{"type":"number"},"ablation_mwe":{"type":"number"},"area_km2":{"type":"number"}},"required":["accumulation_mwe","ablation_mwe","area_km2"]}) => |args| -> ToolResult {
            let a = args["accumulation_mwe"].as_f64().unwrap_or(0.0);
            let b = args["ablation_mwe"].as_f64().unwrap_or(0.0);
            let area = args["area_km2"].as_f64().unwrap_or(1.0);
            let r = crate::glacier::mass_balance(a, b, area);
            serde_json::to_value(r).map_err(geo_core::errors::GeoError::Serde)
        },
        sync "cryo_permafrost_alt" => "Active layer thickness from Stefan solution" ; serde_json::json!({"type":"object","properties":{"thawing_degree_days":{"type":"number"},"thermal_conductivity":{"type":"number","default":1.5},"ice_content":{"type":"number","default":0.3}},"required":["thawing_degree_days"]}) => |args| -> ToolResult {
            let tdd = args["thawing_degree_days"].as_f64().unwrap_or(0.0);
            let tc = args["thermal_conductivity"].as_f64().unwrap_or(1.5);
            let ic = args["ice_content"].as_f64().unwrap_or(0.3);
            let alt = crate::permafrost::active_layer_thickness_stefan(tdd, tc, ic);
            Ok(serde_json::json!({"active_layer_thickness_cm": (alt*100.0).round()/100.0}))
        },
        sync "cryo_freeze_thaw_index" => "Freezing/Thawing degree days from daily temps" ; serde_json::json!({"type":"object","properties":{"daily_temp_c":{"type":"array","items":{"type":"number"}}},"required":["daily_temp_c"]}) => |args| -> ToolResult {
            let t: Vec<f64> = args["daily_temp_c"].as_array().unwrap_or(&vec![]).iter().filter_map(|v| v.as_f64()).collect();
            let fi = crate::permafrost::freeze_thaw_index(&t);
            Ok(serde_json::json!({"freezing_index": (fi.freezing_index*100.0).round()/100.0, "thawing_index": (fi.thawing_index*100.0).round()/100.0}))
        },
    ]);
}
