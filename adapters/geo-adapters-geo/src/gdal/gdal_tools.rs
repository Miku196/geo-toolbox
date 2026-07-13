//! Tool registration — GDAL CLI.
use geo_registry::{register_plugin, PluginRegistry};
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "cli", "GDAL CLI: COG, warp, translate, ogr2ogr", PluginCategory::Adapter, [
        async "cli_cog_convert" => "Convert raster to COG via gdal_translate" ; serde_json::json!({"type":"object","properties":{"input":{"type":"string"},"output":{"type":"string"},"compression":{"type":"string","default":"DEFLATE"}},"required":["input","output"]}) => |args| Box::pin(async move {
        let compression = args["compression"].as_str().unwrap_or("DEFLATE").to_string();
        let opts = super::gdal_raster::CogOptions { compression, ..Default::default() };
        let path = super::RasterOps::to_cog(args["input"].as_str().unwrap_or(""), args["output"].as_str().unwrap_or(""), Some(opts)).await.map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(serde_json::json!({"output":path.to_string_lossy()}))
    }),
        async "cli_ogr2ogr" => "Convert vector format via ogr2ogr" ; serde_json::json!({"type":"object","properties":{"input":{"type":"string"},"output":{"type":"string"},"epsg":{"type":"integer"},"overwrite":{"type":"boolean"}},"required":["input","output"]}) => |args| Box::pin(async move {
        let opts = super::gdal_vector::Ogr2OgrOptions { target_epsg: args["epsg"].as_u64().map(|v| v as u16), overwrite: args["overwrite"].as_bool().unwrap_or(false), ..Default::default() };
        let path = super::VectorOps::convert(args["input"].as_str().unwrap_or(""), args["output"].as_str().unwrap_or(""), Some(opts)).await.map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(serde_json::json!({"output":path.to_string_lossy()}))
    }),
        async "cli_gdalwarp" => "Reproject/resample/clip raster via gdalwarp" ; serde_json::json!({"type":"object","properties":{"input":{"type":"string"},"output":{"type":"string"},"target_epsg":{"type":"integer"},"resolution_x":{"type":"number"},"resolution_y":{"type":"number"},"resampling":{"type":"string","default":"bilinear"},"cutline_path":{"type":"string"}},"required":["input","output"]}) => |args| Box::pin(async move {
        let mut opts = super::gdal_raster::GdalWarpOptions::default();
        if let Some(epsg) = args["target_epsg"].as_u64() { opts.target_epsg = Some(epsg as u16); }
        if let (Some(rx), Some(ry)) = (args["resolution_x"].as_f64(), args["resolution_y"].as_f64()) { opts.resolution = Some((rx, ry)); }
        if let Some(rs) = args["resampling"].as_str() { opts.resampling = match rs { "nearest"=>super::gdal_raster::ResamplingMethod::Nearest,"bilinear"=>super::gdal_raster::ResamplingMethod::Bilinear,"cubic"=>super::gdal_raster::ResamplingMethod::Cubic,"lanczos"=>super::gdal_raster::ResamplingMethod::Lanczos,"average"=>super::gdal_raster::ResamplingMethod::Average,_=>super::gdal_raster::ResamplingMethod::Bilinear }; }
        if let Some(cut) = args["cutline_path"].as_str() { opts.cutline = Some(std::path::PathBuf::from(cut)); }
        let path = super::RasterOps::gdalwarp(args["input"].as_str().unwrap_or(""), args["output"].as_str().unwrap_or(""), opts).await.map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(serde_json::json!({"output":path.to_string_lossy()}))
    }),
        async "cli_gdal_translate" => "Translate raster format/bands/scale via gdal_translate" ; serde_json::json!({"type":"object","properties":{"input":{"type":"string"},"output":{"type":"string"},"driver":{"type":"string","default":"COG"},"band":{"type":"integer"},"scale_min":{"type":"number"},"scale_max":{"type":"number"},"out_min":{"type":"number"},"out_max":{"type":"number"}},"required":["input","output"]}) => |args| Box::pin(async move {
        let mut opts = super::gdal_raster::GdalTranslateOptions::default();
        if let Some(drv) = args["driver"].as_str() { opts.driver = match drv { "GTiff"=>super::gdal_raster::OutputDriver::GeoTiff,"PNG"=>super::gdal_raster::OutputDriver::Png,"JP2"=>super::gdal_raster::OutputDriver::Jp2,"netCDF"=>super::gdal_raster::OutputDriver::NetCdf,_=>super::gdal_raster::OutputDriver::Cog }; }
        if let Some(b) = args["band"].as_u64() { opts.bands = Some(vec![b as u16]); }
        if let (Some(smin), Some(smax), Some(omin), Some(omax)) = (args["scale_min"].as_f64(), args["scale_max"].as_f64(), args["out_min"].as_f64(), args["out_max"].as_f64()) { opts.scale = Some((smin, smax, omin, omax)); }
        let path = super::RasterOps::gdal_translate(args["input"].as_str().unwrap_or(""), args["output"].as_str().unwrap_or(""), opts).await.map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(serde_json::json!({"output":path.to_string_lossy()}))
    })]);
}
