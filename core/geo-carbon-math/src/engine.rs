//! Carbon calculation engine.
//!
//! Pure-Rust implementation of IPCC Tier 1 emission factor methodology.
//! Extended with multi-gas breakdown, GWP conversion, uncertainty propagation,
//! and audit trail generation.

use std::collections::HashMap;

use crate::factor::{gwp100, EmissionFactor, EmissionScope, FuelType, GreenhouseGas, GwpVersion};
use crate::feature::GeoFeature;
use crate::report::{
    AuditEntry, CarbonReport, ClassResult, FactorSourceUnit, GasBreakdown, ScopeSummary,
};

struct ClassAggregate {
    landcover_class: String,
    factor_source: String,
    factor_unit: String,
    factor_value: f64,
    total_area_ha: f64,
    feature_count: u32,
    gas_contributions: Vec<(GreenhouseGas, f64)>,
    uncertainty_pct: Option<f64>,
    gwp_version: Option<GwpVersion>,
    scope: Option<EmissionScope>,
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

// ── Activity Record (industrial) ─────────────────────────────

/// A single activity record for GHG calculation.
///
/// Represents one unit of activity that produces emissions:
/// fuel burned, electricity consumed, material processed, distance traveled.
#[derive(Debug, Clone)]
pub struct ActivityRecord {
    /// Activity category/label (e.g., "coal_boiler", "grid_power", "truck_freight").
    pub category: String,
    /// Quantity of activity (e.g., tons of coal, kWh of electricity, km traveled).
    pub quantity: f64,
    /// Unit of measurement.
    pub unit: String,
    /// GHG Protocol scope (1/2/3).
    pub scope: EmissionScope,
    /// Optional fuel type for combustion activities.
    pub fuel_type: Option<FuelType>,
    /// Optional NCV override (GJ/unit).
    pub ncv: Option<f64>,
    /// Optional carbon content override (tC/TJ).
    pub cc: Option<f64>,
    /// Optional oxidation rate override.
    pub ox: Option<f64>,
}

impl ActivityRecord {
    /// Create a fuel combustion record (Scope 1).
    pub fn fuel(category: impl Into<String>, fuel_type: FuelType, quantity: f64) -> Self {
        Self {
            category: category.into(),
            quantity,
            unit: fuel_type.unit().to_string(),
            scope: EmissionScope::Scope1,
            fuel_type: Some(fuel_type),
            ncv: None,
            cc: None,
            ox: None,
        }
    }

    /// Create an electricity consumption record (Scope 2).
    pub fn electricity(category: impl Into<String>, kwh: f64) -> Self {
        Self {
            category: category.into(),
            quantity: kwh,
            unit: "kWh".to_string(),
            scope: EmissionScope::Scope2,
            fuel_type: None,
            ncv: None,
            cc: None,
            ox: None,
        }
    }

    /// Create a process material record (Scope 1 or 3).
    pub fn material(
        category: impl Into<String>,
        quantity: f64,
        unit: impl Into<String>,
        scope: EmissionScope,
    ) -> Self {
        Self {
            category: category.into(),
            quantity,
            unit: unit.into(),
            scope,
            fuel_type: None,
            ncv: None,
            cc: None,
            ox: None,
        }
    }

    /// Compute CO₂ emission using fuel parameters or factor lookup.
    pub fn compute_co2(&self, factor: &EmissionFactor) -> f64 {
        if let Some(ft) = self.fuel_type {
            let ncv = self
                .ncv
                .or(factor.ncv_override)
                .unwrap_or_else(|| ft.default_ncv());
            let cc = self
                .cc
                .or(factor.cc_override)
                .unwrap_or_else(|| ft.default_carbon_content());
            let ox = self
                .ox
                .or(factor.ox_override)
                .unwrap_or_else(|| ft.default_oxidation_rate());
            self.quantity * ncv * cc * ox / 1000.0 * (44.0 / 12.0)
        } else {
            self.quantity * factor.factor_value
        }
    }
}

// ── Engine ────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct CarbonEngine;

impl CarbonEngine {
    pub fn new() -> Self {
        Self::default()
    }

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

