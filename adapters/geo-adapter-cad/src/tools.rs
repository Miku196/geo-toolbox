//! Tool registration — CAD.
use geo_core::plugin::PluginCategory;
use geo_registry::registry::ToolDef;
use geo_registry::PluginRegistry;
pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta {
        name: "cad".into(),
        version: "0.1.0".into(),
        description: "CAD format exporter: GeoJSON from PostGIS".into(),
        category: PluginCategory::Output,
        healthy: true,
        extra: serde_json::json!({}),
    });
    registry.register_tool_async("cad", ToolDef { name: "cad_export_geojson".into(), description: "Export PostGIS query to GeoJSON file".into(), input_schema: serde_json::json!({"type":"object","properties":{"sql":{"type":"string"},"output":{"type":"string"},"db_url":{"type":"string"}},"required":["sql","output","db_url"]}) }, |args| Box::pin(async move {
        let db = args["db_url"].as_str().unwrap_or("");
        if db.is_empty() { return Err(geo_core::GeoError::invalid_input("db_url","required")); }
        let pool = sqlx::postgres::PgPoolOptions::new().max_connections(2).connect(db).await.map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
        let count = crate::GeoJsonExporter::new(pool).from_sql(args["sql"].as_str().unwrap_or(""), args["output"].as_str().unwrap_or("")).await.map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(serde_json::json!({"output":args["output"].as_str().unwrap_or(""),"features":count}))
    }));
}
