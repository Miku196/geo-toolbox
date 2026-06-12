//! Tool registration — GeoHash index.
use geo_core::plugin::PluginCategory;
use geo_registry::PluginRegistry;
use geo_registry::registry::{ToolDef, ToolResult};

pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta {
        name: "index".into(), version: env!("CARGO_PKG_VERSION").into(),
        description: "GeoHash spatial index: encode, decode, neighbors".into(),
        category: PluginCategory::Process, healthy: true, extra: serde_json::json!({}),
    });
    registry.register_tool_sync("index", ToolDef {
        name: "geohash_encode".into(), description: "Encode lat/lon to GeoHash".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"lat":{"type":"number"},"lon":{"type":"number"},"precision":{"type":"integer"}},"required":["lat","lon"]}),
    }, |args| -> ToolResult {
        Ok(serde_json::json!({"geohash":crate::encode(args["lon"].as_f64().unwrap_or(0.0), args["lat"].as_f64().unwrap_or(0.0), args["precision"].as_u64().unwrap_or(7) as usize)}))
    });
    registry.register_tool_sync("index", ToolDef {
        name: "geohash_decode".into(), description: "Decode GeoHash to bounding box".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"geohash":{"type":"string"}},"required":["geohash"]}),
    }, |args| -> ToolResult {
        let (clon,clat,bbox) = crate::decode(args["geohash"].as_str().unwrap_or("")).ok_or_else(|| geo_core::GeoError::invalid_input("geohash","invalid"))?;
        Ok(serde_json::json!({"center_lon":clon,"center_lat":clat,"west":bbox.min_x,"south":bbox.min_y,"east":bbox.max_x,"north":bbox.max_y}))
    });
    registry.register_tool_sync("index", ToolDef {
        name: "geohash_neighbors".into(), description: "Get 8 neighbor GeoHashes".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"geohash":{"type":"string"}},"required":["geohash"]}),
    }, |args| -> ToolResult { Ok(serde_json::json!({"neighbors":crate::neighbors(args["geohash"].as_str().unwrap_or(""))})) });
}
