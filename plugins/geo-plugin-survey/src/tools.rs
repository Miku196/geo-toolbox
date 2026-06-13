use crate::SurveyPlugin;
use geo_core::plugin::PluginCategory;
use geo_registry::registry::{ToolDef, ToolResult};
use geo_registry::PluginRegistry;

fn default_plugin() -> SurveyPlugin {
    SurveyPlugin::new(crate::trait_impl::make_default_config())
}

pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta {
        name: "survey".into(),
        version: "0.2.0".into(),
        description: "Surveying: grid earthwork, cross-section, TIN, control network adjustment"
            .into(),
        category: PluginCategory::Process,
        healthy: true,
        extra: serde_json::json!({}),
    });

    // ── Tool 1: survey_earthwork ──
    registry.register_tool_sync(
        "survey",
        ToolDef {
            name: "survey_earthwork".into(),
            description: "Grid method earthwork calculation (cut/fill/net volumes)".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "existing_elevation": {"type": "array", "items": {"type": "number"}},
                    "design_elevation": {"type": "number"},
                    "grid_cols": {"type": "integer"},
                    "grid_rows": {"type": "integer"},
                },
                "required": ["existing_elevation", "design_elevation", "grid_cols", "grid_rows"],
            }),
        },
        |args| -> ToolResult {
            let p = default_plugin();
            let elev: Vec<f64> = args["existing_elevation"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
                .unwrap_or_default();
            let design = args["design_elevation"].as_f64().unwrap_or(0.0);
            let cols = args["grid_cols"].as_u64().unwrap_or(0) as usize;
            let rows = args["grid_rows"].as_u64().unwrap_or(0) as usize;
            let r = p.grid_earthwork(&elev, design, cols, rows);
            Ok(serde_json::to_value(&r).map_err(|e| geo_core::errors::GeoError::Serde(e))?)
        },
    );

    // ── Tool 2: survey_cross_section ──
    registry.register_tool_sync(
        "survey",
        ToolDef {
            name: "survey_cross_section".into(),
            description: "Average end area cross-section earthwork (road/rail)".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "cut_areas_m2": {"type": "array", "items": {"type": "number"}},
                    "fill_areas_m2": {"type": "array", "items": {"type": "number"}},
                    "distances_m": {"type": "array", "items": {"type": "number"}},
                },
                "required": ["cut_areas_m2", "fill_areas_m2", "distances_m"],
            }),
        },
        |args| -> ToolResult {
            let p = default_plugin();
            let cuts: Vec<f64> = args["cut_areas_m2"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
                .unwrap_or_default();
            let fills: Vec<f64> = args["fill_areas_m2"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
                .unwrap_or_default();
            let dists: Vec<f64> = args["distances_m"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
                .unwrap_or_default();
            let n = cuts.len().min(fills.len());
            let sections: Vec<(f64, f64)> = (0..n).map(|i| (cuts[i], fills[i])).collect();
            let r = p.cross_section_earthwork(&sections, &dists);
            Ok(serde_json::to_value(&r).map_err(|e| geo_core::errors::GeoError::Serde(e))?)
        },
    );

    // ── Tool 3: survey_adjustment ──
    registry.register_tool_sync(
        "survey",
        ToolDef {
            name: "survey_adjustment".into(),
            description: "Control network adjustment (simplified least squares)".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "observations": {"type": "array"},
                    "initial": {"type": "number"},
                },
                "required": ["observations"],
            }),
        },
        |args| -> ToolResult {
            let p = default_plugin();
            let obs: Vec<(f64, f64)> = args["observations"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| {
                            let arr = v.as_array()?;
                            Some((arr.get(0)?.as_f64()?, arr.get(1)?.as_f64()?))
                        })
                        .collect()
                })
                .unwrap_or_default();
            let init = args["initial"].as_f64().unwrap_or(0.0);
            let r = p.control_network_adjustment(&obs, init);
            Ok(serde_json::to_value(&r).map_err(|e| geo_core::errors::GeoError::Serde(e))?)
        },
    );

    // ── Tool 4: survey_tin ──
    registry.register_tool_sync(
        "survey",
        ToolDef {
            name: "survey_tin".into(),
            description: "TIN (triangular prism) earthwork volume calculation".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "points": {"type": "array", "items": {"type": "object"}},
                    "design_elevation": {"type": "number"},
                },
                "required": ["points", "design_elevation"],
            }),
        },
        |args| -> ToolResult {
            let p = default_plugin();
            let pts: Vec<crate::survey::ElevationPoint> = args["points"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| {
                            Some(crate::survey::ElevationPoint {
                                x: v["x"].as_f64()?,
                                y: v["y"].as_f64()?,
                                z: v["z"].as_f64()?,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            let design = args["design_elevation"].as_f64().unwrap_or(0.0);
            let vol = p.tin_earthwork(&pts, design);
            Ok(serde_json::json!({"volume_m3": vol}))
        },
    );
}
