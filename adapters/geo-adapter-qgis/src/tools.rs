//! Tool registration — QGIS.
use geo_core::plugin::PluginCategory;
use geo_registry::registry::ToolDef;
use geo_registry::PluginRegistry;

pub fn register_tools(registry: &mut PluginRegistry) {
    let adapter = crate::QgisAdapter::from_env();
    let backend_label = adapter.active_backend().to_string();

    registry.register(geo_core::plugin::PluginMeta {
        name: "qgis".into(),
        version: "0.1.0".into(),
        description: format!("QGIS processing bridge — {}", backend_label),
        category: PluginCategory::Adapter,
        healthy: true,
        extra: serde_json::json!({"endpoint": format!("{}", backend_label)}),
    });

    // Async — both backends produce output files, need async for subprocess
    registry.register_tool_async(
        "qgis",
        ToolDef {
            name: "qgis_buffer".into(),
            description: "Run QGIS buffer".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string"},
                    "distance": {"type": "number"},
                    "output": {"type": "string"}
                },
                "required": ["input", "distance", "output"]
            }),
        },
        |args| {
            let input = args["input"].as_str().unwrap_or("").to_string();
            let distance = args["distance"].as_f64().unwrap_or(0.0);
            let output = args["output"].as_str().unwrap_or("").to_string();
            Box::pin(async move {
                let adapter = crate::QgisAdapter::from_env();
                let result = adapter
                    .buffer(
                        std::path::Path::new(&input),
                        distance,
                        std::path::Path::new(&output),
                    )
                    .await
                    .map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
                Ok(serde_json::json!({ "output": result.to_string_lossy() }))
            })
        },
    );

    registry.register_tool_async(
        "qgis",
        ToolDef {
            name: "qgis_reproject".into(),
            description: "Reproject vector layer".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string"},
                    "epsg": {"type": "integer"},
                    "output": {"type": "string"}
                },
                "required": ["input", "epsg", "output"]
            }),
        },
        |args| {
            let input = args["input"].as_str().unwrap_or("").to_string();
            let epsg = args["epsg"].as_u64().unwrap_or(4326) as u16;
            let output = args["output"].as_str().unwrap_or("").to_string();
            Box::pin(async move {
                let adapter = crate::QgisAdapter::from_env();
                let result = adapter
                    .reproject(
                        std::path::Path::new(&input),
                        epsg,
                        std::path::Path::new(&output),
                    )
                    .await
                    .map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
                Ok(serde_json::json!({ "output": result.to_string_lossy() }))
            })
        },
    );
}
