//! Tool registration — Forestry plugin.
use geo_registry::register_plugin;
use geo_registry::registry::ToolResult;
use geo_registry::PluginRegistry;
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "forestry", "Forest carbon stock assessment (IPCC biomass)", PluginCategory::Carbon, [
        sync "forestry_carbon_stock" => "Assess forest carbon stock change between two periods" ; serde_json::json!({"type":"object","properties":{"aoi_name":{"type":"string"},"aoi_geojson":{"type":"string"},"red_old":{"type":"array","items":{"type":"number"}},"nir_old":{"type":"array","items":{"type":"number"}},"red_new":{"type":"array","items":{"type":"number"}},"nir_new":{"type":"array","items":{"type":"number"}},"cols":{"type":"integer"},"rows":{"type":"integer"},"year_old":{"type":"integer"},"year_new":{"type":"integer"},"baseline_area_ha":{"type":"number"},"baseline_volume_m3_ha":{"type":"number"},"nodata":{"type":"number"}},"required":["aoi_name","red_old","nir_old","red_new","nir_new","cols","rows","year_old","year_new"]}) => |args| -> ToolResult {
        use geo_raster::RasterBand;
        let nd=args["nodata"].as_f64().unwrap_or(-999.0);let c=args["cols"].as_u64().unwrap_or(1) as usize;let r=args["rows"].as_u64().unwrap_or(1) as usize;
        let mk=|k:&str,l:&str|{let v:Vec<f64>=args[k].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|x|x.as_f64()).collect();RasterBand::new(l,c,r,v,nd)};
        let result=crate::ForestryPlugin::new(Default::default()).assess_carbon_stock(args["aoi_name"].as_str().unwrap_or(""),args["aoi_geojson"].as_str().unwrap_or(""),&mk("red_old","r0"),&mk("nir_old","n0"),&mk("red_new","r1"),&mk("nir_new","n1"),args["year_old"].as_u64().unwrap_or(2020) as u16,args["year_new"].as_u64().unwrap_or(2025) as u16,args["baseline_area_ha"].as_f64().unwrap_or(100.0),args["baseline_volume_m3_ha"].as_f64().unwrap_or(200.0))?;
        serde_json::to_value(result).map_err(geo_core::GeoError::Serde)
    },
        sync "forestry_site_index" => "Forest site index (Richards/Logistic growth curves)" ; serde_json::json!({"type":"object","properties":{"species":{"type":"string","enum":["pinus_massoniana","cunninghamia","eucalyptus","quercus","poplar","larix"]},"site_class":{"type":"string","enum":["I","II","III","IV","V"]},"base_age":{"type":"integer","default":20}},"required":["species","site_class"]}) => |args| -> ToolResult {
        let si = crate::site_index::site_index_lookup(args["species"].as_str().unwrap_or(""), args["site_class"].as_str().unwrap_or("III"), args["base_age"].as_u64().unwrap_or(20) as u32);
        Ok(serde_json::json!({"site_index_m": si}))
    },
        sync "forestry_site_class" => "Determine site class from tree height and age" ; serde_json::json!({"type":"object","properties":{"measured_height_m":{"type":"number"},"age":{"type":"number"},"species":{"type":"string","enum":["pinus_massoniana","cunninghamia","eucalyptus","quercus","poplar","larix"]}},"required":["measured_height_m","age","species"]}) => |args| -> ToolResult {
        Ok(crate::site_index::site_class_from_height(args["measured_height_m"].as_f64().unwrap_or(0.0), args["age"].as_f64().unwrap_or(20.0), args["species"].as_str().unwrap_or("")))
    },
        sync "forestry_harvest_selective" => "Selective harvest simulation (basal area removal)" ; serde_json::json!({"type":"object","properties":{"basal_area_before_m2_ha":{"type":"number"},"harvest_intensity_pct":{"type":"number"},"min_dbh_cm":{"type":"number","default":10}},"required":["basal_area_before_m2_ha","harvest_intensity_pct"]}) => |args| -> ToolResult {
        Ok(crate::harvest::selective_harvest(args["basal_area_before_m2_ha"].as_f64().unwrap_or(0.0), args["harvest_intensity_pct"].as_f64().unwrap_or(0.0), args["min_dbh_cm"].as_f64().unwrap_or(10.0)))
    },
        sync "forestry_harvest_clearcut" => "Clearcut harvest simulation" ; serde_json::json!({"type":"object","properties":{"area_ha":{"type":"number"},"volume_m3_ha":{"type":"number"},"carbon_stock_tco2e_ha":{"type":"number"}},"required":["area_ha","volume_m3_ha"]}) => |args| -> ToolResult {
        Ok(crate::harvest::clearcut_harvest(args["area_ha"].as_f64().unwrap_or(0.0), args["volume_m3_ha"].as_f64().unwrap_or(0.0), args["carbon_stock_tco2e_ha"].as_f64().unwrap_or(0.0)))
    },
        sync "forestry_sustainable_yield" => "Sustainable yield (AAC) calculation for forest management" ; serde_json::json!({"type":"object","properties":{"area_ha":{"type":"number"},"volume_m3_ha":{"type":"number"},"rotation_yrs":{"type":"integer"},"growth_rate_m3_ha_yr":{"type":"number"}},"required":["area_ha","volume_m3_ha","rotation_yrs"]}) => |args| -> ToolResult {
        let aac = crate::harvest::sustainable_yield(args["area_ha"].as_f64().unwrap_or(0.0), args["volume_m3_ha"].as_f64().unwrap_or(0.0), args["rotation_yrs"].as_u64().unwrap_or(30) as u32, args["growth_rate_m3_ha_yr"].as_f64().unwrap_or(5.0));
        Ok(serde_json::json!({"annual_allowable_cut_m3": aac}))
    },
        sync "forestry_harvest_carbon_impact" => "Carbon debt/payback analysis for forest harvest" ; serde_json::json!({"type":"object","properties":{"area_ha":{"type":"number"},"pre_harvest_carbon_tco2e_ha":{"type":"number"},"harvest_method":{"type":"string","enum":["clearcut","selective"]},"harvest_intensity_pct":{"type":"number"},"regeneration_carbon_tco2e_ha_yr":{"type":"number"},"time_horizon_yrs":{"type":"integer","default":30}},"required":["area_ha","pre_harvest_carbon_tco2e_ha","harvest_method","harvest_intensity_pct","regeneration_carbon_tco2e_ha_yr"]}) => |args| -> ToolResult {
        Ok(crate::harvest::harvest_carbon_impact(args["area_ha"].as_f64().unwrap_or(0.0), args["pre_harvest_carbon_tco2e_ha"].as_f64().unwrap_or(0.0), args["harvest_method"].as_str().unwrap_or("clearcut"), args["harvest_intensity_pct"].as_f64().unwrap_or(100.0), args["regeneration_carbon_tco2e_ha_yr"].as_f64().unwrap_or(0.0), args["time_horizon_yrs"].as_u64().unwrap_or(30) as u32))
    }]);
}
