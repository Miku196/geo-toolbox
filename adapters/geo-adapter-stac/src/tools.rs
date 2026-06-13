//! Tool registration — STAC.
use geo_core::plugin::PluginCategory;
use geo_registry::registry::ToolDef;
use geo_registry::PluginRegistry;
pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta { name: "stac".into(), version: "0.1.0".into(), description: "STAC API client — search satellite imagery".into(), category: PluginCategory::Adapter, healthy: true, extra: serde_json::json!({"endpoint":"https://planetarycomputer.microsoft.com/api/stac/v1"}) });
    registry.register_tool_async("stac", ToolDef { name: "stac_search".into(), description: "Search STAC catalog by bbox and date range".into(), input_schema: serde_json::json!({"type":"object","properties":{"collection":{"type":"string","default":"sentinel-2-l2a"},"min_lon":{"type":"number"},"min_lat":{"type":"number"},"max_lon":{"type":"number"},"max_lat":{"type":"number"},"date_from":{"type":"string"},"date_to":{"type":"string"},"limit":{"type":"integer"},"endpoint":{"type":"string"}},"required":["min_lon","min_lat","max_lon","max_lat","date_from","date_to"]}) }, |args| Box::pin(async move {
        let ep = args["endpoint"].as_str().unwrap_or("https://planetarycomputer.microsoft.com/api/stac/v1");
        let client = crate::StacClient::new(ep);
        let limit = (args["limit"].as_u64().unwrap_or(10) as usize).try_into().unwrap_or(10);
        let items = client.search(args["collection"].as_str().unwrap_or("sentinel-2-l2a"),args["min_lon"].as_f64().unwrap_or(0.0),args["min_lat"].as_f64().unwrap_or(0.0),args["max_lon"].as_f64().unwrap_or(0.0),args["max_lat"].as_f64().unwrap_or(0.0),args["date_from"].as_str().unwrap_or("2025-01-01"),args["date_to"].as_str().unwrap_or("2025-12-31"),limit).await.map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(serde_json::to_value(items).map_err(geo_core::GeoError::Serde)?)
    }));
}
