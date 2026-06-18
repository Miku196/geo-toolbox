//! Tool registration — Forestry plugin.
use geo_registry::register_plugin;
use geo_registry::registry::ToolResult;
use geo_registry::PluginRegistry;
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "forestry", "Forest carbon stock assessment (IPCC biomass)", PluginCategory::Carbon, [
        sync "forestry_carbon_stock" => "Assess forest carbon stock change between two periods" ; serde_json::json!({"type":"object","properties":{"aoi_name":{"type":"string"},"aoi_geojson":{"type":"string"},"red_old":{"type":"array","items":{"type":"number"}},"nir_old":{"type":"array","items":{"type":"number"}},"red_new":{"type":"array","items":{"type":"number"}},"nir_new":{"type":"array","items":{"type":"number"}},"cols":{"type":"integer"},"rows":{"type":"integer"},"year_old":{"type":"integer"},"year_new":{"type":"integer"},"baseline_area_ha":{"type":"number"},"baseline_volume_m3_ha":{"type":"number"},"nodata":{"type":"number"}},"required":["aoi_name","red_old","nir_old","red_new","nir_new","cols","rows","year_old","year_new"]}) => |args| -> ToolResult {
        use geo_raster::RasterBand;
        let nd=args["nodata"].as_f64().unwrap_or(-999.0);let c=args["cols"].as_u64().unwrap_or(1) as usize;let r=args["rows"].as_u64().unwrap_or(1) as usize;
        let mk=|k:&str,l:&str|{let v:Vec<f64>=args[k].as_array().unwrap_or(&vec![]).iter().filter_map(|x|x.as_f64()).collect();RasterBand::new(l,c,r,v,nd)};
        let result=crate::ForestryPlugin::new(Default::default()).assess_carbon_stock(args["aoi_name"].as_str().unwrap_or(""),args["aoi_geojson"].as_str().unwrap_or(""),&mk("red_old","r0"),&mk("nir_old","n0"),&mk("red_new","r1"),&mk("nir_new","n1"),args["year_old"].as_u64().unwrap_or(2020) as u16,args["year_new"].as_u64().unwrap_or(2025) as u16,args["baseline_area_ha"].as_f64().unwrap_or(100.0),args["baseline_volume_m3_ha"].as_f64().unwrap_or(200.0))?;
        serde_json::to_value(result).map_err(geo_core::GeoError::Serde)
    }]);
}
