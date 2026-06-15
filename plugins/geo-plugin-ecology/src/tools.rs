//! Tool registration — Ecology plugin.
use crate::config::EcologyConfig;
use geo_core::plugin::PluginCategory;
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};

fn default_plugin() -> crate::ecology::EcologyPlugin {
    crate::ecology::EcologyPlugin::new(EcologyConfig::default())
}

pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "ecology", "Ecological restoration assessment: NDVI change", PluginCategory::Process, [
        sync "ecology_ndvi_change" => "NDVI change detection from two RasterBand arrays" ; serde_json::json!({"type":"object","properties":{"red_before":{"type":"array","items":{"type":"number"}},"nir_before":{"type":"array","items":{"type":"number"}},"red_after":{"type":"array","items":{"type":"number"}},"nir_after":{"type":"array","items":{"type":"number"}},"cols":{"type":"integer"},"rows":{"type":"integer"},"nodata":{"type":"number","default":-999}},"required":["red_before","nir_before","red_after","nir_after","cols","rows"]}) => |args| -> ToolResult {
            use geo_raster::RasterBand;
            let cols = args["cols"].as_u64().unwrap_or(1) as usize;
            let rows = args["rows"].as_u64().unwrap_or(1) as usize;
            let nodata = args["nodata"].as_f64().unwrap_or(-999.0);
            let red_before: Vec<f64> = args["red_before"].as_array().map(|a| a.iter().map(|v| v.as_f64().unwrap_or(nodata)).collect()).unwrap_or_default();
            let nir_before: Vec<f64> = args["nir_before"].as_array().map(|a| a.iter().map(|v| v.as_f64().unwrap_or(nodata)).collect()).unwrap_or_default();
            let red_after: Vec<f64> = args["red_after"].as_array().map(|a| a.iter().map(|v| v.as_f64().unwrap_or(nodata)).collect()).unwrap_or_default();
            let nir_after: Vec<f64> = args["nir_after"].as_array().map(|a| a.iter().map(|v| v.as_f64().unwrap_or(nodata)).collect()).unwrap_or_default();
            let rb = RasterBand::new("B4_before", cols, rows, red_before, nodata);
            let nb = RasterBand::new("B8_before", cols, rows, nir_before, nodata);
            let ra = RasterBand::new("B4_after", cols, rows, red_after, nodata);
            let na = RasterBand::new("B8_after", cols, rows, nir_after, nodata);
            let p = default_plugin();
            let (prev, curr) = p.detect_ndvi_change(&rb, &nb, &ra, &na).map_err(|e| geo_core::GeoError::from(e))?;
            Ok(serde_json::json!({"mean_ndvi_before": prev.mean_ndvi, "mean_ndvi_after": curr.mean_ndvi, "healthy_ratio_before": prev.healthy_ratio, "healthy_ratio_after": curr.healthy_ratio}))
        },
    ]);
}
