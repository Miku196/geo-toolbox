use crate::UrbanPlugin;
use geo_core::plugin::PluginCategory;
use geo_registry::registry::{ToolDef, ToolResult};
use geo_registry::PluginRegistry;

fn default_plugin() -> UrbanPlugin {
    UrbanPlugin::new(crate::trait_impl::make_default_config())
}

pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta {
        name: "urban".into(),
        version: "0.2.0".into(),
        description: "Urban planning: FAR, land use, solar analysis, UHI, ventilation".into(),
        category: PluginCategory::Process,
        healthy: true,
        extra: serde_json::json!({}),
    });

    // ── Tool 1: urban_far ──
    registry.register_tool_sync(
        "urban",
        ToolDef {
            name: "urban_far".into(),
            description: "Compute FAR, building density, average height, and compliance check"
                .into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "total_floor_area_m2": {"type": "number"},
                    "building_footprint_m2": {"type": "number"},
                    "site_area_m2": {"type": "number"},
                },
                "required": ["total_floor_area_m2", "building_footprint_m2", "site_area_m2"],
            }),
        },
        |args| -> ToolResult {
            let p = default_plugin();
            let tfa = args["total_floor_area_m2"].as_f64().unwrap_or(0.0);
            let bf = args["building_footprint_m2"].as_f64().unwrap_or(0.0);
            let sa = args["site_area_m2"].as_f64().unwrap_or(0.0);

            let far = p.far(tfa, sa);
            let density = p.building_density(bf, sa);
            let avg_h = p.estimate_avg_height(far, density);
            let (fc, dc) = p.check_compliance(far, density);

            Ok(serde_json::json!({
                "far": far,
                "building_density": density,
                "estimated_avg_height_m": avg_h,
                "far_compliant": fc,
                "density_compliant": dc,
            }))
        },
    );

    // ── Tool 2: urban_land_use ──
    registry.register_tool_sync(
        "urban",
        ToolDef {
            name: "urban_land_use".into(),
            description: "Classify land use (NLCD) from NDVI and impervious surface arrays".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "ndvi_values": {"type": "array", "items": {"type": "number"}},
                    "impervious_values": {"type": "array", "items": {"type": "number"}},
                    "total_area_ha": {"type": "number"},
                },
            }),
        },
        |args| -> ToolResult {
            let p = default_plugin();
            let ndvi: Vec<Option<f64>> = args["ndvi_values"]
                .as_array()
                .map(|a| a.iter().map(|v| v.as_f64()).collect())
                .unwrap_or_default();
            let imp: Vec<Option<f64>> = args["impervious_values"]
                .as_array()
                .map(|a| a.iter().map(|v| v.as_f64()).collect())
                .unwrap_or_default();
            let area_ha = args["total_area_ha"].as_f64().unwrap_or(0.0);
            let stats = p.land_use_stats(&ndvi, &imp, area_ha);
            Ok(serde_json::json!({"land_use_areas_ha": stats}))
        },
    );

    // ── Tool 3: urban_heat_island ──
    registry.register_tool_sync("urban", ToolDef {
        name: "urban_heat_island".into(),
        description: "Compute urban heat island index from impervious ratio, building density, and green ratio".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "impervious_ratio": {"type": "number"},
                "building_density": {"type": "number"},
                "green_ratio": {"type": "number"},
            },
            "required": ["impervious_ratio", "building_density", "green_ratio"],
        }),
    }, |args| -> ToolResult {
        let p = default_plugin();
        let imp = args["impervious_ratio"].as_f64().unwrap_or(0.0);
        let den = args["building_density"].as_f64().unwrap_or(0.0);
        let gr = args["green_ratio"].as_f64().unwrap_or(0.0);
        let uhi = p.uhi_index(imp, den, gr);
        Ok(serde_json::json!({
            "uhi_index": uhi.uhi_index,
            "risk_level": uhi.risk_level,
        }))
    });

    // ── Tool 4: urban_green_space ──
    registry.register_tool_sync(
        "urban",
        ToolDef {
            name: "urban_green_space".into(),
            description: "Compute green ratio, per capita green space, and compliance".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "green_area_m2": {"type": "number"},
                    "total_area_m2": {"type": "number"},
                    "population": {"type": "integer"},
                },
                "required": ["green_area_m2", "total_area_m2"],
            }),
        },
        |args| -> ToolResult {
            let p = default_plugin();
            let ga = args["green_area_m2"].as_f64().unwrap_or(0.0);
            let ta = args["total_area_m2"].as_f64().unwrap_or(0.0);
            let pop = args["population"].as_u64().unwrap_or(0);
            let ratio = p.green_ratio(ga, ta);
            let pc = p.green_per_capita(ga, pop);
            let min_ratio = p.config().vegetation.min_green_ratio;
            let min_pc = p.config().vegetation.min_green_per_capita_m2;
            Ok(serde_json::json!({
                "green_ratio": ratio,
                "green_per_capita_m2": pc,
                "ratio_compliant": ratio >= min_ratio,
                "per_capita_compliant": pc >= min_pc,
            }))
        },
    );

    // ── Tool 5: urban_solar ──
    registry.register_tool_sync(
        "urban",
        ToolDef {
            name: "urban_solar".into(),
            description: "Solar / shadow analysis for a building (winter + summer)".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "building_height_m": {"type": "number"},
                    "neighbor_distance_m": {"type": "number"},
                },
                "required": ["building_height_m"],
            }),
        },
        |args| -> ToolResult {
            let p = default_plugin();
            let h = args["building_height_m"].as_f64().unwrap_or(30.0);
            let nd = args["neighbor_distance_m"].as_f64().unwrap_or(50.0);
            let sr = p.solar_analysis(h, nd);
            Ok(crate::trait_impl::solar_to_json(&sr))
        },
    );

    // ── Tool 6: urban_assess ──
    registry.register_tool_sync(
        "urban",
        ToolDef {
            name: "urban_assess".into(),
            description: "Comprehensive urban planning assessment (all indicators at once)".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "total_floor_area_m2": {"type": "number"},
                    "building_footprint_m2": {"type": "number"},
                    "site_area_m2": {"type": "number"},
                    "green_area_m2": {"type": "number"},
                    "population": {"type": "integer"},
                    "impervious_ratio": {"type": "number"},
                    "ndvi_values": {"type": "array", "items": {"type": "number"}},
                    "impervious_values": {"type": "array", "items": {"type": "number"}},
                },
                "required": ["total_floor_area_m2", "building_footprint_m2", "site_area_m2"],
            }),
        },
        |args| -> ToolResult {
            let p = default_plugin();
            let tfa = args["total_floor_area_m2"].as_f64().unwrap_or(0.0);
            let bf = args["building_footprint_m2"].as_f64().unwrap_or(0.0);
            let sa = args["site_area_m2"].as_f64().unwrap_or(0.0);
            let ga = args["green_area_m2"].as_f64().unwrap_or(0.0);
            let pop = args["population"].as_u64().unwrap_or(0);
            let imp = args["impervious_ratio"].as_f64().unwrap_or(0.0);
            let ndvi: Vec<Option<f64>> = args["ndvi_values"]
                .as_array()
                .map(|a| a.iter().map(|v| v.as_f64()).collect())
                .unwrap_or_default();
            let iv: Vec<Option<f64>> = args["impervious_values"]
                .as_array()
                .map(|a| a.iter().map(|v| v.as_f64()).collect())
                .unwrap_or_default();

            let a = p.assess(tfa, bf, sa, ga, pop, imp, &ndvi, &iv);
            Ok(serde_json::to_value(&a).map_err(|e| geo_core::errors::GeoError::Serde(e))?)
        },
    );
}
