use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};

use crate::paleocoastline::paleocoastline_flooding;
use crate::proxies::proxy_temperature;
use crate::sea_level::sea_level_reconstruction;

pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "paleoclimate", "Paleoclimate: sea level reconstruction, paleocoastline, proxy temperature inversion", PluginCategory::Process, [
        sync "paleo_sea_level" => "Glacial-interglacial sea level reconstruction from eustatic curve" ; serde_json::json!({"type":"object","properties":{"ages_ka":{"type":"array","items":{"type":"number"},"description":"Ages in ka BP"},"eustatic_curve":{"type":"array","items":{"type":"array","items":{"type":"number"},"minItems":2,"maxItems":2},"description":"[(age_ka, sea_level_m), ...]"},"isostatic_frac":{"type":"number","default":0.3,"description":"Isostatic adjustment fraction (0-1)"}},"required":["ages_ka","eustatic_curve"]}) => |args| -> ToolResult {
            let ages: Vec<f64> = args["ages_ka"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect();
            let curve_arr = args["eustatic_curve"].as_array().map(|a| a.as_slice()).unwrap_or(&[]);
            let curve: Vec<(f64,f64)> = curve_arr.iter().filter_map(|c| {
                let a = c.as_array()?;
                Some((a.first()?.as_f64()?, a.get(1)?.as_f64()?))
            }).collect();
            let iso = args["isostatic_frac"].as_f64().unwrap_or(0.3);
            let r = sea_level_reconstruction(&ages, &curve, iso);
            serde_json::to_value(r).map_err(geo_core::errors::GeoError::Serde)
        },

        sync "paleo_coastline" => "Paleocoastline restoration from modern DEM and sea level offset" ; serde_json::json!({"type":"object","properties":{"elevation_m":{"type":"array","items":{"type":"number"}},"cols":{"type":"integer"},"sea_level_offset_m":{"type":"number","default":-125.0}},"required":["elevation_m","cols"]}) => |args| -> ToolResult {
            let elev: Vec<f64> = args["elevation_m"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect();
            let cols = args["cols"].as_u64().unwrap_or(10) as usize;
            let sl = args["sea_level_offset_m"].as_f64().unwrap_or(-125.0);
            let (mask, bathy) = paleocoastline_flooding(&elev, sl, cols);
            Ok(serde_json::json!({
                "coastline_mask": mask,
                "paleobathymetry": bathy,
                "n_land": mask.iter().filter(|&&v| v >= 1).count(),
                "n_water": mask.iter().filter(|&&v| v == 0).count()
            }))
        },

        sync "paleo_proxy_temperature" => "Invert paleoclimate proxies (d18O, CH4, pollen, alkenone) to temperature" ; serde_json::json!({"type":"object","properties":{"proxies":{"type":"array","items":{"type":"object","properties":{"type":{"type":"string","enum":["d18o","ch4","pollen","alkenone"]},"value":{"type":"number"}},"required":["type","value"]}},"d18o_slope":{"type":"number","default":-4.5},"d18o_intercept":{"type":"number","default":20.0},"ch4_baseline":{"type":"number","default":700.0},"ch4_gradient":{"type":"number","default":0.055}},"required":["proxies"]}) => |args| -> ToolResult {
            let proxies_arr = args["proxies"].as_array().map(|a| a.as_slice()).unwrap_or(&[]);
            let proxies: Vec<(&str, f64)> = proxies_arr.iter().filter_map(|p| {
                let obj = p.as_object()?;
                let t = obj.get("type")?.as_str()?;
                let v = obj.get("value")?.as_f64()?;
                Some((t, v))
            }).collect();
            let d18o_s = args["d18o_slope"].as_f64().unwrap_or(-4.5);
            let d18o_i = args["d18o_intercept"].as_f64().unwrap_or(20.0);
            let ch4_b = args["ch4_baseline"].as_f64().unwrap_or(700.0);
            let ch4_g = args["ch4_gradient"].as_f64().unwrap_or(0.055);
            let temp = proxy_temperature(&proxies, d18o_s, d18o_i, ch4_b, ch4_g);
            Ok(serde_json::json!({
                "temperature_c": if temp.is_nan() { serde_json::Value::Null } else { serde_json::Value::Number(serde_json::Number::from_f64((temp*100.0).round()/100.0).unwrap()) },
                "n_proxies": proxies.len()
            }))
        },
    ]);
}
