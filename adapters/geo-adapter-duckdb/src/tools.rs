//! Tool registration — DuckDB.
use geo_core::plugin::PluginCategory;
use geo_registry::PluginRegistry;
use geo_registry::registry::ToolDef;
pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta { name: "duckdb".into(), version: "0.1.0".into(), description: "DuckDB embedded spatial database".into(), category: PluginCategory::Store, healthy: true, extra: serde_json::json!({}) });
    registry.register_tool_async("duckdb", ToolDef { name: "duckdb_query".into(), description: "Execute SQL on in-memory DuckDB, return JSON".into(), input_schema: serde_json::json!({"type":"object","properties":{"sql":{"type":"string"}},"required":["sql"]}) }, |args| Box::pin(async move {
        let store = crate::DuckDbStore::in_memory().map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
        Ok(serde_json::json!(store.query_json(args["sql"].as_str().unwrap_or("SELECT 1")).map_err(|e| geo_core::GeoError::Database(e.to_string()))?))
    }));
    registry.register_tool_async("duckdb", ToolDef { name: "duckdb_ingest_geojson".into(), description: "Ingest GeoJSON into DuckDB".into(), input_schema: serde_json::json!({"type":"object","properties":{"table":{"type":"string"},"geojson":{"type":"string"}},"required":["table","geojson"]}) }, |args| Box::pin(async move {
        let store = crate::DuckDbStore::in_memory().map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
        let t = args["table"].as_str().unwrap_or("features");
        let count = store.ingest_geojson_raw(t, args["geojson"].as_str().unwrap_or("{}")).map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
        Ok(serde_json::json!({"table":t,"ingested":count}))
    }));
}
