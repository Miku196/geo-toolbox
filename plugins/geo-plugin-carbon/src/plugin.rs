//! 碳核算插件主体。
//!
//! 封装 geo-carbon-math 引擎，提供从 GeoJSON + 配置直接输出报告的接口。

use crate::config::CarbonConfig;
use geo_carbon_math::{CarbonEngine, CarbonReport, EmissionFactor, GeoFeature};
use geo_core::errors::{GeoError, GeoResult};
use geo_emission_factors::EfDatabase;

/// 碳核算插件。
pub struct CarbonPlugin {
    config: CarbonConfig,
    engine: CarbonEngine,
    /// 排放因子数据库（可选，优先于配置中的硬编码值）。
    ef_db: Option<EfDatabase>,
}

impl CarbonPlugin {
    /// 从 rules.toml 加载配置。
    pub fn load(config: CarbonConfig) -> Self {
        Self {
            config,
            engine: CarbonEngine::new(),
            ef_db: None,
        }
    }

    /// 设置排放因子数据库。
    pub fn with_ef_db(mut self, ef_db: EfDatabase) -> Self {
        self.ef_db = Some(ef_db);
        self
    }

    /// 构建排放因子列表（优先使用 EF DB，回退到配置中的硬编码值）。
    fn build_factors(&self, area_ha: f64) -> (Vec<EmissionFactor>, String) {
        let defaults = &self.config.carbon;
        let source = &defaults.source;
        let all_classes: [&str; 7] = [
            "forest",
            "grassland",
            "wetland",
            "cropland",
            "built_up",
            "water",
            "bare",
        ];

        if let Some(ref ef_db) = self.ef_db {
            // Tier 2/3: lookup from database
            let factors: Vec<EmissionFactor> = all_classes
                .iter()
                .map(|class| {
                    ef_db
                        .simple_land_use(class, area_ha)
                        .unwrap_or_else(|| EmissionFactor::new(*class, 0.0, "ef-db"))
                })
                .collect();
            (factors, "IPCC Tier 2 — geo-emission-factors".into())
        } else {
            // Tier 1: use config values
            let factors: Vec<EmissionFactor> = all_classes
                .iter()
                .map(|class| {
                    let value = match *class {
                        "forest" => defaults.forest,
                        "grassland" => defaults.grassland,
                        "wetland" => defaults.wetland,
                        "cropland" => defaults.cropland,
                        "built_up" => defaults.built_up,
                        "water" => defaults.water,
                        "bare" => defaults.bare,
                        _ => 0.0,
                    };
                    EmissionFactor::new(*class, value, source.as_str())
                })
                .collect();
            (factors, format!("IPCC Tier 1 — {}", source))
        }
    }

    /// 从 GeoJSON FeatureCollection 计算碳核算。
    ///
    /// Features 的 properties 必须包含 `landcover`/`class`/`category` 之一。
    pub fn calculate_from_geojson(&self, geojson_fc: &str, year: u16) -> GeoResult<CarbonReport> {
        // 1. 解析 GeoJSON features
        let fc: serde_json::Value = serde_json::from_str(geojson_fc)
            .map_err(|e| GeoError::Validation(format!("Invalid GeoJSON: {e}")))?;

        let features_json = fc["features"]
            .as_array()
            .ok_or_else(|| GeoError::invalid_input("aoi_geojson", "missing 'features' array"))?;

        let mut features = Vec::with_capacity(features_json.len());
        let mut total_area_ha = 0.0;
        for f in features_json {
            let feat_str = serde_json::to_string(f).map_err(GeoError::Serde)?;
            match GeoFeature::from_feature_json(&feat_str) {
                Ok(gf) => {
                    total_area_ha += gf.area_ha();
                    features.push(gf);
                }
                Err(_) => continue,
            }
        }

        // 2. 获取排放因子（EF DB 或配置）
        let (factors, methodology) = self.build_factors(total_area_ha);

        // 3. 计算
        let mut report = self
            .engine
            .calculate(&features, &factors, year)
            .map_err(GeoError::Validation)?;

        report.methodology = Some(methodology);
        Ok(report)
    }

    /// Run a 5-pool carbon scenario (A/R, IFM, Deforestation).
    #[allow(clippy::too_many_arguments)]
    pub fn calculate_scenario(
        &self,
        scenario: geo_carbon_math::CarbonScenario,
        area_ha: f64,
        before_class: &str,
        before_stem_volume: f64,
        after_class: &str,
        after_stem_volume: f64,
        ecozone: geo_carbon_math::EcoZone,
        time_horizon_years: f64,
    ) -> GeoResult<geo_carbon_math::ScenarioResult> {
        let before = geo_carbon_math::LandState {
            landcover_class: before_class.to_string(),
            stem_volume_m3_ha: before_stem_volume,
            ecozone,
            biomass_params: None,
            soc_params: None,
            years_since_transition: 0.0,
        };
        let after = geo_carbon_math::LandState {
            landcover_class: after_class.to_string(),
            stem_volume_m3_ha: after_stem_volume,
            ecozone,
            biomass_params: None,
            soc_params: None,
            years_since_transition: 0.0,
        };
        let input = geo_carbon_math::ScenarioInput {
            scenario,
            area_ha,
            before,
            after,
            time_horizon_years,
            methodology: String::new(),
        };
        Ok(self.engine.calculate_scenario(&input))
    }

