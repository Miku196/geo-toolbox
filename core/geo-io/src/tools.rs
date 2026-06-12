//! Tool registration for PluginRegistry — CRS + Ingest tools.
//!
//! Extracted from CLI build_registry() to give geo-io locality
//! over CRS transform, NMEA parsing, and CamoFox import tools.

use geo_registry::PluginRegistry;
use geo_registry::registry::{ToolDef, ToolResult};
use geo_core::plugin::PluginCategory;

/// Register CRS + Ingest tools into the PluginRegistry.
pub fn register_tools(registry: &mut PluginRegistry) {
    // ── CRS ──
    registry.register(geo_core::plugin::PluginMeta {
        name: "crs".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        description: "CRS coordinate reference system registry".into(),
        category: PluginCategory::Process,
        healthy: true,
        extra: serde_json::json!({}),
    });

    registry.register_tool_sync("crs", ToolDef {
        name: "crs_list".into(),
        description: "List all registered coordinate reference systems".into(),
        input_schema: serde_json::json!({"type":"object","properties":{},"required":[]}),
    }, |_args| -> ToolResult {
        let list: Vec<serde_json::Value> = geo_core::crs::CrsRegistry::new()
            .list()
            .map(|c| serde_json::json!({
                "epsg": c.epsg,
                "name": c.name,
                "category": format!("{:?}", c.category),
                "proj4": c.proj4,
            }))
            .collect();
        Ok(serde_json::json!(list))
    });

    registry.register_tool_sync("crs", ToolDef {
        name: "crs_transform".into(),
        description: "Transform coordinates between CRS".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "from_epsg": {"type": "integer"},
                "to_epsg": {"type": "integer"},
                "x": {"type": "number"},
                "y": {"type": "number"},
            },
            "required": ["from_epsg", "to_epsg", "x", "y"],
        }),
    }, |args| -> ToolResult {
        let reg = geo_core::crs::CrsRegistry::new();
        let from = args["from_epsg"].as_u64().unwrap_or(4326) as u16;
        let to = args["to_epsg"].as_u64().unwrap_or(4326) as u16;
        let x = args["x"].as_f64().unwrap_or(0.0);
        let y = args["y"].as_f64().unwrap_or(0.0);
        let (ox, oy) = reg.transform_point(from, to, x, y)
            .map_err(|e| geo_core::GeoError::CrsTransform(e.to_string()))?;
        Ok(serde_json::json!({
            "from_epsg": from, "to_epsg": to,
            "input": [x, y], "output": [ox, oy],
            "message": format!("EPSG:{from} ({x}, {y}) → EPSG:{to} ({ox:.4}, {oy:.4})"),
        }))
    });

    // ── Ingest ──
    registry.register(geo_core::plugin::PluginMeta {
        name: "ingest".into(),
        version: "0.1.0".into(),
        description: "Data ingestion (CamoFox, NMEA)".into(),
        category: PluginCategory::Ingest,
        healthy: true,
        extra: serde_json::json!({}),
    });

    registry.register_tool_async("ingest", ToolDef {
        name: "ingest_camofox".into(),
        description: "Parse a CamoFox JSON file and return records".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {"file": {"type": "string"}},
            "required": ["file"],
        }),
    }, |args| Box::pin(async move {
        let file = args["file"].as_str().unwrap_or("");
        let content = tokio::fs::read_to_string(file).await
            .map_err(geo_core::GeoError::from)?;
        let (_rows, result) = crate::camofox::parse_camofox_file(&content, file)
            .map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(serde_json::json!({
            "accepted": result.accepted,
            "rejected": result.rejected,
            "file": file,
        }))
    }));

    registry.register_tool_async("ingest", ToolDef {
        name: "ingest_nmea".into(),
        description: "Parse an NMEA GPS log file and return fixes".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {"file": {"type": "string"}},
            "required": ["file"],
        }),
    }, |args| Box::pin(async move {
        let file = args["file"].as_str().unwrap_or("");
        let content = tokio::fs::read_to_string(file).await
            .map_err(geo_core::GeoError::from)?;
        let mut fixes = 0u32;
        let mut records: Vec<serde_json::Value> = Vec::new();
        for line in content.lines().filter(|l| !l.trim().is_empty()) {
            if let Ok(msg) = crate::nmea::parse_nmea_line(line.trim()) {
                use crate::nmea::NmeaMessage;
                match msg {
                    NmeaMessage::Gga(fix) => {
                        records.push(serde_json::json!({
                            "type": "GGA", "time": fix.time,
                            "lat": fix.lat, "lng": fix.lng,
                            "quality": fix.quality, "satellites": fix.satellites,
                        }));
                        fixes += 1;
                    }
                    NmeaMessage::Rmc(rmc) => {
                        records.push(serde_json::json!({
                            "type": "RMC", "time": rmc.time,
                            "lat": rmc.lat, "lng": rmc.lng,
                            "speed_knots": rmc.speed_knots,
                        }));
                        fixes += 1;
                    }
                    _ => {}
                }
            }
        }
        Ok(serde_json::json!({
            "total_fixes": fixes,
            "records": records.iter().take(10).cloned().collect::<Vec<_>>(),
        }))
    }));
}
