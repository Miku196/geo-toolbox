//! Tool registration — GDAL CLI.
use geo_core::plugin::PluginCategory;
use geo_registry::PluginRegistry;
use geo_registry::registry::ToolDef;
pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta { name: "cli".into(), version: "0.1.0".into(), description: "GDAL CLI: COG conversion, vector format conversion".into(), category: PluginCategory::Adapter, healthy: true, extra: serde_json::json!({"endpoint":"gdal_translate"}) });
    registry.register_tool_async("cli", ToolDef { name: "cli_cog_convert".into(), description: "Convert raster to COG via gdal_translate".into(), input_schema: serde_json::json!({"type":"object","properties":{"input":{"type":"string"},"output":{"type":"string"},"compression":{"type":"string","default":"DEFLATE"}},"required":["input","output"]}) }, |args| Box::pin(async move {
        let compression = args["compression"].as_str().unwrap_or("DEFLATE").to_string();
        let opts = crate::raster::CogOptions { compression, ..Default::default() };
        let path = crate::RasterOps::to_cog(args["input"].as_str().unwrap_or(""), args["output"].as_str().unwrap_or(""), Some(opts)).await.map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(serde_json::json!({"output":path.to_string_lossy()}))
    }));
    registry.register_tool_async("cli", ToolDef { name: "cli_ogr2ogr".into(), description: "Convert vector format via ogr2ogr".into(), input_schema: serde_json::json!({"type":"object","properties":{"input":{"type":"string"},"output":{"type":"string"},"epsg":{"type":"integer"},"overwrite":{"type":"boolean"}},"required":["input","output"]}) }, |args| Box::pin(async move {
        let opts = crate::vector::Ogr2OgrOptions { target_epsg: args["epsg"].as_u64().map(|v| v as u16), overwrite: args["overwrite"].as_bool().unwrap_or(false), ..Default::default() };
        let path = crate::VectorOps::convert(args["input"].as_str().unwrap_or(""), args["output"].as_str().unwrap_or(""), Some(opts)).await.map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(serde_json::json!({"output":path.to_string_lossy()}))
    }));
}