    /// Compute 5-pool carbon stock for a given stand.
    pub fn calculate_pool_stock(
        &self,
        area_ha: f64,
        stem_volume_m3_ha: f64,
        biomass: &geo_carbon_math::BiomassParams,
        soc: &geo_carbon_math::SocParams,
    ) -> geo_carbon_math::MultiPoolStock {
        self.engine
            .calculate_pool_stock(area_ha, stem_volume_m3_ha, biomass, soc)
    }

    /// Match scenario to best VCS methodology.
    pub fn match_vcs(
        &self,
        scenario: geo_carbon_math::CarbonScenario,
    ) -> Option<geo_carbon_math::VcsProjectSummary> {
        self.engine.match_vcs_methodology(scenario)
    }

    /// 使用外部提供的 features 和 factors 计算。
    pub fn calculate(&self, features: &[GeoFeature], year: u16) -> Result<CarbonReport, String> {
        let area_ha: f64 = features.iter().map(|f| f.area_ha()).sum();
        let (factors, methodology) = self.build_factors(area_ha);

        let mut report = self.engine.calculate(features, &factors, year)?;
        report.methodology = Some(methodology);
        Ok(report)
    }

    /// Monte Carlo uncertainty analysis (±20% factor perturbation, N simulations).
    pub fn monte_carlo_uncertainty(
        &self,
        features: &[GeoFeature],
        year: u16,
        simulations: usize,
    ) -> GeoResult<UncertaintyReport> {
        let area_ha: f64 = features.iter().map(|f| f.area_ha()).sum();
        let (base_factors, _methodology) = self.build_factors(area_ha);
        let mut totals: Vec<f64> = Vec::with_capacity(simulations);
        let mut seed = year as u64;

        for _ in 0..simulations {
            let factors: Vec<EmissionFactor> = base_factors
                .iter()
                .map(|ef| {
                    seed = seed
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    let noise = ((seed as f64 / u64::MAX as f64) - 0.5) * 0.4;
                    let perturbed = ef.factor_value * (1.0 + noise);
                    EmissionFactor::new(&ef.category, perturbed, &ef.source)
                })
                .collect();
            if let Ok(report) = self.engine.calculate(features, &factors, year) {
                totals.push(report.total_emission_tco2e);
            }
        }
        if totals.is_empty() {
            return Ok(UncertaintyReport {
                mean: 0.0,
                median: 0.0,
                p5: 0.0,
                p95: 0.0,
                stddev: 0.0,
                cv: 0.0,
                simulations: 0,
            });
        }
        totals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = totals.len();
        let mean = totals.iter().sum::<f64>() / n as f64;
        let median = totals[n / 2];
        let p5 = totals[(0.05 * n as f64) as usize];
        let p95 = totals[(0.95 * n as f64) as usize];
        let variance = totals.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / n as f64;
        let stddev = variance.sqrt();
        let cv = if mean.abs() > 1e-10 {
            (stddev / mean.abs()) * 100.0
        } else {
            0.0
        };
        Ok(UncertaintyReport {
            mean,
            median,
            p5,
            p95,
            stddev,
            cv,
            simulations: n,
        })
    }
}

/// Monte Carlo uncertainty result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UncertaintyReport {
    pub mean: f64,
    pub median: f64,
    pub p5: f64,
    pub p95: f64,
    pub stddev: f64,
    pub cv: f64,
    pub simulations: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_from_geojson() {
        let config: CarbonConfig = toml::from_str(
            r#"
            [plugin]
            name = "carbon"
            version = "0.1.0"
            description = "test"
        "#,
        )
        .unwrap();

        let plugin = CarbonPlugin::load(config);

        let geojson = r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "properties": {"class": "forest"},
                    "geometry": {"type": "Polygon", "coordinates": [[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}
                }
            ]
        }"#;

        let report = plugin.calculate_from_geojson(geojson, 2025).unwrap();
        assert_eq!(report.classes.len(), 1);
        assert_eq!(report.classes[0].landcover_class, "forest");
        // 森林碳汇为负值
        assert!(report.total_emission_tco2e < 0.0);
        assert!(report.is_net_sink());
    }

    #[test]
    fn test_monte_carlo_uncertainty() {
        let config: CarbonConfig = toml::from_str(include_str!("../rules.toml")).unwrap();
        let plugin = CarbonPlugin::load(config);
        let features = vec![
            GeoFeature::new("forest", r#"{"type":"Polygon","coordinates":[[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}"#).unwrap(),
        ];
        let report = plugin
            .monte_carlo_uncertainty(&features, 2025, 100)
            .unwrap();
        assert_eq!(report.simulations, 100);
        // Mean should be negative (carbon sink)
        assert!(
            report.mean < 0.0,
            "Expected negative mean (sink), got {}",
            report.mean
        );
        // CV should be > 0
        assert!(report.cv > 0.0);
        // p5 < p95
        assert!(report.p5 <= report.p95);
    }

    #[test]
    fn test_build_factors_with_default_config() {
        let config: CarbonConfig = toml::from_str(
            r#"
            [plugin]
            name = "carbon"
            version = "0.1.0"
            description = "test"
        "#,
        )
        .unwrap();
        let plugin = CarbonPlugin::load(config);
        let (factors, source) = plugin.build_factors(100.0);
        assert_eq!(factors.len(), 7);
        assert!(!source.is_empty());
        let classes: Vec<&str> = factors.iter().map(|f| f.category.as_str()).collect();
        assert!(classes.contains(&"forest"));
        assert!(classes.contains(&"grassland"));
        assert!(classes.contains(&"wetland"));
        assert!(classes.contains(&"cropland"));
        assert!(classes.contains(&"built_up"));
        assert!(classes.contains(&"water"));
        assert!(classes.contains(&"bare"));
    }
}
