use geo_core::plugin::PluginCategory;
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};

use crate::ash_dispersion::ash_dispersion_assessment;
use crate::hazard_zoning::hazard_zone_classification;
use crate::lava_flow::lava_flow_simulation;

pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "volcanology", "Volcanology: ash dispersion, lava flow path, hazard zoning", PluginCategory::Process, [
        sync "volc_ash_dispersion" => "Volcanic ash dispersion: centerline concentration, deposition, settling velocity" ; serde_json::json!({"type":"object","properties":{"emission_rate_kg_s":{"type":"number","description":"Ash emission rate"},"wind_speed_m_s":{"type":"number"},"plume_height_m":{"type":"number","default":5000.0},"particle_diameter_mm":{"type":"number","default":0.5},"particle_density_kgm3":{"type":"number","default":2500.0},"stability":{"type":"string","default":"D","description":"Pasquill-Gifford class A-F"},"n_points":{"type":"integer","default":20}},"required":["emission_rate_kg_s","wind_speed_m_s"]}) => |args| -> ToolResult {
            let rate = args["emission_rate_kg_s"].as_f64().unwrap_or(0.0);
            let wind = args["wind_speed_m_s"].as_f64().unwrap_or(10.0);
            let h = args["plume_height_m"].as_f64().unwrap_or(5000.0);
            let diam_mm = args["particle_diameter_mm"].as_f64().unwrap_or(0.5);
            let diam_m = diam_mm / 1000.0;
            let dens = args["particle_density_kgm3"].as_f64().unwrap_or(2500.0);
            let stab = args["stability"].as_str().unwrap_or("D");
            let n = args["n_points"].as_u64().unwrap_or(20) as usize;
            let r = ash_dispersion_assessment(rate, wind, h, diam_m, dens, stab, n);
            serde_json::to_value(r).map_err(geo_core::errors::GeoError::Serde)
        },

        sync "volc_lava_flow" => "Lava flow path simulation using Dijkstra-based least-cost path" ; serde_json::json!({"type":"object","properties":{"dem":{"type":"array","items":{"type":"number"},"description":"Digital elevation model"},"vent_row":{"type":"integer"},"vent_col":{"type":"integer"},"effusion_rate_m3s":{"type":"number","default":500.0},"viscosity_Pa_s":{"type":"number","default":5000.0},"rows":{"type":"integer"},"cols":{"type":"integer"}},"required":["dem","vent_row","vent_col","rows","cols"]}) => |args| -> ToolResult {
            let dem: Vec<f64> = args["dem"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect();
            let vr = args["vent_row"].as_u64().unwrap_or(0) as usize;
            let vc = args["vent_col"].as_u64().unwrap_or(0) as usize;
            let eff = args["effusion_rate_m3s"].as_f64().unwrap_or(500.0);
            let visc = args["viscosity_Pa_s"].as_f64().unwrap_or(5000.0);
            let rows = args["rows"].as_u64().unwrap_or(10) as usize;
            let cols = args["cols"].as_u64().unwrap_or(10) as usize;
            let r = lava_flow_simulation(&dem, vr, vc, eff, visc, rows, cols);
            serde_json::to_value(r).map_err(geo_core::errors::GeoError::Serde)
        },

        sync "volc_hazard_zone" => "Classify volcanic hazard level from ash, lava, and distance factors" ; serde_json::json!({"type":"object","properties":{"mode":{"type":"string","enum":["point","grid"],"default":"point"},"ash_thickness_mm":{"type":"number","default":0.0},"on_lava_path":{"type":"boolean","default":false},"distance_km":{"type":"number","default":10.0},"ash_grid":{"type":"array","items":{"type":"number"}},"lava_grid":{"type":"array","items":{"type":"integer"}},"dist_grid":{"type":"array","items":{"type":"number"}},"slope_grid":{"type":"array","items":{"type":"number"}},"source_row":{"type":"integer","default":0},"source_col":{"type":"integer","default":0}},"required":["mode"]}) => |args| -> ToolResult {
            let mode = args["mode"].as_str().unwrap_or("point");
            match mode {
                "point" => {
                    let ash = args["ash_thickness_mm"].as_f64().unwrap_or(0.0);
                    let lava = args["on_lava_path"].as_bool().unwrap_or(false);
                    let dist = args["distance_km"].as_f64().unwrap_or(10.0);
                    let level = hazard_zone_classification(ash, lava, dist);
                    Ok(serde_json::json!({
                        "hazard_level": level.as_str(),
                        "score": level.score()
                    }))
                },
                _ => {
                    let ash: Vec<f64> = args["ash_grid"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect();
                    let lava: Vec<u8> = args["lava_grid"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_u64().map(|x| x as u8)).collect();
                    let dist: Vec<f64> = args["dist_grid"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect();
                    let slope: Vec<f64> = args["slope_grid"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect();
                    let sr = args["source_row"].as_u64().unwrap_or(0) as usize;
                    let sc = args["source_col"].as_u64().unwrap_or(0) as usize;
                    if ash.is_empty() || lava.is_empty() {
                        return Err(geo_core::GeoError::invalid_input("ash_grid", "ash_grid and lava_grid required for grid mode"));
                    }
                    let r = crate::hazard_zoning::volcanic_hazard_zoning(&ash, &lava, &dist, &slope, ash.len(), sr, sc);
                    serde_json::to_value(r).map_err(geo_core::errors::GeoError::Serde)
                }
            }
        },
    ]);
}
