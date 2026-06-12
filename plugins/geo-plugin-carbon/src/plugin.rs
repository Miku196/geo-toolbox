//! 碳核算插件主体。
//!
//! 封装 geo-carbon-math 引擎，提供从 GeoJSON + 配置直接输出报告的接口。

use geo_carbon_math::{CarbonEngine, CarbonReport, EmissionFactor, GeoFeature};
use geo_core::errors::{GeoError, GeoResult};
use crate::config::CarbonConfig;

/// 碳核算插件。
pub struct CarbonPlugin {
    config: CarbonConfig,
    engine: CarbonEngine,
}

impl CarbonPlugin {
    /// 从 rules.toml 加载配置。
    pub fn load(config: CarbonConfig) -> Self {
        Self {
            config,
            engine: CarbonEngine::new(),
        }
    }

    /// 从 GeoJSON FeatureCollection 计算碳核算。
    ///
    /// Features 的 properties 必须包含 `landcover`/`class`/`category` 之一。
    pub fn calculate_from_geojson(
        &self,
        geojson_fc: &str,
        year: u16,
    ) -> GeoResult<CarbonReport> {
        // 1. 解析 GeoJSON features
        let fc: serde_json::Value = serde_json::from_str(geojson_fc)
            .map_err(|e| GeoError::Validation(format!("Invalid GeoJSON: {e}")))?;

        let features_json = fc["features"].as_array()
            .ok_or_else(|| GeoError::invalid_input("aoi_geojson", "missing 'features' array"))?;

        let mut features = Vec::with_capacity(features_json.len());
        for f in features_json {
            let feat_str = serde_json::to_string(f)
                .map_err(GeoError::Serde)?;
            match GeoFeature::from_feature_json(&feat_str) {
                Ok(gf) => features.push(gf),
                Err(_) => continue, // skip unparseable features
            }
        }

        // 2. 从配置构建 EmissionFactor 列表
        let defaults = &self.config.carbon;
        let source = &defaults.source;
        let all_classes = [
            ("forest", defaults.forest),
            ("grassland", defaults.grassland),
            ("wetland", defaults.wetland),
            ("cropland", defaults.cropland),
            ("built_up", defaults.built_up),
            ("water", defaults.water),
            ("bare", defaults.bare),
        ];

        let factors: Vec<EmissionFactor> = all_classes.iter()
            .map(|(class, value)| EmissionFactor::new(*class, *value, source.as_str()))
            .collect();

        // 3. 计算
        let mut report = self.engine.calculate(&features, &factors, year)
            .map_err(GeoError::Validation)?;

        report.methodology = Some(format!("IPCC Tier 1 — {}", source));
        Ok(report)
    }

    /// 使用外部提供的 features 和 factors 计算。
    pub fn calculate(
        &self,
        features: &[GeoFeature],
        year: u16,
    ) -> Result<CarbonReport, String> {
        let defaults = &self.config.carbon;
        let source = &defaults.source;
        let all_classes = [
            ("forest", defaults.forest),
            ("grassland", defaults.grassland),
            ("wetland", defaults.wetland),
            ("cropland", defaults.cropland),
            ("built_up", defaults.built_up),
            ("water", defaults.water),
            ("bare", defaults.bare),
        ];

        let factors: Vec<EmissionFactor> = all_classes.iter()
            .map(|(class, value)| EmissionFactor::new(*class, *value, source.as_str()))
            .collect();

        let mut report = self.engine.calculate(features, &factors, year)?;
        report.methodology = Some(format!("IPCC Tier 1 — {}", source));
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_from_geojson() {
        let config: CarbonConfig = toml::from_str(r#"
            [plugin]
            name = "carbon"
            version = "0.1.0"
            description = "test"
        "#).unwrap();

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
}
