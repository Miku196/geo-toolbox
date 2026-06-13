//! Tool registration — GDAL CLI.
use geo_core::plugin::PluginCategory;
use geo_registry::registry::ToolDef;
use geo_registry::PluginRegistry;
pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta {
        name: "cli".into(),
        version: "0.1.0".into(),
        description: "GDAL CLI: COG, warp, translate, ogr2ogr".into(),
        category: PluginCategory::Adapter,
        healthy: true,
        extra: serde_json::json!({"endpoint":"gdal_translate,gdalwarp"}),
    });
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
    registry.register_tool_async("cli", ToolDef { name: "cli_gdalwarp".into(), description: "Reproject/resample/clip raster via gdalwarp".into(), input_schema: serde_json::json!({"type":"object","properties":{"input":{"type":"string"},"output":{"type":"string"},"target_epsg":{"type":"integer"},"resolution_x":{"type":"number"},"resolution_y":{"type":"number"},"resampling":{"type":"string","default":"bilinear"},"cutline_path":{"type":"string"}},"required":["input","output"]}) }, |args| Box::pin(async move {
        let mut opts = crate::raster::GdalWarpOptions::default();
        if let Some(epsg) = args["target_epsg"].as_u64() { opts.target_epsg = Some(epsg as u16); }
        if let (Some(rx), Some(ry)) = (args["resolution_x"].as_f64(), args["resolution_y"].as_f64()) { opts.resolution = Some((rx, ry)); }
        if let Some(rs) = args["resampling"].as_str() {
            opts.resampling = match rs {
                "nearest" => crate::raster::ResamplingMethod::Nearest,
                "bilinear" => crate::raster::ResamplingMethod::Bilinear,
                "cubic" => crate::raster::ResamplingMethod::Cubic,
                "lanczos" => crate::raster::ResamplingMethod::Lanczos,
                "average" => crate::raster::ResamplingMethod::Average,
                _ => crate::raster::ResamplingMethod::Bilinear,
            };
        }
        if let Some(cut) = args["cutline_path"].as_str() { opts.cutline = Some(std::path::PathBuf::from(cut)); }
        let path = crate::RasterOps::gdalwarp(args["input"].as_str().unwrap_or(""), args["output"].as_str().unwrap_or(""), opts).await.map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(serde_json::json!({"output":path.to_string_lossy()}))
    }));
    registry.register_tool_async("cli", ToolDef { name: "cli_gdal_translate".into(), description: "Translate raster format/bands/scale via gdal_translate".into(), input_schema: serde_json::json!({"type":"object","properties":{"input":{"type":"string"},"output":{"type":"string"},"driver":{"type":"string","default":"COG"},"band":{"type":"integer"},"scale_min":{"type":"number"},"scale_max":{"type":"number"},"out_min":{"type":"number"},"out_max":{"type":"number"}},"required":["input","output"]}) }, |args| Box::pin(async move {
        let mut opts = crate::raster::GdalTranslateOptions::default();
        if let Some(drv) = args["driver"].as_str() {
            opts.driver = match drv {
                "GTiff" => crate::raster::OutputDriver::GeoTiff,
                "PNG" => crate::raster::OutputDriver::Png,
                "JP2" => crate::raster::OutputDriver::Jp2,
                "netCDF" => crate::raster::OutputDriver::NetCdf,
                _ => crate::raster::OutputDriver::Cog,
            };
        }
        if let Some(b) = args["band"].as_u64() { opts.bands = Some(vec![b as u16]); }
        if let (Some(smin), Some(smax), Some(omin), Some(omax)) = (args["scale_min"].as_f64(), args["scale_max"].as_f64(), args["out_min"].as_f64(), args["out_max"].as_f64()) {
            opts.scale = Some((smin, smax, omin, omax));
        }
        let path = crate::RasterOps::gdal_translate(args["input"].as_str().unwrap_or(""), args["output"].as_str().unwrap_or(""), opts).await.map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(serde_json::json!({"output":path.to_string_lossy()}))
    }));
}
