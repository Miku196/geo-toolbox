use crate::HydroPlugin;
use geo_core::plugin::PluginCategory;
use geo_registry::registry::{ToolDef, ToolResult};
use geo_registry::PluginRegistry;

fn default_plugin() -> HydroPlugin {
    HydroPlugin::new(crate::trait_impl::make_default_config())
}

pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta {
        name: "hydro".into(),
        version: "0.2.0".into(),
        description: "Hydrology: flow accumulation, runoff, inundation".into(),
        category: PluginCategory::Process,
        healthy: true,
        extra: serde_json::json!({}),
    });

    // Tool 1: hydro_inundation
    registry.register_tool_sync(
        "hydro",
        ToolDef {
            name: "hydro_inundation".into(),
            description: "Inundation area from catchment area and rainfall".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "catchment_area_ha": {"type": "number"},
                    "rainfall_mm": {"type": "number"},
                },
                "required": ["catchment_area_ha", "rainfall_mm"],
            }),
        },
        |args| -> ToolResult {
            let p = default_plugin();
            let area_ha = args["catchment_area_ha"].as_f64().unwrap_or(0.0);
            let rain = args["rainfall_mm"].as_f64().unwrap_or(0.0);
            let a = p.estimate_inundation_area(area_ha, rain);
            Ok(serde_json::json!({"inundation_area_m2": a}))
        },
    );

    // Tool 2: hydro_runoff
    registry.register_tool_sync(
        "hydro",
        ToolDef {
            name: "hydro_runoff".into(),
            description: "Runoff coefficient and peak discharge (Rational Method)".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "impervious_ratio": {"type": "number"},
                    "rainfall_intensity_mmh": {"type": "number"},
                    "catchment_area_ha": {"type": "number"},
                },
                "required": ["impervious_ratio", "rainfall_intensity_mmh", "catchment_area_ha"],
            }),
        },
        |args| -> ToolResult {
            let p = default_plugin();
            let imp = args["impervious_ratio"].as_f64().unwrap_or(0.0);
            let rain = args["rainfall_intensity_mmh"].as_f64().unwrap_or(50.0);
            let area = args["catchment_area_ha"].as_f64().unwrap_or(0.0);
            let rc = p.runoff_coefficient(imp, 1.0 - imp, 0.0);
            let r = p.peak_discharge(rc, rain, area);
            Ok(serde_json::to_value(&r).map_err(|e| geo_core::errors::GeoError::Serde(e))?)
        },
    );

    // Tool 3: hydro_flow_accumulation
    registry.register_tool_sync(
        "hydro",
        ToolDef {
            name: "hydro_flow_accumulation".into(),
            description: "D8 flow accumulation from DEM".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "dem": {"type": "array", "items": {"type": "number"}},
                    "rows": {"type": "integer"},
                    "cols": {"type": "integer"},
                    "cell_size_m": {"type": "number"},
                },
                "required": ["dem", "rows", "cols"],
            }),
        },
        |args| -> ToolResult {
            let p = default_plugin();
            let dem: Vec<f64> = args["dem"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
                .unwrap_or_default();
            let rows = args["rows"].as_u64().unwrap_or(0) as usize;
            let cols = args["cols"].as_u64().unwrap_or(0) as usize;
            let cell = args["cell_size_m"].as_f64().unwrap_or(10.0);
            let r = p.flow_accumulation(&dem, rows, cols, cell);
            Ok(serde_json::json!({"catchment_area_ha": r.catchment_area_ha, "cells": rows * cols}))
        },
    );

    // Tool 4: hydro_inundation_detail
    registry.register_tool_sync(
        "hydro",
        ToolDef {
            name: "hydro_inundation_detail".into(),
            description: "Detailed inundation analysis from DEM + water volume".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "dem": {"type": "array", "items": {"type": "number"}},
                    "water_volume_m3": {"type": "number"},
                    "rows": {"type": "integer"},
                    "cols": {"type": "integer"},
                    "cell_size_m": {"type": "number"},
                },
                "required": ["dem", "water_volume_m3", "rows", "cols"],
            }),
        },
        |args| -> ToolResult {
            let p = default_plugin();
            let dem: Vec<f64> = args["dem"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
                .unwrap_or_default();
            let vol = args["water_volume_m3"].as_f64().unwrap_or(0.0);
            let rows = args["rows"].as_u64().unwrap_or(0) as usize;
            let cols = args["cols"].as_u64().unwrap_or(0) as usize;
            let cell = args["cell_size_m"].as_f64().unwrap_or(10.0);
            let r = p.inundation_analysis(&dem, vol, rows, cols, cell);
            Ok(serde_json::to_value(&r).map_err(|e| geo_core::errors::GeoError::Serde(e))?)
        },
    );
}
