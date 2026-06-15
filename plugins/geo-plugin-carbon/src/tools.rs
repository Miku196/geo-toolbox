//! Tool registration — Carbon plugin.
use crate::{CarbonConfig, CarbonPlugin as Cp};
use geo_core::plugin::PluginCategory;
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "carbon", "IPCC Tier 1 carbon accounting", PluginCategory::Carbon, [
        sync "carbon_calculate_geojson" => "Calculate carbon from GeoJSON FeatureCollection" ; serde_json::json!({"type":"object","properties":{"geojson":{"type":"string"},"year":{"type":"integer"}},"required":["geojson","year"]}) => |args| -> ToolResult {
        let plugin = Cp::load(CarbonConfig::default());
        let report = plugin.calculate_from_geojson(args["geojson"].as_str().unwrap_or(""), args["year"].as_u64().unwrap_or(2025) as u16)?;
        Ok(serde_json::to_value(report).map_err(geo_core::GeoError::Serde)?)
    },
        sync "carbon_pool_stock" => "5-pool carbon stock model (AGB/BGB/Deadwood/Litter/SOC)" ; serde_json::json!({"type":"object","properties":{"area_ha":{"type":"number"},"stem_volume_m3_ha":{"type":"number"},"ecozone":{"type":"string","default":"temperate_broadleaf"},"soc_ref":{"type":"number","default":70}},"required":["area_ha","stem_volume_m3_ha"]}) => |args| -> ToolResult {
        use geo_carbon_math::{BiomassParams, CarbonEngine, SocParams};
        let engine = CarbonEngine::new();
        let bp = match args["ecozone"].as_str().unwrap_or("temperate_broadleaf") {
            "tropical_moist" => BiomassParams::tropical_moist(),
            "tropical_dry" => BiomassParams { wood_density:0.65, bef:2.0, carbon_fraction:0.47, root_shoot_ratio:0.28, deadwood_ratio:0.10, litter_ratio:0.03, litter_turnover:0.55, deadwood_decay_rate:0.09 },
            "temperate_coniferous" => BiomassParams::temperate_coniferous(),
            "boreal" => BiomassParams::boreal(),
            _ => BiomassParams::temperate_broadleaf(),
        };
        let soc = SocParams::native_forest(args["soc_ref"].as_f64().unwrap_or(70.0));
        let stock = engine.calculate_pool_stock(args["area_ha"].as_f64().unwrap_or(1.0), args["stem_volume_m3_ha"].as_f64().unwrap_or(0.0), &bp, &soc);
        Ok(serde_json::to_value(stock).map_err(geo_core::GeoError::Serde)?)
    },
        sync "carbon_scenario" => "Carbon scenario (A/R, IFM, Deforestation) with 5-pool model" ; serde_json::json!({"type":"object","properties":{"scenario":{"type":"string"},"area_ha":{"type":"number"},"before_class":{"type":"string"},"after_class":{"type":"string"},"before_stem_volume":{"type":"number","default":0},"after_stem_volume":{"type":"number"},"ecozone":{"type":"string","default":"temperate_broadleaf"},"time_horizon_years":{"type":"number","default":30}},"required":["scenario","area_ha","before_class","after_class","after_stem_volume"]}) => |args| -> ToolResult {
        use geo_carbon_math::{CarbonEngine, CarbonScenario, EcoZone, LandState, ScenarioInput};
        let engine = CarbonEngine::new();
        let scenario = match args["scenario"].as_str().unwrap_or("") { "ifm" => CarbonScenario::IFM, "deforestation" => CarbonScenario::Deforestation, _ => CarbonScenario::Afforestation };
        let ez = match args["ecozone"].as_str().unwrap_or("temperate_broadleaf") { "tropical_moist" => EcoZone::TropicalMoist, "tropical_dry" => EcoZone::TropicalDry, "temperate_coniferous" => EcoZone::TemperateConiferous, "boreal" => EcoZone::Boreal, "subtropical_humid" => EcoZone::SubtropicalHumid, _ => EcoZone::TemperateBroadleaf };
        let before = if args["before_stem_volume"].as_f64().unwrap_or(0.0) > 0.0 { LandState::forest(args["before_class"].as_str().unwrap_or(""), args["before_stem_volume"].as_f64().unwrap_or(0.0), ez) } else { LandState::non_forest(args["before_class"].as_str().unwrap_or("")) };
        let after = LandState { landcover_class: args["after_class"].as_str().unwrap_or("").into(), stem_volume_m3_ha: args["after_stem_volume"].as_f64().unwrap_or(0.0), ecozone: ez, biomass_params: None, soc_params: None, years_since_transition: 0.0 };
        let input = ScenarioInput { scenario, area_ha: args["area_ha"].as_f64().unwrap_or(0.0), before, after, time_horizon_years: args["time_horizon_years"].as_f64().unwrap_or(30.0), methodology: String::new() };
        let result = engine.calculate_scenario(&input);
        Ok(serde_json::to_value(result).map_err(geo_core::GeoError::Serde)?)
    },
        sync "carbon_vcs" => "VCS/CCB methodology match for carbon scenario" ; serde_json::json!({"type":"object","properties":{"scenario":{"type":"string"}},"required":["scenario"]}) => |args| -> ToolResult {
        use geo_carbon_math::{CarbonEngine, CarbonScenario};
        let engine = CarbonEngine::new();
        let scenario = match args["scenario"].as_str().unwrap_or("") { "ifm" => CarbonScenario::IFM, "deforestation" => CarbonScenario::Deforestation, _ => CarbonScenario::Afforestation };
        match engine.match_vcs_methodology(scenario) {
            Some(s) => Ok(serde_json::to_value(s).map_err(geo_core::GeoError::Serde)?),
            None => Ok(serde_json::json!({"error": "no methodology found"}))
        }
    }]);
}
