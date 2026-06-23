//! Tool registration — Socioeconomic plugin
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};

pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "socioeconomic", "Socioeconomic analysis: population, GDP, land-use change, accessibility", PluginCategory::Process, [
        sync "socio_pop_density" => "Dasymetric population density from admin pop + landcover weights" ; serde_json::json!({"type":"object","properties":{"admin_pop":{"type":"number"},"landcover_weights":{"type":"array","items":{"type":"number"}},"cell_area_km2":{"type":"number","default":0.01}},"required":["admin_pop","landcover_weights"]}) => |args| -> ToolResult {
            let pop = args["admin_pop"].as_f64().unwrap_or(0.0);
            let cell = args["cell_area_km2"].as_f64().unwrap_or(0.01);
            let w: Vec<f64> = args["landcover_weights"].as_array().unwrap_or(&vec![]).iter().filter_map(|v| v.as_f64()).collect();
            let (densities, pops) = crate::population::pop_density_from_landcover(pop, &w, cell);
            Ok(serde_json::json!({"densities": densities, "population_per_cell": pops}))
        },
        sync "socio_nightlight_gdp" => "GDP estimation from nightlight values" ; serde_json::json!({"type":"object","properties":{"ntl_values":{"type":"array","items":{"type":"number"}},"calibration_factor":{"type":"number","default":0.5}},"required":["ntl_values"]}) => |args| -> ToolResult {
            let ntl: Vec<f64> = args["ntl_values"].as_array().unwrap_or(&vec![]).iter().filter_map(|v| v.as_f64()).collect();
            let cal = args["calibration_factor"].as_f64().unwrap_or(0.5);
            let gdp = crate::population::nightlight_to_gdp(&ntl, cal);
            Ok(serde_json::json!({"gdp_grid": gdp}))
        },
        sync "socio_wealth_index" => "Composite wealth index from NTL + building + road density" ; serde_json::json!({"type":"object","properties":{"ntl":{"type":"array","items":{"type":"number"}},"building_density":{"type":"array","items":{"type":"number"}},"road_density":{"type":"array","items":{"type":"number"}}},"required":["ntl","building_density","road_density"]}) => |args| -> ToolResult {
            let ntl: Vec<f64> = args["ntl"].as_array().unwrap_or(&vec![]).iter().filter_map(|v| v.as_f64()).collect();
            let bld: Vec<f64> = args["building_density"].as_array().unwrap_or(&vec![]).iter().filter_map(|v| v.as_f64()).collect();
            let road: Vec<f64> = args["road_density"].as_array().unwrap_or(&vec![]).iter().filter_map(|v| v.as_f64()).collect();
            let idx = crate::population::wealth_index(&ntl, &bld, &road);
            Ok(serde_json::json!({"wealth_index": idx}))
        },
        sync "socio_transition_matrix" => "Compute LULC transition probability matrix from two time points" ; serde_json::json!({"type":"object","properties":{"from_lulc":{"type":"array","items":{"type":"integer"}},"to_lulc":{"type":"array","items":{"type":"integer"}},"n_classes":{"type":"integer"}},"required":["from_lulc","to_lulc","n_classes"]}) => |args| -> ToolResult {
            let from: Vec<u8> = args["from_lulc"].as_array().unwrap_or(&vec![]).iter().filter_map(|v| v.as_u64().map(|x|x as u8)).collect();
            let to: Vec<u8> = args["to_lulc"].as_array().unwrap_or(&vec![]).iter().filter_map(|v| v.as_u64().map(|x|x as u8)).collect();
            let nc = args["n_classes"].as_u64().unwrap_or(3) as u8;
            let mat = crate::landuse_change::transition_probability(&from, &to, nc);
            Ok(serde_json::json!({"matrix": mat}))
        },
        sync "socio_ca_markov" => "CA-Markov LULC change simulation" ; serde_json::json!({"type":"object","properties":{"current_lulc":{"type":"array","items":{"type":"integer"}},"transition_matrix":{"type":"array","items":{"type":"array","items":{"type":"number"}}},"drivers":{"type":"array","items":{"type":"array","items":{"type":"number"}}},"iterations":{"type":"integer","default":10},"cols":{"type":"integer"}},"required":["current_lulc","transition_matrix","cols"]}) => |args| -> ToolResult {
            let lulc: Vec<u8> = args["current_lulc"].as_array().unwrap_or(&vec![]).iter().filter_map(|v| v.as_u64().map(|x|x as u8)).collect();
            let mat: Vec<Vec<f64>> = args["transition_matrix"].as_array().unwrap_or(&vec![]).iter().map(|row| row.as_array().unwrap_or(&vec![]).iter().filter_map(|v| v.as_f64()).collect()).collect();
            let drivers: Vec<Vec<f64>> = args["drivers"].as_array().unwrap_or(&vec![]).iter().map(|row| row.as_array().unwrap_or(&vec![]).iter().filter_map(|v| v.as_f64()).collect()).collect();
            let iter = args["iterations"].as_u64().unwrap_or(10) as usize;
            let cols = args["cols"].as_u64().unwrap_or(1) as usize;
            let result = crate::landuse_change::ca_markov_simulate(&lulc, &mat, &drivers, iter, 0.3, cols);
            serde_json::to_value(result).map_err(geo_core::errors::GeoError::Serde)
        },
        sync "socio_travel_time" => "Travel time from origin to all cells via Dijkstra" ; serde_json::json!({"type":"object","properties":{"origin_idx":{"type":"integer"},"cost_surface":{"type":"array","items":{"type":"number"}},"max_cost":{"type":"number","default":120.0},"cols":{"type":"integer"}},"required":["origin_idx","cost_surface","cols"]}) => |args| -> ToolResult {
            let origin = args["origin_idx"].as_u64().unwrap_or(0) as usize;
            let cost: Vec<f64> = args["cost_surface"].as_array().unwrap_or(&vec![]).iter().filter_map(|v| v.as_f64()).collect();
            let max_c = args["max_cost"].as_f64().unwrap_or(120.0);
            let cols = args["cols"].as_u64().unwrap_or(1) as usize;
            let tt = crate::accessibility::travel_time_to_city(origin, &cost, max_c, cols);
            Ok(serde_json::json!({"travel_time": tt}))
        },
        sync "socio_market_potential" => "Market potential (gravity model)" ; serde_json::json!({"type":"object","properties":{"population":{"type":"array","items":{"type":"number"}},"travel_time":{"type":"array","items":{"type":"number"}},"decay":{"type":"number","default":0.05}},"required":["population","travel_time"]}) => |args| -> ToolResult {
            let pop: Vec<f64> = args["population"].as_array().unwrap_or(&vec![]).iter().filter_map(|v| v.as_f64()).collect();
            let tt: Vec<f64> = args["travel_time"].as_array().unwrap_or(&vec![]).iter().filter_map(|v| v.as_f64()).collect();
            let decay = args["decay"].as_f64().unwrap_or(0.05);
            let potential = crate::accessibility::market_potential(&pop, &tt, decay);
            Ok(serde_json::json!({"market_potential": potential}))
        },
    ]);
}
