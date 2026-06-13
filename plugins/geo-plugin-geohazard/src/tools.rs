use geo_core::plugin::{Plugin, ProcessPlugin};
use geo_registry::registry::ToolDef;
use geo_registry::PluginRegistry;

/// Register geohazard tools into the PluginRegistry.
pub fn register_tools(registry: &mut PluginRegistry) {
    let config_paths = [
        std::path::PathBuf::from("plugins/geo-plugin-geohazard/rules.toml"),
        std::path::PathBuf::from("../../plugins/geo-plugin-geohazard/rules.toml"),
    ];
    let plugin = config_paths
        .iter()
        .find_map(|p| crate::GeohazardPlugin::load_from_file(p).ok())
        .unwrap_or_else(|| crate::GeohazardPlugin::new(Default::default()));

    registry.register(geo_core::plugin::PluginMeta {
        name: plugin.name().to_string(),
        version: plugin.version().to_string(),
        description: plugin.description().to_string(),
        category: plugin.category(),
        healthy: plugin.is_healthy(),
        extra: serde_json::json!({}),
    });

    let plugin_arc = std::sync::Arc::new(plugin);

    // ── Tool 1: 滑坡敏感性 ──
    registry.register_tool_async(
        "geohazard",
        ToolDef {
            name: "geohazard_landslide".into(),
            description: "Compute 6-factor landslide susceptibility index with fuzzy membership. Returns normalized score [0,1] and risk level.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "slope_deg": {"type": "number", "description": "Slope angle in degrees"},
                    "aspect_deg": {"type": "number", "description": "Aspect in degrees (0=North, 180=South)"},
                    "lithology_index": {"type": "number", "description": "Lithology class [0,1]: 0=bedrock, 0.33=semi-rock, 0.67=loose, 1.0=very soft"},
                    "rainfall_mm": {"type": "number", "description": "24-hour rainfall (mm)"},
                    "fault_distance_m": {"type": "number", "description": "Distance to nearest fault (m)"},
                    "ndvi": {"type": "number", "description": "NDVI value [-1,1], lower = less vegetation = higher risk"}
                },
                "required": ["slope_deg", "lithology_index", "rainfall_mm", "fault_distance_m", "ndvi"]
            }),
        },
        {
            let plugin = std::sync::Arc::clone(&plugin_arc);
            move |args| {
                let plugin = std::sync::Arc::clone(&plugin);
                Box::pin(async move {
                    plugin
                        .execute(serde_json::json!({
                            "task": "landslide",
                            "slope_deg": args["slope_deg"].as_f64().unwrap_or(15.0),
                            "aspect_deg": args["aspect_deg"].as_f64().unwrap_or(180.0),
                            "lithology_index": args["lithology_index"].as_f64().unwrap_or(0.5),
                            "rainfall_mm": args["rainfall_mm"].as_f64().unwrap_or(100.0),
                            "fault_distance_m": args["fault_distance_m"].as_f64().unwrap_or(500.0),
                            "ndvi": args["ndvi"].as_f64().unwrap_or(0.3),
                            "aoi_name": args.get("aoi_name").and_then(|v| v.as_str()).unwrap_or("default"),
                        }))
                        .await
                })
            }
        },
    );

    // ── Tool 2: 泥石流危险性 ──
    registry.register_tool_async(
        "geohazard",
        ToolDef {
            name: "geohazard_debris_flow".into(),
            description: "Compute debris flow hazard from channel gradient, loose material volume, and rainfall trigger.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "channel_gradient_deg": {"type": "number", "description": "Channel gradient in degrees"},
                    "material_volume_per_km": {"type": "number", "description": "Loose material volume (m³/km)"},
                    "rainfall_24h_mm": {"type": "number", "description": "24-hour rainfall trigger (mm)"}
                },
                "required": ["channel_gradient_deg", "material_volume_per_km", "rainfall_24h_mm"]
            }),
        },
        {
            let plugin = std::sync::Arc::clone(&plugin_arc);
            move |args| {
                let plugin = std::sync::Arc::clone(&plugin);
                Box::pin(async move {
                    plugin
                        .execute(serde_json::json!({
                            "task": "debris_flow",
                            "channel_gradient_deg": args["channel_gradient_deg"].as_f64().unwrap_or(10.0),
                            "material_volume_per_km": args["material_volume_per_km"].as_f64().unwrap_or(500.0),
                            "rainfall_24h_mm": args["rainfall_24h_mm"].as_f64().unwrap_or(30.0),
                        }))
                        .await
                })
            }
        },
    );

    // ── Tool 3: 综合风险图 ──
    registry.register_tool_async(
        "geohazard",
        ToolDef {
            name: "geohazard_risk_map".into(),
            description: "Combined landslide + debris flow risk assessment. Returns overall risk level.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "aoi_name": {"type": "string", "description": "Area name"},
                    "slope_deg": {"type": "number"},
                    "aspect_deg": {"type": "number", "default": 180},
                    "lithology_index": {"type": "number"},
                    "rainfall_mm": {"type": "number"},
                    "fault_distance_m": {"type": "number"},
                    "ndvi": {"type": "number"},
                    "channel_gradient_deg": {"type": "number", "description": "Optional: for debris flow assessment"},
                    "material_volume_per_km": {"type": "number", "description": "Optional: for debris flow assessment"},
                    "rainfall_24h_mm": {"type": "number", "description": "Optional: for debris flow assessment"}
                },
                "required": ["aoi_name", "slope_deg", "lithology_index", "rainfall_mm", "fault_distance_m", "ndvi"]
            }),
        },
        {
            let plugin = std::sync::Arc::clone(&plugin_arc);
            move |args| {
                let plugin = std::sync::Arc::clone(&plugin);
                Box::pin(async move {
                    plugin.execute(args).await
                })
            }
        },
    );
}
