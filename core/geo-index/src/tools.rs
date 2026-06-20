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
    register_plugin!(registry, "index", "H3 spatial index: lat/lon → H3, grid disk, cover bbox, neighbors", PluginCategory::Process, [
        sync "h3_latlon_to_h3" => "Convert lat/lon to H3 index at given resolution" ; serde_json::json!({"type":"object","properties":{"lat":{"type":"number"},"lon":{"type":"number"},"resolution":{"type":"integer","minimum":0,"maximum":15}},"required":["lat","lon","resolution"]}) => |args| -> ToolResult {
        let lat = args["lat"].as_f64().unwrap_or(0.0);
        let lon = args["lon"].as_f64().unwrap_or(0.0);
        let res = args["resolution"].as_u64().unwrap_or(4) as u8;
        match crate::h3::latlon_to_h3(lat, lon, res) {
            Some(idx) => {
                let (clat, clon) = idx.to_latlon();
                Ok(serde_json::json!({"i":idx.i,"j":idx.j,"resolution":idx.resolution,"center_lat":clat,"center_lon":clon,"hex_str":crate::h3::h3_to_string(&idx)}))
            },
            None => Err(geo_core::GeoError::invalid_input("resolution", "must be 0-15"))
        }
    },
        sync "h3_hex_boundary" => "Get GeoJSON boundary of an H3 hexagon" ; serde_json::json!({"type":"object","properties":{"i":{"type":"integer"},"j":{"type":"integer"},"resolution":{"type":"integer","minimum":0,"maximum":15}},"required":["i","j","resolution"]}) => |args| -> ToolResult {
        let idx = crate::h3::H3Index {
            i: args["i"].as_i64().unwrap_or(0),
            j: args["j"].as_i64().unwrap_or(0),
            resolution: args["resolution"].as_u64().unwrap_or(4) as u8,
        };
        Ok(crate::h3::h3_to_geojson(&idx))
    },
        sync "h3_neighbors" => "Get 6 neighbors of an H3 hexagon" ; serde_json::json!({"type":"object","properties":{"i":{"type":"integer"},"j":{"type":"integer"},"resolution":{"type":"integer","minimum":0,"maximum":15}},"required":["i","j","resolution"]}) => |args| -> ToolResult {
        let idx = crate::h3::H3Index {
            i: args["i"].as_i64().unwrap_or(0),
            j: args["j"].as_i64().unwrap_or(0),
            resolution: args["resolution"].as_u64().unwrap_or(4) as u8,
        };
        let neighbors: Vec<serde_json::Value> = idx.neighbors().iter().map(|n| {
            let (lat, lon) = n.to_latlon();
            serde_json::json!({"i":n.i,"j":n.j,"center_lat":lat,"center_lon":lon})
        }).collect();
        Ok(serde_json::json!({"neighbors":neighbors}))
    },
        sync "h3_grid_disk" => "Get all H3 hexagons within radius km of a point" ; serde_json::json!({"type":"object","properties":{"lat":{"type":"number"},"lon":{"type":"number"},"radius_km":{"type":"number","minimum":0},"resolution":{"type":"integer","minimum":0,"maximum":15}},"required":["lat","lon","radius_km","resolution"]}) => |args| -> ToolResult {
        let lat = args["lat"].as_f64().unwrap_or(0.0);
        let lon = args["lon"].as_f64().unwrap_or(0.0);
        let r = args["radius_km"].as_f64().unwrap_or(10.0);
        let res = args["resolution"].as_u64().unwrap_or(4) as u8;
        let hexes = crate::h3::h3_grid_disk(lat, lon, r, res);
        let count = hexes.len();
        let hex_list: Vec<serde_json::Value> = hexes.iter().map(|h| {
            let (clat, clon) = h.to_latlon();
            serde_json::json!({"i":h.i,"j":h.j,"center_lat":clat,"center_lon":clon})
        }).take(1000).collect();
        Ok(serde_json::json!({"count":count,"hexagons":hex_list,"truncated": count > 1000}))
    },
        sync "h3_cover_bbox" => "Cover a bounding box with H3 hexagons" ; serde_json::json!({"type":"object","properties":{"west":{"type":"number"},"south":{"type":"number"},"east":{"type":"number"},"north":{"type":"number"},"resolution":{"type":"integer","minimum":0,"maximum":15}},"required":["west","south","east","north","resolution"]}) => |args| -> ToolResult {
        let bbox = geo_core::types::BBox::new(
            args["west"].as_f64().unwrap_or(0.0),
            args["south"].as_f64().unwrap_or(0.0),
            args["east"].as_f64().unwrap_or(0.0),
            args["north"].as_f64().unwrap_or(0.0),
        );
        let res = args["resolution"].as_u64().unwrap_or(4) as u8;
        let hexes = crate::h3::h3_cover_bbox(&bbox, res);
        let hex_list: Vec<serde_json::Value> = hexes.iter().map(|h| {
            let (clat, clon) = h.to_latlon();
            serde_json::json!({"i":h.i,"j":h.j,"center_lat":clat,"center_lon":clon})
        }).take(1000).collect();
        Ok(serde_json::json!({"count":hexes.len(),"hexagons":hex_list,"truncated": hexes.len() > 1000}))
    }]);
}
