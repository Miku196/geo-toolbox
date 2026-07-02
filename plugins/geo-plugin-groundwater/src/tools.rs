use geo_registry::register_plugin;
use geo_registry::registry::ToolResult;
use geo_registry::PluginRegistry;

pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "groundwater", "Groundwater resources — pumping test, recharge, trend",
    PluginCategory::Process, [
        sync "groundwater_pumping_test" => "Analyze pumping test (Cooper-Jacob method)"
            ; serde_json::json!({"type":"object","properties":{"time_min":{"type":"array","items":{"type":"number"}},"drawdown_m":{"type":"array","items":{"type":"number"}},"pumping_rate_m3_day":{"type":"number"},"aquifer_thickness_m":{"type":"number"},"r_m":{"type":"number"}},"required":["time_min","drawdown_m","pumping_rate_m3_day","r_m"]})
            => |args| -> ToolResult {
                let time: Vec<f64> = args["time_min"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
                let dd: Vec<f64> = args["drawdown_m"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
                let q = args["pumping_rate_m3_day"].as_f64().unwrap_or(0.0);
                let b = args["aquifer_thickness_m"].as_f64();
                let r = args["r_m"].as_f64().unwrap_or(10.0);
                let plugin = crate::GroundwaterPlugin::new(Default::default());
                let result = plugin.analyze_pumping_test(&time, &dd, q, b, r)?;
                serde_json::to_value(result).map_err(geo_core::GeoError::Serde)
            },
        sync "groundwater_theis_drawdown" => "Compute Theis drawdown at a given distance and time"
            ; serde_json::json!({"type":"object","properties":{"t_day":{"type":"number"},"r_m":{"type":"number"},"transmissivity_m2_day":{"type":"number"},"storativity":{"type":"number"},"pumping_rate_m3_day":{"type":"number"}},"required":["t_day","r_m","transmissivity_m2_day","storativity","pumping_rate_m3_day"]})
            => |args| -> ToolResult {
                let t = args["t_day"].as_f64().unwrap_or(1.0);
                let r = args["r_m"].as_f64().unwrap_or(10.0);
                let tr = args["transmissivity_m2_day"].as_f64().unwrap_or(100.0);
                let s = args["storativity"].as_f64().unwrap_or(0.001);
                let q = args["pumping_rate_m3_day"].as_f64().unwrap_or(100.0);
                let plugin = crate::GroundwaterPlugin::new(Default::default());
                let dd = plugin.theis_drawdown(t, r, tr, s, q);
                Ok(serde_json::json!({"drawdown_m": dd}))
            },
        sync "groundwater_theis_profile" => "Compute Theis drawdown at multiple distances"
            ; serde_json::json!({"type":"object","properties":{"t_day":{"type":"number"},"distances_m":{"type":"array","items":{"type":"number"}},"transmissivity_m2_day":{"type":"number"},"storativity":{"type":"number"},"pumping_rate_m3_day":{"type":"number"}},"required":["t_day","distances_m","transmissivity_m2_day","storativity","pumping_rate_m3_day"]})
            => |args| -> ToolResult {
                let t = args["t_day"].as_f64().unwrap_or(1.0);
                let dists: Vec<f64> = args["distances_m"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
                let tr = args["transmissivity_m2_day"].as_f64().unwrap_or(100.0);
                let s = args["storativity"].as_f64().unwrap_or(0.001);
                let q = args["pumping_rate_m3_day"].as_f64().unwrap_or(100.0);
                let profile = crate::groundwater::theis_drawdown_profile(t, &dists, tr, s, q);
                serde_json::to_value(profile).map_err(geo_core::GeoError::Serde)
            },
        sync "groundwater_recharge" => "Estimate recharge via water balance method"
            ; serde_json::json!({"type":"object","properties":{"precipitation_mm":{"type":"number"},"evapotranspiration_mm":{"type":"number"},"runoff_coefficient":{"type":"number"},"area_km2":{"type":"number"}},"required":["precipitation_mm","evapotranspiration_mm","runoff_coefficient"]})
            => |args| -> ToolResult {
                let p = args["precipitation_mm"].as_f64().unwrap_or(0.0);
                let et = args["evapotranspiration_mm"].as_f64().unwrap_or(0.0);
                let rc = args["runoff_coefficient"].as_f64().unwrap_or(0.1);
                let area = args["area_km2"].as_f64();
                let plugin = crate::GroundwaterPlugin::new(Default::default());
                let result = plugin.recharge(p, et, rc, area);
                serde_json::to_value(result).map_err(geo_core::GeoError::Serde)
            },
        sync "groundwater_trend" => "Analyze water table trend from time series"
            ; serde_json::json!({"type":"object","properties":{"years":{"type":"array","items":{"type":"number"}},"water_level_m":{"type":"array","items":{"type":"number"}}},"required":["years","water_level_m"]})
            => |args| -> ToolResult {
                let years: Vec<f64> = args["years"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
                let levels: Vec<f64> = args["water_level_m"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
                let plugin = crate::GroundwaterPlugin::new(Default::default());
                let result = plugin.water_table_trend(&years, &levels)?;
                serde_json::to_value(result).map_err(geo_core::GeoError::Serde)
            },
        sync "groundwater_chloride_recharge" => "Estimate recharge using chloride mass balance"
            ; serde_json::json!({"type":"object","properties":{"precipitation_mm":{"type":"number"},"cl_precip_mg_l":{"type":"number"},"cl_gw_mg_l":{"type":"number"}},"required":["precipitation_mm","cl_precip_mg_l","cl_gw_mg_l"]})
            => |args| -> ToolResult {
                let p = args["precipitation_mm"].as_f64().unwrap_or(0.0);
                let cl_p = args["cl_precip_mg_l"].as_f64().unwrap_or(1.0);
                let cl_gw = args["cl_gw_mg_l"].as_f64().unwrap_or(1.0);
                let plugin = crate::GroundwaterPlugin::new(Default::default());
                let r = plugin.chloride_recharge(p, cl_p, cl_gw);
                Ok(serde_json::json!({"recharge_mm": r}))
            },
        sync "groundwater_specific_capacity" => "Estimate transmissivity from specific capacity"
            ; serde_json::json!({"type":"object","properties":{"specific_capacity_m2_day":{"type":"number"},"factor":{"type":"number","default":1.5}},"required":["specific_capacity_m2_day"]})
            => |args| -> ToolResult {
                let sc = args["specific_capacity_m2_day"].as_f64().unwrap_or(0.0);
                let factor = args["factor"].as_f64().unwrap_or(1.5);
                let t = crate::groundwater::specific_capacity_to_t(sc, factor);
                Ok(serde_json::json!({"transmissivity_m2_day": t}))
            }
    ]);
}
