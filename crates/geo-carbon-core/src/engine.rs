//! Carbon calculation engine.
//!
//! Pure-Rust implementation of IPCC Tier 1 emission factor methodology.
//! No database, no network, no file system — just computation.

use std::collections::HashMap;

use crate::factor::EmissionFactor;
use crate::feature::GeoFeature;
use crate::report::{CarbonReport, ClassResult, FactorSourceUnit};

/// The carbon accounting engine.
///
/// ## Algorithm
///
/// For each landcover polygon:
/// 1. Compute area in hectares (equirectangular approximation for WGS84)
/// 2. Match landcover class to emission factor
/// 3. Multiply area × factor → tCO₂e
/// 4. Aggregate by class
///
/// ## Accuracy
///
/// Equirectangular approximation is accurate to ±5% for areas
/// within a single UTM zone (< 6° longitude). For higher precision,
/// reproject to EPSG:3405 (equal-area) before passing features.
#[derive(Debug, Default)]
pub struct CarbonEngine {
    // Future: cache for computed areas, factor lookups
}

impl CarbonEngine {
    /// Create a new carbon engine.
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate carbon emissions from features and emission factors.
    ///
    /// ## Parameters
    ///
    /// - `features`: Array of landcover polygons with `landcover_class` labels.
    /// - `factors`: Emission factor lookup table.
    /// - `year`: Target year (used for factor validity filtering).
    ///
    /// ## Returns
    ///
    /// A `CarbonReport` with per-class results and totals.
    pub fn calculate(
        &self,
        features: &[GeoFeature],
        factors: &[EmissionFactor],
        year: u16,
    ) -> Result<CarbonReport, String> {
        if features.is_empty() {
            return Err("No features provided".into());
        }
        if factors.is_empty() {
            return Err("No emission factors provided".into());
        }

        let year_i32 = year as i32;

        // Build factor lookup, filtering by year validity.
        // Key: category (or category+subcategory for granular matching)
        let factor_map: HashMap<String, &EmissionFactor> = factors
            .iter()
            .filter(|f| f.is_valid_for_year(year_i32))
            .fold(HashMap::new(), |mut acc, f| {
                let key = match &f.subcategory {
                    Some(sub) if !sub.is_empty() => format!("{}:{}", f.category, sub),
                    _ => f.category.clone(),
                };
                acc.entry(key).or_insert(f); // first match wins
                acc
            });

        if factor_map.is_empty() {
            return Err(format!("No emission factors valid for year {year}"));
        }

        // Aggregate by (class, source, unit)
        let mut aggregates: HashMap<(String, String, String), ClassAggregate> = HashMap::new();

        for feature in features {
            let class_lower = feature.landcover_class.to_lowercase();

            // Match: try exact key first, then fallback to category-only
            let factor = match factor_map.get(class_lower.as_str()) {
                Some(f) => *f,
                None => {
                    // If "category:subcategory" didn't match, try just "category"
                    if let Some((cat, _)) = class_lower.split_once(':') {
                        match factor_map.get(cat) {
                            Some(f) => *f,
                            None => continue,
                        }
                    } else {
                        continue;
                    }
                }
            };

            let area_ha = feature.area_ha();
            if area_ha <= 0.0 {
                continue;
            }

            let key = (
                class_lower.clone(),
                factor.source.clone(),
                factor.unit.clone(),
            );

            let entry = aggregates.entry(key).or_insert_with(|| ClassAggregate {
                landcover_class: class_lower,
                factor_source: factor.source.clone(),
                factor_unit: factor.unit.clone(),
                factor_value: factor.factor_value,
                total_area_ha: 0.0,
                feature_count: 0,
            });

            entry.total_area_ha += area_ha;
            entry.feature_count += 1;
        }

        if aggregates.is_empty() {
            let available: Vec<_> = factor_map.keys().collect();
            return Err(format!(
                "No features matched any emission factor. \
                 Features have classes that don't match factors. \
                 Available factors: {:?}",
                available
            ));
        }

        // Count unclassified features
        let classified_count: u32 = aggregates.values().map(|a| a.feature_count).sum();
        let skipped = features.len() as u32 - classified_count;

        // Build results sorted by absolute emission (largest first)
        let mut classes: Vec<ClassResult> = aggregates
            .into_values()
            .map(|agg| {
                let emission_tco2e = agg.total_area_ha * agg.factor_value;
                ClassResult {
                    landcover_class: agg.landcover_class,
                    area_ha: round2(agg.total_area_ha),
                    factor_value: agg.factor_value,
                    emission_tco2e: round2(emission_tco2e),
                    factor_source: FactorSourceUnit {
                        source: agg.factor_source,
                        unit: agg.factor_unit,
                    },
                    feature_count: agg.feature_count,
                }
            })
            .collect();

        classes.sort_by(|a, b| {
            b.emission_tco2e.abs()
                .partial_cmp(&a.emission_tco2e.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let total_area_ha: f64 = classes.iter().map(|c| c.area_ha).sum();
        let total_emission_tco2e: f64 = classes.iter().map(|c| c.emission_tco2e).sum();

        let calculated_at = chrono::Utc::now().to_rfc3339();

        Ok(CarbonReport {
            aoi_name: None,
            year,
            classes,
            total_area_ha: round2(total_area_ha),
            total_emission_tco2e: round2(total_emission_tco2e),
            total_features: features.len() as u32,
            classified_features: classified_count,
            skipped_features: skipped,
            calculated_at,
            auditor: None,
            methodology: Some("IPCC Tier 1 — Emission Factor Method".into()),
        })
    }

    /// Calculate from GeoJSON FeatureCollection and emission factor CSV.
    ///
    /// Convenience method for JSON-in/JSON-out usage (WASM, HTTP APIs).
    pub fn calculate_from_geojson(
        &self,
        geojson_fc: &str,
        factors_csv: &str,
        year: u16,
    ) -> Result<CarbonReport, String> {
        let fc: serde_json::Value = serde_json::from_str(geojson_fc)
            .map_err(|e| format!("Invalid GeoJSON: {e}"))?;

        let features_json = fc["features"].as_array()
            .ok_or("GeoJSON has no 'features' array")?;

        let features: Vec<GeoFeature> = features_json
            .iter()
            .filter_map(|f| GeoFeature::from_feature_json(&f.to_string()).ok())
            .collect();

        let factors = crate::factor::load_factors_from_csv(factors_csv)?;

        self.calculate(&features, &factors, year)
    }
}

// ── Internal helpers ─────────────────────────────────────────────

struct ClassAggregate {
    landcover_class: String,
    factor_source: String,
    factor_unit: String,
    factor_value: f64,
    total_area_ha: f64,
    feature_count: u32,
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factor::EmissionFactor;
    use crate::factor::load_factors_from_csv;
    use crate::feature::GeoFeature;

    fn make_features() -> Vec<GeoFeature> {
        let forest = r#"{"type":"Polygon","coordinates":[[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}"#;
        let grassland = r#"{"type":"Polygon","coordinates":[[[104.1,30.5],[104.2,30.5],[104.2,30.6],[104.1,30.6],[104.1,30.5]]]}"#;

        vec![
            GeoFeature::new("forest", forest).unwrap(),
            GeoFeature::new("grassland", grassland).unwrap(),
        ]
    }

    fn make_factors() -> Vec<EmissionFactor> {
        vec![
            EmissionFactor::new("forest", 5.0, "IPCC_2019"),
            EmissionFactor::new("grassland", -1.0, "IPCC_2019"),
        ]
    }

    #[test]
    fn test_basic_calculation() {
        let engine = CarbonEngine::new();
        let features = make_features();
        let factors = make_factors();

        let report = engine.calculate(&features, &factors, 2025).unwrap();
        assert_eq!(report.classes.len(), 2);
        assert!(report.total_area_ha > 0.0);
        assert!(report.total_emission_tco2e > 0.0); // forest emissions > grassland sink
        assert_eq!(report.year, 2025);
        assert_eq!(report.total_features, 2);
    }

    #[test]
    fn test_empty_features() {
        let engine = CarbonEngine::new();
        let result = engine.calculate(&[], &make_factors(), 2025);
        assert!(result.is_err());
    }

    #[test]
    fn test_no_matching_factors() {
        let engine = CarbonEngine::new();
        let features = make_features();
        let factors = vec![EmissionFactor::new("wetland", 2.0, "TEST")];
        let result = engine.calculate(&features, &factors, 2025);
        assert!(result.is_err());
    }

    #[test]
    fn test_year_filtering() {
        let engine = CarbonEngine::new();
        let features = make_features();

        let mut forest_factor = EmissionFactor::new("forest", 5.0, "IPCC");
        forest_factor.valid_from_year = 2030;
        let grassland_factor = EmissionFactor::new("grassland", -1.0, "IPCC");

        let report = engine.calculate(&features, &[forest_factor, grassland_factor], 2025).unwrap();
        // Only grassland should be classified
        assert_eq!(report.classes.len(), 1);
        assert_eq!(report.classified_features, 1);
        assert_eq!(report.skipped_features, 1);
    }

    #[test]
    fn test_report_serialization() {
        let engine = CarbonEngine::new();
        let report = engine.calculate(&make_features(), &make_factors(), 2025).unwrap();
        let json = serde_json::to_string_pretty(&report).unwrap();
        assert!(json.contains("forest"));
        assert!(json.contains("IPCC_2019"));
        assert!(json.contains("methodology"));
    }

    #[test]
    fn test_subcategory_match() {
        let engine = CarbonEngine::new();
        // Feature with "forest:evergreen_broadleaf" composite key
        let poly = r#"{"type":"Polygon","coordinates":[[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}"#;
        let mut feat = GeoFeature::new("forest", poly).unwrap();
        feat.landcover_class = "forest:evergreen_broadleaf".into();

        let mut factor = EmissionFactor::new("forest", 5.0, "IPCC");
        factor.subcategory = Some("evergreen_broadleaf".into());

        let report = engine.calculate(&[feat], &[factor], 2025).unwrap();
        assert_eq!(report.classes.len(), 1);
        assert_eq!(report.classes[0].landcover_class, "forest:evergreen_broadleaf");
    }

    #[test]
    fn test_subcategory_fallback() {
        let engine = CarbonEngine::new();
        let poly = r#"{"type":"Polygon","coordinates":[[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}"#;
        // Feature only has category, but factor has subcategory for other rows
        let feat = GeoFeature::new("forest", poly).unwrap();
        
        // Factor has subcategory set but feature doesn't => should fallback to category-only match
        let mut factor_evergreen = EmissionFactor::new("forest", -350.0, "IPCC");
        factor_evergreen.subcategory = Some("evergreen_broadleaf".into());
        let factor_generic = EmissionFactor::new("forest", -250.0, "IPCC");

        // Feature is just "forest", should match factor_generic (key "forest")
        let report = engine.calculate(&[feat], &[factor_evergreen, factor_generic], 2025).unwrap();
        assert_eq!(report.classes.len(), 1);
        assert_eq!(report.classes[0].factor_value, -250.0);
    }

    #[test]
    fn test_header_based_csv() {
        // CSV with columns in non-standard order (like real IPCC data)
        let csv = "source,category,subcategory,factor_value,unit,valid_from_year,valid_to_year,region\nIPCC_2019,forest,evergreen_broadleaf,-380.0,tCO2e/ha,2019,2030,CN-51";
        let factors = load_factors_from_csv(csv).unwrap();
        assert_eq!(factors.len(), 1);
        assert_eq!(factors[0].category, "forest");
        assert_eq!(factors[0].subcategory, Some("evergreen_broadleaf".into()));
        assert_eq!(factors[0].factor_value, -380.0);
        assert_eq!(factors[0].source, "IPCC_2019");
        assert_eq!(factors[0].region, Some("CN-51".into()));
    }
}
