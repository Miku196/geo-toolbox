//! Tool registration — Hydrology plugin.
use crate::HydroPlugin;
use geo_core::plugin::PluginCategory;
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};
fn default_plugin() -> HydroPlugin {
    HydroPlugin::new(crate::trait_impl::make_default_config())
}
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "hydro", "Hydrology: flow accumulation, runoff, inundation", PluginCategory::Process, [
        sync "hydro_inundation" => "Inundation area from catchment area and rainfall" ; serde_json::json!({"type":"object","properties":{"catchment_area_ha":{"type":"number"},"rainfall_mm":{"type":"number"}},"required":["catchment_area_ha","rainfall_mm"]}) => |args| -> ToolResult {
        let p = default_plugin();
        Ok(serde_json::json!({"inundation_area_m2": p.estimate_inundation_area(args["catchment_area_ha"].as_f64().unwrap_or(0.0), args["rainfall_mm"].as_f64().unwrap_or(0.0))}))
    },
        sync "hydro_runoff" => "Runoff coefficient and peak discharge (Rational Method)" ; serde_json::json!({"type":"object","properties":{"impervious_ratio":{"type":"number"},"rainfall_intensity_mmh":{"type":"number"},"catchment_area_ha":{"type":"number"}},"required":["impervious_ratio","rainfall_intensity_mmh","catchment_area_ha"]}) => |args| -> ToolResult {
        let p = default_plugin();
        let rc = p.runoff_coefficient(args["impervious_ratio"].as_f64().unwrap_or(0.0), 1.0 - args["impervious_ratio"].as_f64().unwrap_or(0.0), 0.0);
        let r = p.peak_discharge(rc, args["rainfall_intensity_mmh"].as_f64().unwrap_or(50.0), args["catchment_area_ha"].as_f64().unwrap_or(0.0));
        Ok(serde_json::to_value(&r).map_err(|e| geo_core::errors::GeoError::Serde(e))?)
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
        Ok(serde_json::to_value(&r).map_err(|e| geo_core::errors::GeoError::Serde(e))?)
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
            let soil_group: Vec<crate::scs_cn::SoilGroup> = args["soil_group"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(|s| crate::scs_cn::SoilGroup::from_str(s))).collect()).unwrap_or_default();
            let lu_refs: Vec<&str> = landuse.iter().map(|s| s.as_str()).collect();
            let result = crate::scs_cn::assess_runoff(&lu_refs, &soil_group, rainfall, amc, cells, cellsize_m, 0.2);
            Ok(serde_json::to_value(&result).map_err(|e| geo_core::errors::GeoError::Serde(e))?)
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
            Ok(serde_json::to_value(&result).map_err(|e| geo_core::errors::GeoError::Serde(e))?)
        },
        sync "hydro_invest_water_yield" => "InVEST water yield from precipitation/PET/AWC grids" ; serde_json::json!({"type":"object","properties":{"precipitation":{"type":"array","items":{"type":"number"}},"pet":{"type":"array","items":{"type":"number"}},"awc":{"type":"array","items":{"type":"number"}},"z_coefficient":{"type":"number","default":5},"cellsize_m":{"type":"number","default":30}},"required":["precipitation","pet","awc"]}) => |args| -> ToolResult {
            let precip: Vec<f64> = args["precipitation"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
            let pet: Vec<f64> = args["pet"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
            let awc: Vec<f64> = args["awc"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
            let z = args["z_coefficient"].as_f64().unwrap_or(5.0);
            let cellsize_m = args["cellsize_m"].as_f64().unwrap_or(30.0);
            let result = crate::invest::assess_water_yield(&precip, &pet, &awc, z, cellsize_m);
            Ok(serde_json::to_value(&result).map_err(|e| geo_core::errors::GeoError::Serde(e))?)
        },
    ]);
}
