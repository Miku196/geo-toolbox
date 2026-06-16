//! Tool registration — Carbon plugin.
use crate::ccer::{CcerMethodology, CcerReport};
use crate::plume::{GaussianPlume, StabilityClass};
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
    },
        sync "gaussian_plume" => "Gaussian plume dispersion model" ; serde_json::json!({"type":"object","properties":{"emission_rate_g_s":{"type":"number"},"wind_speed_m_s":{"type":"number"},"stability":{"type":"string","default":"D"},"source_height_m":{"type":"number"},"distance_m":{"type":"number"}},"required":["emission_rate_g_s","wind_speed_m_s","source_height_m","distance_m"]}) => |args| -> ToolResult {
        let stab = match args["stability"].as_str().unwrap_or("D") {
            "A" => StabilityClass::A, "B" => StabilityClass::B, "C" => StabilityClass::C,
            "E" => StabilityClass::E, "F" => StabilityClass::F, _ => StabilityClass::D,
        };
        let plume = GaussianPlume::new(args["emission_rate_g_s"].as_f64().unwrap_or(0.0), args["wind_speed_m_s"].as_f64().unwrap_or(0.0), stab, args["source_height_m"].as_f64().unwrap_or(0.0));
        let c = plume.downwind_concentration_mg_m3(args["distance_m"].as_f64().unwrap_or(0.0));
        let c_g = c / 1000.0;
        Ok(serde_json::json!({"concentration_g_m3": c_g, "concentration_mg_m3": c, "distance_m": args["distance_m"].as_f64().unwrap_or(0.0), "stability": stab.as_str()}))
    },
        sync "ccer_report" => "CCER project design document report" ; serde_json::json!({"type":"object","properties":{"project_name":{"type":"string"},"methodology":{"type":"string","default":"afforestation"},"baseline_tco2e":{"type":"number"},"project_tco2e":{"type":"number"},"leakage_tco2e":{"type":"number","default":0}},"required":["project_name","baseline_tco2e","project_tco2e"]}) => |args| -> ToolResult {
        let method = match args["methodology"].as_str().unwrap_or("afforestation") {
            "forest_mgmt" => CcerMethodology::ForestMgmtMr,
            "renewable" => CcerMethodology::RenewableMr,
            "industrial_eff" => CcerMethodology::IndustrialEffMr,
            "waste_recovery" => CcerMethodology::WasteRecoveryMr,
            _ => CcerMethodology::AfforestationMr,
        };
        let report = CcerReport::new(args["project_name"].as_str().unwrap_or(""), &method, args["baseline_tco2e"].as_f64().unwrap_or(0.0), args["project_tco2e"].as_f64().unwrap_or(0.0)).with_leakage(args["leakage_tco2e"].as_f64().unwrap_or(0.0));
        Ok(report.summary_json())
    }]);
}