        let factor_map: HashMap<String, &EmissionFactor> = factors
            .iter()
            .filter(|f| f.is_valid_for_year(year_i32))
            .fold(HashMap::new(), |mut acc, f| {
                let key = match &f.subcategory {
                    Some(sub) if !sub.is_empty() => format!("{}:{}", f.category, sub),
                    _ => f.category.clone(),
                };
                acc.entry(key).or_insert(f);
                acc
            });

        if factor_map.is_empty() {
            return Err(format!("No emission factors valid for year {year}"));
        }

        let mut aggregates: HashMap<(String, String, String), ClassAggregate> = HashMap::new();

        for feature in features {
            let class_lower = feature.landcover_class.to_lowercase();
            let factor = match factor_map.get(class_lower.as_str()) {
                Some(f) => *f,
                None => {
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
                gas_contributions: Vec::new(),
                uncertainty_pct: factor.uncertainty_pct,
                gwp_version: factor.gas_factors.first().map(|gf| gf.gwp_version),
                scope: factor.scope,
            });

            entry.total_area_ha += area_ha;
            entry.feature_count += 1;

            if factor.has_gas_breakdown() {
                for gf in &factor.gas_factors {
                    let tco2e = area_ha * gf.factor * gwp100(gf.gas, gf.gwp_version) / 1000.0;
                    entry.gas_contributions.push((gf.gas, tco2e));
                }
            } else {
                entry
                    .gas_contributions
                    .push((GreenhouseGas::CO2, area_ha * factor.factor_value));
            }
        }

        if aggregates.is_empty() {
            let available: Vec<_> = factor_map.keys().collect();
            return Err(format!(
                "No features matched any emission factor. \
                 Features have classes that don't match factors. \
                 Available factors: {available:?}"
            ));
        }

        let classified_count: u32 = aggregates.values().map(|a| a.feature_count).sum();
        let skipped = features.len() as u32 - classified_count;

        let mut classes: Vec<ClassResult> = aggregates
            .into_values()
            .map(|agg| {
                let emission_tco2e = agg.total_area_ha * agg.factor_value;
                let mut consolidated: HashMap<GreenhouseGas, f64> = HashMap::new();
                for (gas, val) in &agg.gas_contributions {
                    *consolidated.entry(*gas).or_insert(0.0) += val;
                }
                let contributions: Vec<(GreenhouseGas, f64)> = consolidated.into_iter().collect();
                let gas_breakdown = GasBreakdown::from_gas_contributions(&contributions);
                let uncertainty_tco2e = agg
                    .uncertainty_pct
                    .map(|pct| (emission_tco2e.abs() * pct / 100.0).abs());

                ClassResult {
                    landcover_class: agg.landcover_class,
                    area_ha: round2(agg.total_area_ha),
                    factor_value: agg.factor_value,
                    emission_tco2e: round2(emission_tco2e),
                    factor_source: FactorSourceUnit {
                        source: agg.factor_source,
                        unit: agg.factor_unit,
                        gwp_version: agg
                            .gwp_version
                            .map(|v| format!("{v:?}"))
                            .unwrap_or_default(),
                    },
                    feature_count: agg.feature_count,
                    gas_breakdown,
                    uncertainty_tco2e: uncertainty_tco2e.map(round2),
                    scope: agg.scope,
                }
            })
            .collect();

