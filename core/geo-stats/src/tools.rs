//! Tool registration — Zonal statistics.
use geo_core::plugin::PluginCategory;
use geo_registry::PluginRegistry;
use geo_registry::registry::{ToolDef, ToolResult};

/// Register stats tools into the PluginRegistry.
pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta {
        name: "stats".into(), version: env!("CARGO_PKG_VERSION").into(),
        description: "Zonal statistics: compute raster stats within polygon zones".into(),
        category: PluginCategory::Process, healthy: true, extra: serde_json::json!({}),
    });
    registry.register_tool_sync("stats", ToolDef {
        name: "zonal_stats".into(), description: "Compute zonal statistics for raster data within bboxes".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"zones":{"type":"array"},"raster_data":{"type":"array","items":{"type":"number"}},"raster_cols":{"type":"integer"},"raster_min_x":{"type":"number"},"raster_min_y":{"type":"number"},"raster_max_x":{"type":"number"},"raster_max_y":{"type":"number"},"nodata":{"type":"number"}},"required":["zones","raster_data","raster_cols"]}),
    }, |args| -> ToolResult {
        let empty = vec![];
        let zones_json = args["zones"].as_array().unwrap_or(&empty);
        let data: Vec<f64> = args["raster_data"].as_array().unwrap_or(&vec![]).iter().filter_map(|v| v.as_f64()).collect();
        let cols = args["raster_cols"].as_u64().unwrap_or(1) as usize;
        let rows = data.len()/cols.max(1);
        let nodata = args["nodata"].as_f64().unwrap_or(-999.0);
        let rb = geo_core::types::BBox{min_x:args["raster_min_x"].as_f64().unwrap_or(0.0),min_y:args["raster_min_y"].as_f64().unwrap_or(0.0),max_x:args["raster_max_x"].as_f64().unwrap_or(0.0),max_y:args["raster_max_y"].as_f64().unwrap_or(0.0)};
        let mut results = Vec::new();
        for z in zones_json {
            let zn = z["name"].as_str().unwrap_or("zone");
            let zb = geo_core::types::BBox{min_x:z["min_x"].as_f64().unwrap_or(0.0),min_y:z["min_y"].as_f64().unwrap_or(0.0),max_x:z["max_x"].as_f64().unwrap_or(0.0),max_y:z["max_y"].as_f64().unwrap_or(0.0)};
            let zr = crate::zonal_stats(&data,rows,cols,nodata,rb,&zb,zn).map_err(|e| geo_core::GeoError::Validation(e.to_string()))?;
            results.push(serde_json::json!({"zone":zn,"pixel_count":zr.pixel_count,"mean":zr.mean,"min":zr.min,"max":zr.max,"sum":zr.sum}));
        }
        Ok(serde_json::json!(results))
    });
}
