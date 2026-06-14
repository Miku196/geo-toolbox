//! Tool registration — CRS + Ingest tools (mixed sync/async).
use geo_core::plugin::PluginCategory;
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, register_sync_tools, register_async_tools, PluginRegistry};
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "crs", "CRS coordinate reference system registry", PluginCategory::Process, [
        sync "crs_list" => "List all registered coordinate reference systems" ; serde_json::json!({"type":"object","properties":{"category":{"type":"string"}},"required":[]}) => |args| -> ToolResult {
        let cat_filter = args["category"].as_str();
        let list: Vec<serde_json::Value> = geo_core::crs::CrsRegistry::new().list().filter(|c| match cat_filter { Some(cat) => format!("{:?}", c.category).to_lowercase() == cat.to_lowercase(), None => true })
            .map(|c| serde_json::json!({"epsg":c.epsg,"name":c.name,"category":format!("{:?}",c.category),"proj4":c.proj4,})).collect();
        Ok(serde_json::json!(list))
    },
        sync "crs_transform" => "Transform coordinates between CRS" ; serde_json::json!({"type":"object","properties":{"from_epsg":{"type":"integer"},"to_epsg":{"type":"integer"},"x":{"type":"number"},"y":{"type":"number"}},"required":["from_epsg","to_epsg","x","y"]}) => |args| -> ToolResult {
        let reg = geo_core::crs::CrsRegistry::new();
        let from = args["from_epsg"].as_u64().unwrap_or(4326) as u16;
        let to = args["to_epsg"].as_u64().unwrap_or(4326) as u16;
        let x = args["x"].as_f64().unwrap_or(0.0);
        let y = args["y"].as_f64().unwrap_or(0.0);
        let (ox, oy) = reg.transform_point(from, to, x, y).map_err(|e| geo_core::GeoError::CrsTransform(e.to_string()))?;
        let msg = format!("EPSG:{from} ({x}, {y}) -> EPSG:{to} ({ox:.4}, {oy:.4})");
        Ok(serde_json::json!({"from_epsg":from,"to_epsg":to,"input":[x,y],"output":[ox,oy],"message":msg}))
    }]);
    registry.register(geo_core::plugin::PluginMeta {
        name: "ingest".into(), version: "0.1.0".into(), description: "Data ingestion (CamoFox, NMEA)".into(),
        category: PluginCategory::Ingest, healthy: true, extra: serde_json::json!({}),
    });
    register_sync_tools!(registry, "ingest", [
        "validate_coord" => "Validate a coordinate pair (longitude, latitude)" ; serde_json::json!({"type":"object","properties":{"lon":{"type":"number"},"lat":{"type":"number"}},"required":["lon","lat"]}) => |args| -> ToolResult {
        let lon = args["lon"].as_f64().unwrap_or(999.0);
        let lat = args["lat"].as_f64().unwrap_or(999.0);
        let valid = geo_core::types::validate_coord(lon, lat).is_ok();
        let mut issues = Vec::new();
        if !(-180.0..=180.0).contains(&lon) { issues.push("lon out of range [-180,180]"); }
        if !(-90.0..=90.0).contains(&lat) { issues.push("lat out of range [-90,90]"); }
        Ok(serde_json::json!({"valid":valid,"lon":lon,"lat":lat,"issues":issues}))
    }]);
    register_async_tools!(registry, "ingest", [
        "ingest_camofox" => "Parse a CamoFox JSON file and return records" ; serde_json::json!({"type":"object","properties":{"file":{"type":"string"}},"required":["file"]}) => |args| Box::pin(async move {
        let file = args["file"].as_str().unwrap_or("");
        let content = tokio::fs::read_to_string(file).await.map_err(geo_core::GeoError::from)?;
        let (_rows, result) = crate::camofox::parse_camofox_file(&content, file).map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(serde_json::json!({"accepted":result.accepted,"rejected":result.rejected,"file":file}))
    }),
        "ingest_nmea" => "Parse an NMEA GPS log file and return fixes" ; serde_json::json!({"type":"object","properties":{"file":{"type":"string"}},"required":["file"]}) => |args| Box::pin(async move {
        let file = args["file"].as_str().unwrap_or("");
        let content = tokio::fs::read_to_string(file).await.map_err(geo_core::GeoError::from)?;
        let mut fixes = 0u32;
        let mut records: Vec<serde_json::Value> = Vec::new();
        for line in content.lines().filter(|l| !l.trim().is_empty()) {
            if let Ok(msg) = crate::nmea::parse_nmea_line(line.trim()) {
                use crate::nmea::NmeaMessage;
                match msg {
                    NmeaMessage::Gga(fix) => { records.push(serde_json::json!({"type":"GGA","time":fix.time,"lat":fix.lat,"lng":fix.lng,"quality":fix.quality,"satellites":fix.satellites})); fixes += 1; }
                    NmeaMessage::Rmc(rmc) => { records.push(serde_json::json!({"type":"RMC","time":rmc.time,"lat":rmc.lat,"lng":rmc.lng,"speed_knots":rmc.speed_knots})); fixes += 1; }
                    _ => {}
                }
            }
        }
        Ok(serde_json::json!({"total_fixes":fixes,"records":records.iter().take(10).cloned().collect::<Vec<_>>()}))
    })]);
}
