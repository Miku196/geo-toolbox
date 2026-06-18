//! Tool registration — GeoHash index.
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "index", "GeoHash spatial index: encode, decode, neighbors", PluginCategory::Process, [
        sync "geohash_encode" => "Encode lat/lon to GeoHash" ; serde_json::json!({"type":"object","properties":{"lat":{"type":"number"},"lon":{"type":"number"},"precision":{"type":"integer"}},"required":["lat","lon"]}) => |args| -> ToolResult {
        Ok(serde_json::json!({"geohash":crate::encode(args["lon"].as_f64().unwrap_or(0.0), args["lat"].as_f64().unwrap_or(0.0), args["precision"].as_u64().unwrap_or(7) as usize)}))
    },
        sync "geohash_decode" => "Decode GeoHash to bounding box" ; serde_json::json!({"type":"object","properties":{"geohash":{"type":"string"}},"required":["geohash"]}) => |args| -> ToolResult {
        let (clon,clat,bbox) = crate::decode(args["geohash"].as_str().unwrap_or("")).ok_or_else(|| geo_core::GeoError::invalid_input("geohash","invalid"))?;
        Ok(serde_json::json!({"center_lon":clon,"center_lat":clat,"west":bbox.min_x,"south":bbox.min_y,"east":bbox.max_x,"north":bbox.max_y}))
    },
        sync "geohash_neighbors" => "Get 8 neighbor GeoHashes" ; serde_json::json!({"type":"object","properties":{"geohash":{"type":"string"}},"required":["geohash"]}) => |args| -> ToolResult {
        Ok(serde_json::json!({"neighbors":crate::neighbors(args["geohash"].as_str().unwrap_or(""))}))
    }]);
}
