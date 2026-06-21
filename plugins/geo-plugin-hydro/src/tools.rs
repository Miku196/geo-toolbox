//! Tool registration — Hydrology plugin.
use crate::HydroPlugin;
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};
fn default_plugin() -> HydroPlugin {
    HydroPlugin::new(Default::default())
}
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "hydro", "Hydrology: flow accumulation, runoff, inundation, watershed", PluginCategory::Process, [
        sync "hydro_inundation" => "Inundation area from catchment area and rainfall" ; serde_json::json!({"type":"object","properties":{"catchment_area_ha":{"type":"number"},"rainfall_mm":{"type":"number"}},"required":["catchment_area_ha","rainfall_mm"]}) => |args| -> ToolResult {
        let p = default_plugin();
        Ok(serde_json::json!({"inundation_area_m2": p.estimate_inundation_area(args["catchment_area_ha"].as_f64().unwrap_or(0.0), args["rainfall_mm"].as_f64().unwrap_or(0.0))}))
    },
        sync "hydro_runoff" => "Runoff coefficient and peak discharge (Rational Method)" ; serde_json::json!({"type":"object","properties":{"impervious_ratio":{"type":"number"},"rainfall_intensity_mmh":{"type":"number"},"catchment_area_ha":{"type":"number"}},"required":["impervious_ratio","rainfall_intensity_mmh","catchment_area_ha"]}) => |args| -> ToolResult {
        let p = default_plugin();
        let rc = p.runoff_coefficient(args["impervious_ratio"].as_f64().unwrap_or(0.0), 1.0 - args["impervious_ratio"].as_f64().unwrap_or(0.0), 0.0);
        let r = p.peak_discharge(rc, args["rainfall_intensity_mmh"].as_f64().unwrap_or(50.0), args["catchment_area_ha"].as_f64().unwrap_or(0.0));
        serde_json::to_value(&r).map_err(geo_core::errors::GeoError::Serde)
    },
        sync "hydro_flow_accumulation" => "D8 flow accumulation from DEM" ; serde_json::json!({"type":"object","properties":{"dem":{"type":"array","items":{"type":"number"}},"rows":{"type":"integer"},"cols":{"type":"integer"},"cell_size_m":{"type":"number"}},"required":["dem","rows","cols"]}) => |args| -> ToolResult {
        let p = default_plugin();
        let dem: Vec<f64> = args["dem"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
        let rows = args["rows"].as_u64().unwrap_or(0) as usize;
        let cols = args["cols"].as_u64().unwrap_or(0) as usize;
        let r = p.flow_accumulation(&dem, rows, cols, args["cell_size_m"].as_f64().unwrap_or(10.0));
        Ok(serde_json::json!({"catchment_area_ha": r.catchment_area_ha, "cells": rows * cols}))
    },
        sync "hydro_inundation_detail" => "Detailed inundation analysis from DEM + water volume" ; serde_json::json!({"type":"object","properties":{"dem":{"type":"array","items":{"type":"number"}},"water_volume_m3":{"type":"number"},"rows":{"type":"integer"},"cols":{"type":"integer"},"cell_size_m":{"type":"number"}},"required":["dem","water_volume_m3","rows","cols"]}) => |args| -> ToolResult {
        let p = default_plugin();
        let dem: Vec<f64> = args["dem"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
        let r = p.inundation_analysis(&dem, args["water_volume_m3"].as_f64().unwrap_or(0.0), args["rows"].as_u64().unwrap_or(0) as usize, args["cols"].as_u64().unwrap_or(0) as usize, args["cell_size_m"].as_f64().unwrap_or(10.0));
        serde_json::to_value(&r).map_err(geo_core::errors::GeoError::Serde)
    },
        sync "hydro_scs_cn_assessment" => "SCS-CN runoff assessment from landuse/soil grid + rainfall" ; serde_json::json!({"type":"object","properties":{"landuse":{"type":"array","items":{"type":"string"}},"soil_group":{"type":"array","items":{"type":"string"}},"rainfall_mm":{"type":"number"},"amc":{"type":"string","default":"Normal","enum":["Dry","Normal","Wet"]},"cells":{"type":"integer"},"cellsize_m":{"type":"number","default":30}},"required":["landuse","soil_group","rainfall_mm","cells"]}) => |args| -> ToolResult {
            let cells = args["cells"].as_u64().unwrap_or(1) as usize;
            let cellsize_m = args["cellsize_m"].as_f64().unwrap_or(30.0);
            let rainfall = args["rainfall_mm"].as_f64().unwrap_or(0.0);
            let amc = match args["amc"].as_str().unwrap_or("Normal") {
                "Dry" => crate::scs_cn::AMC::Dry,
                "Wet" => crate::scs_cn::AMC::Wet,
                _ => crate::scs_cn::AMC::Normal,
            };
            let landuse: Vec<String> = args["landuse"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()).unwrap_or_default();
            let soil_group: Vec<crate::scs_cn::SoilGroup> = args["soil_group"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(crate::scs_cn::SoilGroup::from_str)).collect()).unwrap_or_default();
            let lu_refs: Vec<&str> = landuse.iter().map(|s| s.as_str()).collect();
            let result = crate::scs_cn::assess_runoff(&lu_refs, &soil_group, rainfall, amc, cells, cellsize_m, 0.2);
            serde_json::to_value(&result).map_err(geo_core::errors::GeoError::Serde)
        },
        sync "hydro_scs_cn_single" => "SCS-CN single cell runoff from CN and rainfall" ; serde_json::json!({"type":"object","properties":{"rainfall_mm":{"type":"number"},"cn":{"type":"number","minimum":0,"maximum":100},"ia_ratio":{"type":"number","default":0.2}},"required":["rainfall_mm","cn"]}) => |args| -> ToolResult {
            let rainfall = args["rainfall_mm"].as_f64().unwrap_or(0.0);
            let cn = args["cn"].as_f64().unwrap_or(70.0);
            let ia_ratio = args["ia_ratio"].as_f64().unwrap_or(0.2);
            let q = crate::scs_cn::compute_runoff(rainfall, cn, ia_ratio);
            let s = crate::scs_cn::compute_s(cn);
            let ia = s * ia_ratio;
            Ok(serde_json::json!({"runoff_mm": q, "s_mm": s, "ia_mm": ia, "rainfall_mm": rainfall, "cn": cn}))
        },
        sync "hydro_invest_carbon" => "InVEST carbon storage from landuse grid" ; serde_json::json!({"type":"object","properties":{"landuse":{"type":"array","items":{"type":"string"}},"cellsize_m":{"type":"number","default":30}},"required":["landuse"]}) => |args| -> ToolResult {
            let landuse: Vec<String> = args["landuse"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()).unwrap_or_default();
            let cellsize_m = args["cellsize_m"].as_f64().unwrap_or(30.0);
            let lu_refs: Vec<&str> = landuse.iter().map(|s| s.as_str()).collect();
            let result = crate::invest::assess_carbon_storage(&lu_refs, cellsize_m, None);
            serde_json::to_value(&result).map_err(geo_core::errors::GeoError::Serde)
        },
        sync "hydro_invest_water_yield" => "InVEST water yield from precipitation/PET/AWC grids" ; serde_json::json!({"type":"object","properties":{"precipitation":{"type":"array","items":{"type":"number"}},"pet":{"type":"array","items":{"type":"number"}},"awc":{"type":"array","items":{"type":"number"}},"z_coefficient":{"type":"number","default":5},"cellsize_m":{"type":"number","default":30}},"required":["precipitation","pet","awc"]}) => |args| -> ToolResult {
            let precip: Vec<f64> = args["precipitation"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
            let pet: Vec<f64> = args["pet"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
            let awc: Vec<f64> = args["awc"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
            let z = args["z_coefficient"].as_f64().unwrap_or(5.0);
            let cellsize_m = args["cellsize_m"].as_f64().unwrap_or(30.0);
            let result = crate::invest::assess_water_yield(&precip, &pet, &awc, z, cellsize_m);
            serde_json::to_value(&result).map_err(geo_core::errors::GeoError::Serde)
        },
        sync "hydro_watershed" => "Watershed extraction from D8 flow direction grid" ; serde_json::json!({"type":"object","properties":{"flow_dir":{"type":"array","items":{"type":"integer"}},"nrows":{"type":"integer"},"ncols":{"type":"integer"},"pour_row":{"type":"integer"},"pour_col":{"type":"integer"},"cell_size_m":{"type":"number","default":10},"xmin":{"type":"number","default":0},"ymax":{"type":"number","default":0}},"required":["flow_dir","nrows","ncols","pour_row","pour_col"]}) => |args| -> ToolResult {
            let flow_dir: Vec<Option<usize>> = args["flow_dir"].as_array().map(|a| a.iter().map(|v| {
                let d = v.as_i64().unwrap_or(-1);
                if (0..8).contains(&d) { Some(d as usize) } else { None }
            }).collect()).unwrap_or_default();
            let nrows = args["nrows"].as_u64().unwrap_or(0) as usize;
            let ncols = args["ncols"].as_u64().unwrap_or(0) as usize;
            let pour_row = args["pour_row"].as_u64().unwrap_or(0) as usize;
            let pour_col = args["pour_col"].as_u64().unwrap_or(0) as usize;
            let cell_size_m = args["cell_size_m"].as_f64().unwrap_or(10.0);
            let xmin = args["xmin"].as_f64().unwrap_or(0.0);
            let ymax = args["ymax"].as_f64().unwrap_or(0.0);
            let result = crate::watershed::extract_watershed(&flow_dir, nrows, ncols, pour_row, pour_col, cell_size_m);
            let geojson = crate::watershed::watershed_to_geojson(&result.cells, ncols, cell_size_m, xmin, ymax);
            Ok(serde_json::json!({"num_cells": result.num_cells, "area_ha": result.area_ha, "geojson": geojson}))
        },
        // ── TR-55 Urban Hydrology ──
        sync "hydro_tr55_cn_lookup" => "TR-55 CN lookup by landuse and soil group" ; serde_json::json!({"type":"object","properties":{"landuse":{"type":"string"},"soil_group":{"type":"string","enum":["A","B","C","D"]}},"required":["landuse","soil_group"]}) => |args| -> ToolResult {
            let lu = args["landuse"].as_str().unwrap_or("woods_good");
            let sg = args["soil_group"].as_str().unwrap_or("B");
            let cn = crate::tr55::tr55_cn_lookup(lu, sg);
            Ok(serde_json::json!({"cn": cn}))
        },
        sync "hydro_tr55_sheet_flow" => "TR-55 sheet flow time of concentration" ; serde_json::json!({"type":"object","properties":{"length_m":{"type":"number"},"slope_pct":{"type":"number"},"manning_n":{"type":"number"},"rainfall_2yr_24h_mm":{"type":"number"}},"required":["length_m","slope_pct","manning_n","rainfall_2yr_24h_mm"]}) => |args| -> ToolResult {
            let tc = crate::tr55::tr55_time_of_concentration_sheet_flow(
                args["length_m"].as_f64().unwrap_or(0.0),
                args["slope_pct"].as_f64().unwrap_or(1.0),
                args["manning_n"].as_f64().unwrap_or(0.011),
                args["rainfall_2yr_24h_mm"].as_f64().unwrap_or(50.0),
            );
            Ok(serde_json::json!({"time_of_concentration_min": (tc * 100.0).round() / 100.0}))
        },
        sync "hydro_tr55_shallow_flow" => "TR-55 shallow concentrated flow time" ; serde_json::json!({"type":"object","properties":{"length_m":{"type":"number"},"slope_pct":{"type":"number"},"surface_type":{"type":"string","enum":["paved","unpaved"]}},"required":["length_m","slope_pct"]}) => |args| -> ToolResult {
            let tc = crate::tr55::tr55_time_of_concentration_shallow_flow(
                args["length_m"].as_f64().unwrap_or(0.0),
                args["slope_pct"].as_f64().unwrap_or(1.0),
                args["surface_type"].as_str().unwrap_or("unpaved"),
            );
            Ok(serde_json::json!({"travel_time_min": (tc * 100.0).round() / 100.0}))
        },
        sync "hydro_tr55_peak_discharge" => "TR-55 peak discharge (graphical method)" ; serde_json::json!({"type":"object","properties":{"runoff_mm":{"type":"number"},"area_km2":{"type":"number"},"tc_hrs":{"type":"number"},"rainfall_type":{"type":"string","enum":["I","II","III"]}},"required":["runoff_mm","area_km2","tc_hrs"]}) => |args| -> ToolResult {
            let r = crate::tr55::tr55_peak_discharge(
                args["runoff_mm"].as_f64().unwrap_or(0.0),
                args["area_km2"].as_f64().unwrap_or(0.0),
                args["tc_hrs"].as_f64().unwrap_or(0.5),
                args["rainfall_type"].as_str().unwrap_or("II"),
            );
            Ok(r)
        },
        sync "hydro_tr55_assessment" => "Full TR-55 hydrology assessment" ; serde_json::json!({"type":"object","properties":{"landuse":{"type":"array","items":{"type":"string"}},"soil_group":{"type":"array","items":{"type":"string"}},"rainfall_mm":{"type":"number"},"area_km2":{"type":"number"},"flow_lengths":{"type":"array","items":{"type":"number"}},"slopes_pct":{"type":"array","items":{"type":"number"}},"rainfall_type":{"type":"string","default":"II"}},"required":["landuse","soil_group","rainfall_mm","area_km2"]}) => |args| -> ToolResult {
            let lu: Vec<&str> = args["landuse"].as_array().map(|a| a.iter().filter_map(|v| v.as_str()).collect()).unwrap_or_default();
            let sg: Vec<&str> = args["soil_group"].as_array().map(|a| a.iter().filter_map(|v| v.as_str()).collect()).unwrap_or_default();
            let flows: Vec<f64> = args["flow_lengths"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
            let slopes: Vec<f64> = args["slopes_pct"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
            let r = crate::tr55::tr55_full_assessment(
                &lu, &sg,
                args["rainfall_mm"].as_f64().unwrap_or(0.0),
                args["area_km2"].as_f64().unwrap_or(0.0),
                &flows, &slopes,
                args["rainfall_type"].as_str().unwrap_or("II"),
            );
            serde_json::to_value(&r).map_err(geo_core::errors::GeoError::Serde)
        },
        // ── Muskingum Flood Routing ──
        sync "hydro_muskingum_route" => "Muskingum flood routing (coefficients + routing)" ; serde_json::json!({"type":"object","properties":{"inflow":{"type":"array","items":{"type":"number"}},"k_hrs":{"type":"number"},"x":{"type":"number","minimum":0,"maximum":0.5},"dt_hrs":{"type":"number"}},"required":["inflow","k_hrs","x","dt_hrs"]}) => |args| -> ToolResult {
            let inflow: Vec<f64> = args["inflow"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
            let outflow = crate::muskingum::muskingum_route(&inflow, args["k_hrs"].as_f64().unwrap_or(1.0), args["x"].as_f64().unwrap_or(0.2), args["dt_hrs"].as_f64().unwrap_or(1.0));
            Ok(serde_json::json!({"outflow": outflow}))
        },
        sync "hydro_muskingum_cunge" => "Muskingum-Cunge routing (rectangular channel)" ; serde_json::json!({"type":"object","properties":{"inflow":{"type":"array","items":{"type":"number"}},"channel_length_m":{"type":"number"},"channel_slope":{"type":"number"},"channel_width_m":{"type":"number"},"manning_n":{"type":"number","default":0.035},"dt_hrs":{"type":"number","default":1}},"required":["inflow","channel_length_m","channel_slope","channel_width_m"]}) => |args| -> ToolResult {
            let inflow: Vec<f64> = args["inflow"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
            let outflow = crate::muskingum::muskingum_cunge_route(&inflow, args["channel_length_m"].as_f64().unwrap_or(1000.0), args["channel_slope"].as_f64().unwrap_or(0.001), args["channel_width_m"].as_f64().unwrap_or(50.0), args["manning_n"].as_f64().unwrap_or(0.035), args["dt_hrs"].as_f64().unwrap_or(1.0));
            Ok(serde_json::json!({"outflow": outflow}))
        },
        sync "hydro_muskingum_attenuation" => "Muskingum flood wave attenuation analysis" ; serde_json::json!({"type":"object","properties":{"inflow":{"type":"array","items":{"type":"number"}},"k_hrs":{"type":"number"},"x":{"type":"number"},"dt_hrs":{"type":"number"}},"required":["inflow","k_hrs","x","dt_hrs"]}) => |args| -> ToolResult {
            let inflow: Vec<f64> = args["inflow"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
            let r = crate::muskingum::attenuation_analysis(&inflow, args["k_hrs"].as_f64().unwrap_or(1.0), args["x"].as_f64().unwrap_or(0.2), args["dt_hrs"].as_f64().unwrap_or(1.0));
            serde_json::to_value(&r).map_err(geo_core::errors::GeoError::Serde)
        },
    ]);
}
