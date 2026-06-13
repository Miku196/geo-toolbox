//! Tool registration — Energy plugin.
use geo_core::plugin::PluginCategory;
use geo_registry::registry::{ToolDef, ToolResult};
use geo_registry::PluginRegistry;
pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta {
        name: "energy".into(),
        version: "0.1.0".into(),
        description: "Solar/wind site suitability assessment".into(),
        category: PluginCategory::Process,
        healthy: true,
        extra: serde_json::json!({}),
    });
    registry.register_tool_sync("energy", ToolDef { name: "energy_solar_suitability".into(), description: "Assess solar site suitability from DEM + radiation".into(), input_schema: serde_json::json!({"type":"object","properties":{"aoi_name":{"type":"string"},"aoi_geojson":{"type":"string"},"dem_data":{"type":"array","items":{"type":"number"}},"radiation_data":{"type":"array","items":{"type":"number"}},"cols":{"type":"integer"},"rows":{"type":"integer"},"nodata":{"type":"number"}},"required":["aoi_name","aoi_geojson","dem_data","radiation_data","cols","rows"]}) }, |args| -> ToolResult {
        use geo_raster::RasterBand;
        let nd=args["nodata"].as_f64().unwrap_or(-999.0); let c=args["cols"].as_u64().unwrap_or(1) as usize; let r=args["rows"].as_u64().unwrap_or(1) as usize;
        let mk=|k:&str,l:&str|{let v:Vec<f64>=args[k].as_array().unwrap_or(&vec![]).iter().filter_map(|x|x.as_f64()).collect();RasterBand::new(l,c,r,v,nd)};
        let result=crate::EnergyPlugin::new(Default::default()).assess_solar(args["aoi_name"].as_str().unwrap_or(""),args["aoi_geojson"].as_str().unwrap_or(""),&mk("dem_data","dem"),&mk("radiation_data","rad"))?;
        Ok(serde_json::to_value(result).map_err(geo_core::GeoError::Serde)?)
    });
}