        classes.sort_by(|a, b| {
            b.emission_tco2e
                .abs()
                .partial_cmp(&a.emission_tco2e.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let total_area_ha: f64 = classes.iter().map(|c| c.area_ha).sum();
        let total_emission_tco2e: f64 = classes.iter().map(|c| c.emission_tco2e).sum();

        let mut gas_summary = GasBreakdown::default();
        for cls in &classes {
            gas_summary.merge(&cls.gas_breakdown);
        }

        let uncertainty_total_tco2e = {
            let sum_sq: f64 = classes
                .iter()
                .filter_map(|c| c.uncertainty_tco2e.map(|u| u * u))
                .sum();
            if sum_sq > 0.0 {
                Some(round2(sum_sq.sqrt()))
            } else {
                None
            }
        };

        let audit_trail: Vec<AuditEntry> = classes
            .iter()
            .map(|c| {
                let rel_unc = c.uncertainty_tco2e.and_then(|u| {
                    if c.emission_tco2e.abs() > f64::EPSILON {
                        Some(round2(u / c.emission_tco2e.abs() * 100.0))
                    } else {
                        None
                    }
                });
                AuditEntry {
                    landcover_class: c.landcover_class.clone(),
                    lc_hash: String::new(),
                    factor_id: c.factor_source.source.clone(),
                    factor_hash: String::new(),
                    gwp_version: c.factor_source.gwp_version.clone(),
                    uncertainty_pct: rel_unc,
                    complete: true,
                    scope: c.scope,
                }
            })
            .collect();

        // Compute scope summary
        let mut scope_summary = ScopeSummary::default();
        for cls in &classes {
            match cls.scope {
                Some(EmissionScope::Scope1) => scope_summary.scope1_tco2e += cls.emission_tco2e,
                Some(EmissionScope::Scope2) => scope_summary.scope2_tco2e += cls.emission_tco2e,
                Some(EmissionScope::Scope3) => scope_summary.scope3_tco2e += cls.emission_tco2e,
                None => scope_summary.scope1_tco2e += cls.emission_tco2e, // default to Scope 1
            }
        }

        Ok(CarbonReport {
            aoi_name: None,
            year,
            classes,
            total_area_ha: round2(total_area_ha),
            total_emission_tco2e: round2(total_emission_tco2e),
            total_features: features.len() as u32,
            classified_features: classified_count,
            skipped_features: skipped,
            calculated_at: chrono::Utc::now().to_rfc3339(),
            auditor: None,
            methodology: Some("IPCC Tier 1 — Emission Factor Method".into()),
            gas_summary,
            uncertainty_total_tco2e,
            audit_trail,
            scope_summary,
        })
    }

    pub fn calculate_from_geojson(
        &self,
        geojson_fc: &str,
        factors_csv: &str,
        year: u16,
    ) -> Result<CarbonReport, String> {
        let fc: serde_json::Value =
            serde_json::from_str(geojson_fc).map_err(|e| format!("Invalid GeoJSON: {e}"))?;
        let features_json = fc["features"]
            .as_array()
            .ok_or("GeoJSON has no 'features' array")?;
        let features: Vec<GeoFeature> = features_json
            .iter()
            .filter_map(|f| GeoFeature::from_feature_json(&f.to_string()).ok())
            .collect();
        let factors = crate::factor::load_factors_from_csv(factors_csv)?;
        self.calculate(&features, &factors, year)
    }

    /// Calculate emissions from generic activity records (industrial).
    ///
    /// Supports fuel combustion, electricity, and material processing.
    /// Uses fuel combustion parameters (NCV × CC × Ox × 44/12) when available,
    /// otherwise falls back to factor_value.
    pub fn calculate_activities(
        &self,
        activities: &[ActivityRecord],
        factors: &[EmissionFactor],
        year: u16,
    ) -> Result<CarbonReport, String> {
        if activities.is_empty() {
            return Err("No activities provided".into());
        }
        if factors.is_empty() {
            return Err("No emission factors provided".into());
        }

        let year_i32 = year as i32;
        let factor_map: HashMap<String, &EmissionFactor> = factors
            .iter()
            .filter(|f| f.is_valid_for_year(year_i32))
            .fold(HashMap::new(), |mut acc, f| {
                acc.entry(f.category.clone()).or_insert(f);
                acc
            });

        let mut aggregates: HashMap<String, ClassAggregate> = HashMap::new();

        for activity in activities {
            let factor = match factor_map.get(&activity.category) {
                Some(f) => *f,
                None => continue,
            };

            let emission = activity.compute_co2(factor);

            let entry = aggregates
                .entry(activity.category.clone())
                .or_insert_with(|| ClassAggregate {
                    landcover_class: activity.category.clone(),
                    factor_source: factor.source.clone(),
                    factor_unit: activity.unit.clone(),
                    factor_value: factor.factor_value,
                    total_area_ha: 0.0,
                    feature_count: 0,
                    gas_contributions: Vec::new(),
                    uncertainty_pct: factor.uncertainty_pct,
                    gwp_version: None,
                    scope: Some(activity.scope),
                });

            entry.total_area_ha += activity.quantity;
            entry.feature_count += 1;
            entry.gas_contributions.push((GreenhouseGas::CO2, emission));
        }

        if aggregates.is_empty() {
            return Err("No activities matched any factor".into());
        }

        let classified_count: u32 = aggregates.values().map(|a| a.feature_count).sum();
        let skipped = activities.len() as u32 - classified_count;

        let mut classes: Vec<ClassResult> = aggregates
            .into_values()
            .map(|agg| {
                let emission_tco2e = agg.gas_contributions.iter().map(|(_, v)| v).sum::<f64>();
                let mut consolidated: HashMap<GreenhouseGas, f64> = HashMap::new();
                for (gas, val) in &agg.gas_contributions {
                    *consolidated.entry(*gas).or_insert(0.0) += val;
                }
                let contributions: Vec<(GreenhouseGas, f64)> = consolidated.into_iter().collect();
                let gas_breakdown = GasBreakdown::from_gas_contributions(&contributions);
                let uncertainty_tco2e = agg
                    .uncertainty_pct
                    .map(|pct| (emission_tco2e.abs() * pct / 100.0).abs());
                ClassResult {
                    landcover_class: agg.landcover_class,
                    area_ha: round2(agg.total_area_ha),
                    factor_value: agg.factor_value,
                    emission_tco2e: round2(emission_tco2e),
                    factor_source: FactorSourceUnit {
                        source: agg.factor_source,
                        unit: agg.factor_unit,
                        gwp_version: String::new(),
                    },
                    feature_count: agg.feature_count,
                    gas_breakdown,
                    uncertainty_tco2e: uncertainty_tco2e.map(round2),
                    scope: agg.scope,
                }
            })
            .collect();

        classes.sort_by(|a, b| {
            b.emission_tco2e
                .abs()
                .partial_cmp(&a.emission_tco2e.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let total_emission_tco2e: f64 = classes.iter().map(|c| c.emission_tco2e).sum();
        let total_qty: f64 = classes.iter().map(|c| c.area_ha).sum();
        let mut gas_summary = GasBreakdown::default();
        for cls in &classes {
            gas_summary.merge(&cls.gas_breakdown);
        }

        let uncertainty_total_tco2e = {
            let sum_sq: f64 = classes
                .iter()
                .filter_map(|c| c.uncertainty_tco2e.map(|u| u * u))
                .sum();
            if sum_sq > 0.0 {
                Some(round2(sum_sq.sqrt()))
            } else {
                None
            }
        };

        let mut scope_summary = ScopeSummary::default();
        for cls in &classes {
            match cls.scope {
                Some(EmissionScope::Scope1) => scope_summary.scope1_tco2e += cls.emission_tco2e,
                Some(EmissionScope::Scope2) => scope_summary.scope2_tco2e += cls.emission_tco2e,
                Some(EmissionScope::Scope3) => scope_summary.scope3_tco2e += cls.emission_tco2e,
                None => {}
            }
        }

        let audit_trail: Vec<AuditEntry> = classes
            .iter()
            .map(|c| AuditEntry {
                landcover_class: c.landcover_class.clone(),
                lc_hash: String::new(),
                factor_id: c.factor_source.source.clone(),
                factor_hash: String::new(),
                gwp_version: String::new(),
                uncertainty_pct: c.uncertainty_tco2e.and_then(|u| {
                    if c.emission_tco2e.abs() > f64::EPSILON {
                        Some(round2(u / c.emission_tco2e.abs() * 100.0))
                    } else {
                        None
                    }
                }),
                complete: true,
                scope: c.scope,
            })
            .collect();

        Ok(CarbonReport {
            aoi_name: None,
            year,
            classes,
            total_area_ha: round2(total_qty),
            total_emission_tco2e: round2(total_emission_tco2e),
            total_features: activities.len() as u32,
            classified_features: classified_count,
            skipped_features: skipped,
            calculated_at: chrono::Utc::now().to_rfc3339(),
            auditor: None,
            methodology: Some("IPCC Tier 1 — Industrial".into()),
            gas_summary,
            uncertainty_total_tco2e,
            audit_trail,
            scope_summary,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factor::{load_factors_from_csv, EmissionFactor, GasFactor, GreenhouseGas};
    use crate::feature::GeoFeature;

    fn make_features() -> Vec<GeoFeature> {
        let geom = r#"{"type":"Polygon","coordinates":[[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}"#;
        vec![
            GeoFeature::new("forest", geom).unwrap(),
            GeoFeature::new("grassland", geom).unwrap(),
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
        let report = engine
            .calculate(&make_features(), &make_factors(), 2025)
            .unwrap();
        assert_eq!(report.classes.len(), 2);
        assert!(report.total_area_ha > 0.0);
        assert!(report.total_emission_tco2e > 0.0);
        assert!(!report.gas_summary.is_empty());
        assert!(!report.audit_trail.is_empty());
    }

    #[test]
    fn test_empty_features() {
        assert!(CarbonEngine::new()
            .calculate(&[], &make_factors(), 2025)
            .is_err());
    }

    #[test]
    fn test_no_matching_factors() {
        let factors = vec![EmissionFactor::new("wetland", 2.0, "TEST")];
        assert!(CarbonEngine::new()
            .calculate(&make_features(), &factors, 2025)
            .is_err());
    }

    #[test]
    fn test_year_filtering() {
        let features = make_features();
        let mut forest_factor = EmissionFactor::new("forest", 5.0, "IPCC");
        forest_factor.valid_from_year = 2030;
        let grassland_factor = EmissionFactor::new("grassland", -1.0, "IPCC");
        let report = CarbonEngine::new()
            .calculate(&features, &[forest_factor, grassland_factor], 2025)
            .unwrap();
        assert_eq!(report.classes.len(), 1);
        assert_eq!(report.classified_features, 1);
        assert_eq!(report.skipped_features, 1);
    }

    #[test]
    fn test_report_serialization() {
        let report = CarbonEngine::new()
            .calculate(&make_features(), &make_factors(), 2025)
            .unwrap();
        let json = serde_json::to_string_pretty(&report).unwrap();
        assert!(json.contains("forest"));
        assert!(json.contains("gas_summary"));
    }

    #[test]
    fn test_multi_gas_calculation() {
        let features = vec![
            GeoFeature::new("rice", r#"{"type":"Polygon","coordinates":[[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}"#).unwrap()
        ];
        let factors = vec![EmissionFactor::with_gases(
            "rice",
            "IPCC_2019",
            vec![
                GasFactor::land_use(GreenhouseGas::CH4, 150.0, "kg CH₄/ha/yr"),
                GasFactor::land_use(GreenhouseGas::N2O, 3.0, "kg N₂O/ha/yr"),
            ],
            Some(30.0),
        )];
        let report = CarbonEngine::new()
            .calculate(&features, &factors, 2025)
            .unwrap();
        assert_eq!(report.classes.len(), 1);
        assert!(report.classes[0].gas_breakdown.ch4_tco2e != 0.0);
        assert!(report.classes[0].gas_breakdown.n2o_tco2e != 0.0);
        assert!(report.classes[0].uncertainty_tco2e.is_some());
        assert!(report.uncertainty_total_tco2e.is_some());
    }

    #[test]
    fn test_audit_trail() {
        let report = CarbonEngine::new()
            .calculate(&make_features(), &make_factors(), 2025)
            .unwrap();
        assert_eq!(report.audit_trail.len(), report.classes.len());
        for entry in &report.audit_trail {
            assert!(entry.complete);
        }
    }

    #[test]
    fn test_subcategory_match() {
        let mut feat = GeoFeature::new("forest",
            r#"{"type":"Polygon","coordinates":[[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}"#).unwrap();
        feat.landcover_class = "forest:evergreen_broadleaf".into();
        let mut factor = EmissionFactor::new("forest", 5.0, "IPCC");
        factor.subcategory = Some("evergreen_broadleaf".into());
        let report = CarbonEngine::new()
            .calculate(&[feat], &[factor], 2025)
            .unwrap();
        assert_eq!(report.classes.len(), 1);
    }

    #[test]
    fn test_csv_multi_gas() {
        let csv = "source,category,subcategory,factor_value,unit,valid_from_year,valid_to_year,region\nIPCC_2019,forest,evergreen_broadleaf,-380.0,tCO2e/ha,2019,2030,CN-51";
        let factors = load_factors_from_csv(csv).unwrap();
        assert_eq!(factors.len(), 1);
    }

    // ── Integration tests ──

    /// Full end-to-end: GeoJSON + CSV → CarbonReport
    #[test]
    fn test_integration_geojson_to_report() {
        let geojson = r#"{"type":"FeatureCollection","features":[
            {"type":"Feature","properties":{"class":"forest","area_ha":100.0},"geometry":{"type":"Polygon","coordinates":[[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}},
            {"type":"Feature","properties":{"class":"grassland","area_ha":50.0},"geometry":{"type":"Polygon","coordinates":[[[104.2,30.5],[104.3,30.5],[104.3,30.6],[104.2,30.6],[104.2,30.5]]]}},
            {"type":"Feature","properties":{"class":"wetland","area_ha":20.0},"geometry":{"type":"Polygon","coordinates":[[[104.4,30.5],[104.5,30.5],[104.5,30.6],[104.4,30.6],[104.4,30.5]]]}}
        ]}"#;

        let csv = "source,category,factor_value,unit,valid_from_year,valid_to_year\nIPCC_2019,forest,5.0,tCO2e/ha/yr,2019,2030\nIPCC_2019,grassland,1.2,tCO2e/ha/yr,2019,2030\nIPCC_2019,wetland,8.5,tCO2e/ha/yr,2019,2030";

        let engine = CarbonEngine::new();
        let report = engine.calculate_from_geojson(geojson, csv, 2025).unwrap();

        // Structural assertions
        assert_eq!(report.classes.len(), 3, "should have 3 landcover classes");
        assert!(report.total_area_ha > 0.0);
        assert!(report.total_emission_tco2e > 0.0);
        assert!(!report.gas_summary.is_empty());
        assert!(!report.audit_trail.is_empty());

        // Per-class checks
        let forest = report
            .classes
            .iter()
            .find(|c| c.landcover_class == "forest")
            .unwrap();
        assert!(forest.area_ha > 0.0);
        assert!(
            (forest.emission_tco2e - forest.area_ha * 5.0).abs() < 1.0,
            "forest emission should be area × 5.0"
        );

        let wetland = report
            .classes
            .iter()
            .find(|c| c.landcover_class == "wetland")
            .unwrap();
        assert!(wetland.emission_tco2e > 0.0);

        // Audit trail: all features should have audit entries
        assert_eq!(report.audit_trail.len(), 3);
        for entry in &report.audit_trail {
            assert!(entry.complete, "all audit entries should be complete");
        }
    }

    /// Invalid GeoJSON should return an error, not panic.
    #[test]
    fn test_integration_invalid_geojson() {
        let engine = CarbonEngine::new();
        let csv = "source,category,factor_value,unit,valid_from_year,valid_to_year\nIPCC_2019,forest,5.0,tCO2e/ha/yr,2019,2030";

        let result = engine.calculate_from_geojson("not valid json", csv, 2025);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid GeoJSON"));
    }

    /// Empty GeoJSON features array returns an error (no features).
    #[test]
    fn test_integration_empty_features() {
        let engine = CarbonEngine::new();
        let geojson = r#"{"type":"FeatureCollection","features":[]}"#;
        let csv = "source,category,factor_value,unit,valid_from_year,valid_to_year\nIPCC_2019,forest,5.0,tCO2e/ha/yr,2019,2030";

        let result = engine.calculate_from_geojson(geojson, csv, 2025);
        assert!(result.is_err(), "empty features should be an error");
    }

    /// GeoJSON without a "features" array.
    #[test]
    fn test_integration_no_features_array() {
        let engine = CarbonEngine::new();
        let geojson = r#"{"type":"Point","coordinates":[104.0,30.5]}"#;
        let csv = "source,category,factor_value,unit,valid_from_year,valid_to_year\nIPCC_2019,forest,5.0,tCO2e/ha/yr,2019,2030";

        let result = engine.calculate_from_geojson(geojson, csv, 2025);
        assert!(result.is_err());
    }

    /// Invalid CSV should return an error.
    #[test]
    fn test_integration_invalid_csv() {
        let engine = CarbonEngine::new();
        let geojson = r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"class":"forest"},"geometry":{"type":"Polygon","coordinates":[[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}}]}"#;

        // Completely malformed CSV (only header, no data → parse will fail because
        // there are no valid rows with required fields like 'category' and 'factor_value')
        let csv = "totally,wrong,headers,only\njust,one,row,here";
        let result = engine.calculate_from_geojson(geojson, csv, 2025);
        assert!(
            result.is_err(),
            "malformed CSV should fail: {:?}",
            result.ok()
        );
    }

    /// Year filtering: using a year outside valid range should exclude factors.
    #[test]
    fn test_integration_year_filtering() {
        let engine = CarbonEngine::new();
        let geojson = r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"class":"forest"},"geometry":{"type":"Polygon","coordinates":[[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}}]}"#;

        // Factor valid 2019-2030, query year 2050 → no matching factors
        let csv = "source,category,factor_value,unit,valid_from_year,valid_to_year\nIPCC_2019,forest,5.0,tCO2e/ha/yr,2019,2030";
        let result = engine.calculate_from_geojson(geojson, csv, 2050);
        assert!(result.is_err());
    }

    /// Subcategory matching: specific subcategories should match granular factors.
    #[test]
    fn test_integration_subcategory_matching() {
        let geojson = r#"{"type":"FeatureCollection","features":[
            {"type":"Feature","properties":{"class":"forest","subcategory":"evergreen_broadleaf"},"geometry":{"type":"Polygon","coordinates":[[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}}
        ]}"#;

        // Two factors: one generic "forest", one specific "forest:evergreen_broadleaf"
        let csv = "source,category,subcategory,factor_value,unit,valid_from_year,valid_to_year\nIPCC_2019,forest,,5.0,tCO2e/ha/yr,2019,2030\nIPCC_2019,forest,evergreen_broadleaf,-380.0,tCO2e/ha,2019,2030";

        let engine = CarbonEngine::new();
        let report = engine.calculate_from_geojson(geojson, csv, 2025).unwrap();
        assert_eq!(report.classes.len(), 1);
        // Should compute an emission (positive or negative) using subcategory match
        assert!(report.total_emission_tco2e != 0.0);
    }

    /// Verify report can be serialized to JSON (for API/MCP transport).
    #[test]
    fn test_integration_report_json_roundtrip() {
        let engine = CarbonEngine::new();
        let report = engine
            .calculate(&make_features(), &make_factors(), 2025)
            .unwrap();

        let json = serde_json::to_value(&report).unwrap();
        assert!(json.is_object());
        assert!(json["classes"].is_array());
        assert!(json["total_emission_tco2e"].is_number());
        assert!(json["audit_trail"].is_array());

        // Round-trip: JSON → Report
        let report2: CarbonReport = serde_json::from_value(json).unwrap();
        assert_eq!(report.total_emission_tco2e, report2.total_emission_tco2e);
        assert_eq!(report.classes.len(), report2.classes.len());
    }
}
