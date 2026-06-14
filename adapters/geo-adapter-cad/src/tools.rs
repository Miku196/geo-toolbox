//! Tool registration — CAD.
use geo_registry::{register_plugin, PluginRegistry};
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "cad", "CAD format exporter: GeoJSON from PostGIS", PluginCategory::Output, [
        async "cad_export_geojson" => "Export PostGIS query to GeoJSON file" ; serde_json::json!({"type":"object","properties":{"sql":{"type":"string"},"output":{"type":"string"},"db_url":{"type":"string"}},"required":["sql","output","db_url"]}) => |args| Box::pin(async move {
        let db = args["db_url"].as_str().unwrap_or("");
        if db.is_empty() { return Err(geo_core::GeoError::invalid_input("db_url","required")); }
        let pool = sqlx::postgres::PgPoolOptions::new().max_connections(2).connect(db).await.map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
        let count = crate::GeoJsonExporter::new(pool).from_sql(args["sql"].as_str().unwrap_or(""), args["output"].as_str().unwrap_or("")).await.map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(serde_json::json!({"output":args["output"].as_str().unwrap_or(""),"features":count}))
    })]);
}
