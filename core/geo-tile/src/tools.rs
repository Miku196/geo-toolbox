//! Tool registration — Tile engine.
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};

/// Register tile-related tools (latlon-to-tile, MVT encode, PMTiles).
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "tile", "Vector tile (MVT) encoder + raster tile (PMTiles)", PluginCategory::Process, [
        sync "tile_latlon_to_tile" => "Convert lat/lon to tile z/x/y" ; serde_json::json!({"type":"object","properties":{"lon":{"type":"number"},"lat":{"type":"number"},"zoom":{"type":"integer"}},"required":["lon","lat","zoom"]}) => |args| -> ToolResult {
        let (x,y,z) = crate::latlon_to_tile(args["lon"].as_f64().unwrap_or(0.0), args["lat"].as_f64().unwrap_or(0.0), args["zoom"].as_u64().unwrap_or(0) as u8);
        Ok(serde_json::json!({"x":x,"y":y,"z":z}))
    },
        sync "tile_bounds" => "Get lat/lon bounds of a tile" ; serde_json::json!({"type":"object","properties":{"x":{"type":"integer"},"y":{"type":"integer"},"z":{"type":"integer"}},"required":["x","y","z"]}) => |args| -> ToolResult {
        let (w,s,e,n) = crate::tile_bounds(args["x"].as_u64().unwrap_or(0) as u32, args["y"].as_u64().unwrap_or(0) as u32, args["z"].as_u64().unwrap_or(0) as u8);
        Ok(serde_json::json!({"west":w,"south":s,"east":e,"north":n}))
    },
        sync "tile_url" => "Get tile URL for OSM/Gaode/Tianditu" ; serde_json::json!({"type":"object","properties":{"source":{"type":"string","description":"osm|gaode|tianditu"},"x":{"type":"integer"},"y":{"type":"integer"},"z":{"type":"integer"}},"required":["source","x","y","z"]}) => |args| -> ToolResult {
        let src = match args["source"].as_str().unwrap_or("osm") { "gaode"=>crate::TileSource::Gaode, "tianditu"=>crate::TileSource::TianDiTu, _=>crate::TileSource::OpenStreetMap };
        Ok(serde_json::json!({"url":crate::tile_url(src, args["x"].as_u64().unwrap_or(0) as u32, args["y"].as_u64().unwrap_or(0) as u32, args["z"].as_u64().unwrap_or(0) as u8)}))
    },
        sync "tile_encode_mvt" => "Encode GeoJSON features to MVT (Mapbox Vector Tile) bytes" ; serde_json::json!({"type":"object","properties":{"layer":{"type":"string"},"features":{"type":"array"},"x":{"type":"integer"},"y":{"type":"integer"},"z":{"type":"integer"},"extent":{"type":"integer","default":4096}},"required":["layer","features","x","y","z"]}) => |args| -> ToolResult {
        let features: Vec<serde_json::Value> = args["features"].as_array().unwrap_or(&vec![]).clone();
        let x = args["x"].as_u64().unwrap_or(0) as u32;
        let y = args["y"].as_u64().unwrap_or(0) as u32;
        let z = args["z"].as_u64().unwrap_or(0) as u8;
        let extent = args["extent"].as_u64().unwrap_or(4096) as u32;
        let encoder = crate::MvtEncoder::new(extent);
        let bytes = encoder.encode_tile(args["layer"].as_str().unwrap_or("default"), &features, x, y, z)
            .map_err(|e| geo_core::GeoError::Validation(e.to_string()))?;
        Ok(serde_json::json!({"byte_count": bytes.len()}))
    }]);
}
