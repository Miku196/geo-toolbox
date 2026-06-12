//! Tool registration — QGIS.
use geo_core::plugin::PluginCategory;
use geo_registry::PluginRegistry;
use geo_registry::registry::ToolDef;
use std::path::PathBuf;
fn qgis_config() -> crate::QgisProcessConfig {
    let exe = std::env::var("QGIS_PROCESS_PATH").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("qgis_process"));
    crate::QgisProcessConfig { executable: exe, ..Default::default() }
}
pub fn register_tools(registry: &mut PluginRegistry) {
    let ep = std::env::var("QGIS_PROCESS_PATH").unwrap_or_else(|_| "qgis_process".into());
    registry.register(geo_core::plugin::PluginMeta { name: "qgis".into(), version: "0.1.0".into(), description: "QGIS processing bridge — qgis_process".into(), category: PluginCategory::Adapter, healthy: true, extra: serde_json::json!({"endpoint":ep}) });
    registry.register_tool_async("qgis", ToolDef { name: "qgis_buffer".into(), description: "Run QGIS buffer via qgis_process".into(), input_schema: serde_json::json!({"type":"object","properties":{"input":{"type":"string"},"distance":{"type":"number"},"output":{"type":"string"}},"required":["input","distance","output"]}) }, |args| Box::pin(async move {
        let runner = crate::BatchQgisRunner::new(qgis_config());
        runner.buffer(args["input"].as_str().unwrap_or(""), args["distance"].as_f64().unwrap_or(0.0), args["output"].as_str().unwrap_or("")).await.map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(serde_json::json!({"output":args["output"].as_str().unwrap_or("")}))
    }));
    registry.register_tool_async("qgis", ToolDef { name: "qgis_reproject".into(), description: "Reproject vector layer via qgis_process".into(), input_schema: serde_json::json!({"type":"object","properties":{"input":{"type":"string"},"epsg":{"type":"integer"},"output":{"type":"string"}},"required":["input","epsg","output"]}) }, |args| Box::pin(async move {
        let runner = crate::BatchQgisRunner::new(qgis_config());
        runner.reproject(args["input"].as_str().unwrap_or(""), args["epsg"].as_u64().unwrap_or(4326) as u16, args["output"].as_str().unwrap_or("")).await.map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(serde_json::json!({"output":args["output"].as_str().unwrap_or("")}))
    }));
}
