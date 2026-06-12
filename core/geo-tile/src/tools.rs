//! Tool registration — Tile engine.
use geo_core::plugin::PluginCategory;
use geo_registry::PluginRegistry;
use geo_registry::registry::{ToolDef, ToolResult};

/// Register tile tools into the PluginRegistry.
pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta {
        name: "tile".into(), version: env!("CARGO_PKG_VERSION").into(),
        description: "Vector tile (MVT) encoder + raster tile (PMTiles)".into(),
        category: PluginCategory::Process, healthy: true, extra: serde_json::json!({}),
    });
    registry.register_tool_sync("tile", ToolDef {
        name: "tile_latlon_to_tile".into(), description: "Convert lat/lon to tile z/x/y".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"lon":{"type":"number"},"lat":{"type":"number"},"zoom":{"type":"integer"}},"required":["lon","lat","zoom"]}),
    }, |args| -> ToolResult {
        let (x,y,z) = crate::latlon_to_tile(args["lon"].as_f64().unwrap_or(0.0), args["lat"].as_f64().unwrap_or(0.0), args["zoom"].as_u64().unwrap_or(0) as u8);
        Ok(serde_json::json!({"x":x,"y":y,"z":z}))
    });
    registry.register_tool_sync("tile", ToolDef {
        name: "tile_bounds".into(), description: "Get lat/lon bounds of a tile".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"x":{"type":"integer"},"y":{"type":"integer"},"z":{"type":"integer"}},"required":["x","y","z"]}),
    }, |args| -> ToolResult {
        let (w,s,e,n) = crate::tile_bounds(args["x"].as_u64().unwrap_or(0) as u32, args["y"].as_u64().unwrap_or(0) as u32, args["z"].as_u64().unwrap_or(0) as u8);
        Ok(serde_json::json!({"west":w,"south":s,"east":e,"north":n}))
    });
    registry.register_tool_sync("tile", ToolDef {
        name: "tile_url".into(), description: "Get tile URL for OSM/Gaode/Tianditu".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"source":{"type":"string","description":"osm|gaode|tianditu"},"x":{"type":"integer"},"y":{"type":"integer"},"z":{"type":"integer"}},"required":["source","x","y","z"]}),
    }, |args| -> ToolResult {
        let src = match args["source"].as_str().unwrap_or("osm") { "gaode"=>crate::TileSource::Gaode, "tianditu"=>crate::TileSource::TianDiTu, _=>crate::TileSource::OpenStreetMap };
        Ok(serde_json::json!({"url":crate::tile_url(src, args["x"].as_u64().unwrap_or(0) as u32, args["y"].as_u64().unwrap_or(0) as u32, args["z"].as_u64().unwrap_or(0) as u8)}))
    });
}
