//! Tool registration — Atmosphere plugin
use geo_core::plugin::PluginCategory;
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};

use crate::aod_pm25::aod_pm25_pipeline;
use crate::boundary_layer::{boundary_layer_assessment, StabilityClass};
use crate::dispersion::dispersion_assessment;

pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "atmosphere", "Atmospheric science: boundary layer, dispersion, AOD→PM2.5", PluginCategory::Process, [
        sync "atmo_boundary_layer" => "Boundary layer parameters: ABL height, heat fluxes, Monin-Obukhov length, stability" ; serde_json::json!({"type":"object","properties":{"temp_profile":{"type":"array","items":{"type":"number"},"description":"[T_surface, T_2m, T_10m, T_50m] °C"},"wind_profile":{"type":"array","items":{"type":"number"},"description":"[u_2m, u_10m, u_50m] m/s"},"roughness_m":{"type":"number","default":0.1},"coriolis_param":{"type":"number","default":1.0e-4}},"required":["temp_profile","wind_profile"]}) => |args| -> ToolResult {
            let tp: Vec<f64> = args["temp_profile"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect();
            let wp: Vec<f64> = args["wind_profile"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect();
            let z0 = args["roughness_m"].as_f64().unwrap_or(0.1);
            let f = args["coriolis_param"].as_f64().unwrap_or(1.0e-4);
            let r = boundary_layer_assessment(&tp, &wp, z0, f);
            serde_json::to_value(r).map_err(geo_core::errors::GeoError::Serde)
        },

        sync "atmo_dispersion" => "Gaussian plume dispersion: centerline concentration, max ground concentration" ; serde_json::json!({"type":"object","properties":{"emission_rate_g_s":{"type":"number"},"wind_speed_m_s":{"type":"number"},"stability":{"type":"string","default":"D","description":"A-F Pasquill-Gifford class"},"source_height_m":{"type":"number","default":10.0}},"required":["emission_rate_g_s","wind_speed_m_s"]}) => |args| -> ToolResult {
            let rate = args["emission_rate_g_s"].as_f64().unwrap_or(0.0);
            let wind = args["wind_speed_m_s"].as_f64().unwrap_or(3.0);
            let s = args["stability"].as_str().unwrap_or("D");
            let stab = StabilityClass::from_char(s.chars().next().unwrap_or('D')).unwrap_or(StabilityClass::D);
            let h = args["source_height_m"].as_f64().unwrap_or(10.0);
            let r = dispersion_assessment(rate, wind, stab, h);
            serde_json::to_value(r).map_err(geo_core::errors::GeoError::Serde)
        },

        sync "atmo_aod_pm25" => "AOD550 → PM2.5 concentration → AQI air quality index" ; serde_json::json!({"type":"object","properties":{"aod_values":{"type":"array","items":{"type":"number"}},"aod550_pm25_ratio":{"type":"number","default":0.55},"rh_correction":{"type":"number","default":0.85},"season":{"type":"string","default":"annual","enum":["winter","spring","summer","autumn","annual"]}},"required":["aod_values"]}) => |args| -> ToolResult {
            let aods: Vec<f64> = args["aod_values"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect();
            let ratio = args["aod550_pm25_ratio"].as_f64().unwrap_or(0.55);
            let rh = args["rh_correction"].as_f64().unwrap_or(0.85);
            let season = args["season"].as_str().unwrap_or("annual");
            let r = aod_pm25_pipeline(&aods, ratio, rh, season);
            serde_json::to_value(r).map_err(geo_core::errors::GeoError::Serde)
        },

        sync "atmo_concentration_point" => "Gaussian plume point concentration at (x,y,z)" ; serde_json::json!({"type":"object","properties":{"emission_rate_g_s":{"type":"number"},"wind_speed_m_s":{"type":"number"},"stability":{"type":"string","default":"D"},"source_height_m":{"type":"number","default":10.0},"x_m":{"type":"number","default":500.0},"y_m":{"type":"number","default":0.0},"z_m":{"type":"number","default":0.0}},"required":["emission_rate_g_s","wind_speed_m_s"]}) => |args| -> ToolResult {
            let rate = args["emission_rate_g_s"].as_f64().unwrap_or(0.0);
            let wind = args["wind_speed_m_s"].as_f64().unwrap_or(3.0);
            let s = args["stability"].as_str().unwrap_or("D");
            let stab = StabilityClass::from_char(s.chars().next().unwrap_or('D')).unwrap_or(StabilityClass::D);
            let h = args["source_height_m"].as_f64().unwrap_or(10.0);
            let x = args["x_m"].as_f64().unwrap_or(500.0);
            let y = args["y_m"].as_f64().unwrap_or(0.0);
            let z = args["z_m"].as_f64().unwrap_or(0.0);
            let plume = crate::dispersion::GaussianPlume::new(rate, wind, stab, h);
            let c = plume.concentration(x, y, z);
            Ok(serde_json::json!({
                "concentration_ug_m3": (c*100.0).round()/100.0,
                "x_m": x, "y_m": y, "z_m": z,
                "stability": stab.as_str(),
                "emission_rate_g_s": rate,
                "wind_speed_m_s": wind
            }))
        },
    ]);
}
