//! Tool registration — Coastal plugin.
use geo_core::plugin::PluginCategory;
use geo_registry::PluginRegistry;
use geo_registry::registry::{ToolDef, ToolResult};
pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta { name: "coastal".into(), version: "0.1.0".into(), description: "Coastal change monitoring: erosion + inundation".into(), category: PluginCategory::Process, healthy: true, extra: serde_json::json!({}) });
    registry.register_tool_sync("coastal", ToolDef { name: "coastal_shoreline".into(), description: "Assess shoreline erosion and inundation between two periods".into(), input_schema: serde_json::json!({"type":"object","properties":{"aoi_name":{"type":"string"},"aoi_geojson":{"type":"string"},"dem_data":{"type":"array","items":{"type":"number"}},"ndvi_old":{"type":"array","items":{"type":"number"}},"ndvi_new":{"type":"array","items":{"type":"number"}},"cols":{"type":"integer"},"rows":{"type":"integer"},"baseline_year":{"type":"integer"},"assessment_year":{"type":"integer"},"erosion_threshold_m":{"type":"number"},"nodata":{"type":"number"}},"required":["aoi_name","dem_data","ndvi_old","ndvi_new","cols","rows","baseline_year","assessment_year"]}) }, |args| -> ToolResult {
        use geo_raster::RasterBand;
        let nd=args["nodata"].as_f64().unwrap_or(-999.0);let c=args["cols"].as_u64().unwrap_or(1) as usize;let r=args["rows"].as_u64().unwrap_or(1) as usize;
        let mk=|k:&str,l:&str|{let v:Vec<f64>=args[k].as_array().unwrap_or(&vec![]).iter().filter_map(|x|x.as_f64()).collect();RasterBand::new(l,c,r,v,nd)};
        let report=crate::CoastalPlugin::new().assess_shoreline(args["aoi_name"].as_str().unwrap_or(""),args["aoi_geojson"].as_str().unwrap_or(""),&mk("dem_data","dem"),&mk("ndvi_old","o"),&mk("ndvi_new","n"),args["baseline_year"].as_u64().unwrap_or(2015) as u16,args["assessment_year"].as_u64().unwrap_or(2025) as u16,args["erosion_threshold_m"].as_f64().unwrap_or(1.0))?;
        Ok(serde_json::to_value(report).map_err(geo_core::GeoError::Serde)?)
    });
}
