//! Tool registration for PluginRegistry — GEE tools.
//!
//! Extracted from CLI build_registry() to give the GEE adapter locality
//! over its tool definitions and handlers.

use geo_core::plugin::PluginCategory;
use geo_registry::PluginRegistry;
use geo_registry::registry::ToolDef;

/// Register GEE tools into the PluginRegistry.
pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta {
        name: "gee".into(),
        version: "0.1.0".into(),
        description: "Google Earth Engine remote sensing adapter".into(),
        category: PluginCategory::Adapter,
        healthy: true,
        extra: serde_json::json!({}),
    });

    registry.register_tool_async("gee", ToolDef {
        name: "gee_classify".into(),
        description: "Submit landcover classification task to GEE".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "aoi": {"type": "string"},
                "year": {"type": "integer"},
                "output_gcs": {"type": "string"},
            },
            "required": ["aoi", "year", "output_gcs"],
        }),
    }, |args| Box::pin(async move {
        let adapter = crate::GeeAdapter::new_default().await
            .map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        let aoi = args["aoi"].as_str().unwrap_or("");
        let year = args["year"].as_u64().unwrap_or(2025) as u16;
        let output_gcs = args["output_gcs"].as_str().unwrap_or("gs://gee-exports/lc.tif");
        let task = adapter
            .submit_classification(aoi, year, "COPERNICUS/S2_SR_HARMONIZED", output_gcs)
            .await
            .map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(serde_json::json!({
            "task_id": task, "aoi": aoi, "year": year,
            "collection": "COPERNICUS/S2_SR_HARMONIZED",
        }))
    }));

    registry.register_tool_async("gee", ToolDef {
        name: "gee_status".into(),
        description: "Check GEE task status by correlation ID".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {"cid": {"type": "string"}},
            "required": ["cid"],
        }),
    }, |args| Box::pin(async move {
        let adapter = crate::GeeAdapter::new_default().await
            .map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        let cid = args["cid"].as_str().unwrap_or("");
        let status = adapter.job_status(cid).await
            .map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(serde_json::json!({"cid": cid, "status": status}))
    }));
}
