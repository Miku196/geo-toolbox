//! Tool registration — Agriculture plugin.
use geo_core::plugin::PluginCategory;
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "agri", "Agriculture: crop yield, LAI/NPP, soil rating, irrigation", PluginCategory::Process, [
        sync "agri_yield" => "Estimate crop yield from NDVI and area" ; serde_json::json!({"type":"object","properties":{"area_ha":{"type":"number"},"ndvi_mean":{"type":"number"},"crop_type":{"type":"string","enum":["wheat","corn","rice","soybean"],"default":"wheat"}},"required":["area_ha","ndvi_mean"]}) => |args| -> ToolResult {
        let p = crate::AgriPlugin::new(Default::default());
        let result = p.estimate_yield(args["area_ha"].as_f64().unwrap_or(10.0), args["ndvi_mean"].as_f64().unwrap_or(0.7), args["crop_type"].as_str().unwrap_or("wheat"));
        Ok(serde_json::json!({"yield_kg":result.yield_kg,"yield_casa_kg":result.yield_casa_kg,"yield_simple_kg":result.yield_simple_kg,"lai":result.lai,"npp_gcm2_season":result.npp_gcm2_season,"crop_type":result.crop_type,"area_ha":result.area_ha}))
    },
        sync "agri_soil" => "Comprehensive soil quality rating (0-100)" ; serde_json::json!({"type":"object","properties":{"organic_matter_pct":{"type":"number"},"ph":{"type":"number"},"n_mg_kg":{"type":"number","default":100},"p_mg_kg":{"type":"number","default":15},"k_mg_kg":{"type":"number","default":120},"texture":{"type":"string","enum":["loam","clay_loam","sandy_loam","silt_loam","clay","sand","silt"],"default":"loam"},"drainage_ok":{"type":"boolean","default":true}},"required":["organic_matter_pct","ph"]}) => |args| -> ToolResult {
        let p = crate::AgriPlugin::new(Default::default());
        let result = p.soil_rating_detailed(args["organic_matter_pct"].as_f64().unwrap_or(2.0),args["ph"].as_f64().unwrap_or(6.5),args["n_mg_kg"].as_f64().unwrap_or(100.0),args["p_mg_kg"].as_f64().unwrap_or(15.0),args["k_mg_kg"].as_f64().unwrap_or(120.0),args["texture"].as_str().unwrap_or("loam"),args["drainage_ok"].as_bool().unwrap_or(true));
        Ok(serde_json::json!({"score":result.score,"grade":result.grade,"om_score":result.om_score,"ph_score":result.ph_score,"n_score":result.n_score,"p_score":result.p_score,"k_score":result.k_score,"texture_score":result.texture_score,"drainage_score":result.drainage_score}))
    },
        sync "agri_lai" => "NDVI -> LAI -> fIPAR -> NPP conversion chain" ; serde_json::json!({"type":"object","properties":{"ndvi":{"type":"number"},"crop_type":{"type":"string","enum":["wheat","corn","rice","soybean"],"default":"wheat"},"par_mj_m2_day":{"type":"number","default":20}},"required":["ndvi"]}) => |args| -> ToolResult {
        let p = crate::AgriPlugin::new(Default::default());
        let ndvi = args["ndvi"].as_f64().unwrap_or(0.7);
        let crop = args["crop_type"].as_str().unwrap_or("wheat");
        let par = args["par_mj_m2_day"].as_f64().unwrap_or(20.0);
        let k = p.config.crops.get(crop).map(|c| c.k).unwrap_or(0.5);
        let lai = p.ndvi_to_lai(ndvi, k);
        let fipar = crate::AgriPlugin::lai_to_fipar(lai);
        let npp = p.estimate_npp(par, ndvi, crop);
        Ok(serde_json::json!({"lai":lai,"fipar":fipar,"npp_gcm2_day":npp/120.0,"npp_gcm2_season":npp,"crop_type":crop,"k":k}))
    },
        sync "agri_irrigation" => "Net & gross irrigation requirement" ; serde_json::json!({"type":"object","properties":{"et0_mm_day":{"type":"number"},"rainfall_mm":{"type":"number"},"crop_type":{"type":"string","enum":["wheat","corn","rice","soybean"],"default":"wheat"}},"required":["et0_mm_day"]}) => |args| -> ToolResult {
        let p = crate::AgriPlugin::new(Default::default());
        let et0 = args["et0_mm_day"].as_f64().unwrap_or(5.0);
        let rain = args["rainfall_mm"].as_f64().unwrap_or(0.0);
        let crop = args["crop_type"].as_str().unwrap_or("wheat");
        let net = p.net_irrigation(et0, crop, rain);
        let gross = p.gross_irrigation(net);
        Ok(serde_json::json!({"net_irrigation_mm":net,"gross_irrigation_mm":gross,"kc":p.config.crops.get(crop).map(|c|c.kc),"crop_type":crop}))
    }]);
}
