//! Tool registration — DuckDB.
use geo_registry::{register_plugin, PluginRegistry};
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "duckdb", "DuckDB embedded spatial database", PluginCategory::Store, [
        async "duckdb_query" => "Execute SQL on in-memory DuckDB, return JSON" ; serde_json::json!({"type":"object","properties":{"sql":{"type":"string"}},"required":["sql"]}) => |args| Box::pin(async move {
        let store = crate::DuckDbStore::in_memory().map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
        Ok(serde_json::json!(store.query_json(args["sql"].as_str().unwrap_or("SELECT 1")).map_err(|e| geo_core::GeoError::Database(e.to_string()))?))
    }),
        async "duckdb_ingest_geojson" => "Ingest GeoJSON into DuckDB" ; serde_json::json!({"type":"object","properties":{"table":{"type":"string"},"geojson":{"type":"string"}},"required":["table","geojson"]}) => |args| Box::pin(async move {
        let store = crate::DuckDbStore::in_memory().map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
        let t = args["table"].as_str().unwrap_or("features");
        let count = store.ingest_geojson_raw(t, args["geojson"].as_str().unwrap_or("{}")).map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
        Ok(serde_json::json!({"table":t,"ingested":count}))
    })]);
}
