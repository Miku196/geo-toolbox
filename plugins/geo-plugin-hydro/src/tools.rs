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
    }]);
}
