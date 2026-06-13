//! Carbon report types — the output of a calculation.
//!
//! Extended with multi-gas breakdown, uncertainty tracking, audit trail,
//! and GHG Protocol scope classification.

use crate::factor::{EmissionScope, GreenhouseGas};
use serde::{Deserialize, Serialize};

// ── Scope Summary ─────────────────────────────────────────────

/// Emissions aggregated by GHG Protocol scope.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScopeSummary {
    /// Scope 1: Direct emissions (tCO₂e).
    #[serde(default)]
    pub scope1_tco2e: f64,
    /// Scope 2: Indirect energy emissions (tCO₂e).
    #[serde(default)]
    pub scope2_tco2e: f64,
    /// Scope 3: Value chain emissions (tCO₂e).
    #[serde(default)]
    pub scope3_tco2e: f64,
}

impl ScopeSummary {
    pub fn total(&self) -> f64 {
        self.scope1_tco2e + self.scope2_tco2e + self.scope3_tco2e
    }

    pub fn is_empty(&self) -> bool {
        self.total().abs() < f64::EPSILON
    }
}

// ── Gas Breakdown ─────────────────────────────────────────────

/// Per-gas emission breakdown in tCO₂e.
///
/// Each field is already GWP-converted to CO₂-equivalent tonnes.
/// The sum of all fields equals `total_tco2e`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct GasBreakdown {
    /// Carbon dioxide (GWP=1).
    #[serde(default)]
    pub co2_tco2e: f64,
    /// Methane (GWP=28 AR5, converted to tCO₂e).
    #[serde(default)]
    pub ch4_tco2e: f64,
    /// Nitrous oxide (GWP=265 AR5, converted to tCO₂e).
    #[serde(default)]
    pub n2o_tco2e: f64,
    /// HFCs + PFCs + SF₆ + NF₃ combined (GWP-converted to tCO₂e).
    #[serde(default)]
    pub other_tco2e: f64,
}

impl GasBreakdown {
    /// Total tCO₂e across all gases.
    pub fn total(&self) -> f64 {
        self.co2_tco2e + self.ch4_tco2e + self.n2o_tco2e + self.other_tco2e
    }

    /// Returns true if all gas contributions are zero.
    pub fn is_empty(&self) -> bool {
        self.total().abs() < f64::EPSILON
    }

    /// Merge another breakdown into this one (additive).
    pub fn merge(&mut self, other: &GasBreakdown) {
        self.co2_tco2e += other.co2_tco2e;
        self.ch4_tco2e += other.ch4_tco2e;
        self.n2o_tco2e += other.n2o_tco2e;
        self.other_tco2e += other.other_tco2e;
    }

    /// Compute from a vector of (GreenhouseGas, tCO₂e) pairs.
    pub fn from_gas_contributions(contributions: &[(GreenhouseGas, f64)]) -> Self {
        let mut bd = GasBreakdown::default();
        for (gas, tco2e) in contributions {
            match gas {
                GreenhouseGas::CO2 => bd.co2_tco2e += tco2e,
                GreenhouseGas::CH4 => bd.ch4_tco2e += tco2e,
                GreenhouseGas::N2O => bd.n2o_tco2e += tco2e,
                GreenhouseGas::HFCs
                | GreenhouseGas::PFCs
                | GreenhouseGas::SF6
                | GreenhouseGas::NF3 => bd.other_tco2e += tco2e,
            }
        }
        bd
    }
}

// ── Audit Trail Entry ─────────────────────────────────────────

/// A single audit entry recording factor provenance and computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Landcover class.
    pub landcover_class: String,
    /// Hash of the landcover classification (for immutability).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub lc_hash: String,
    /// Emission factor identifier.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub factor_id: String,
    /// Hash of the emission factor data.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub factor_hash: String,
    /// GWP version used for conversion.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub gwp_version: String,
    /// Uncertainty (±%) if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncertainty_pct: Option<f64>,
    /// Whether all verification checks passed.
    #[serde(default)]
    pub complete: bool,

    /// GHG Protocol scope (1/2/3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<EmissionScope>,
}

// ── Full Carbon Report ────────────────────────────────────────

/// Full carbon accounting report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonReport {
    /// Name of the area of interest (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aoi_name: Option<String>,

    /// Target year for the calculation.
    pub year: u16,

    /// Per-class emission results.
    pub classes: Vec<ClassResult>,

    /// Total area across all classes (hectares).
    pub total_area_ha: f64,

    /// Total net emissions (tCO₂e).
    /// Positive = net emitter, Negative = net sink.
    pub total_emission_tco2e: f64,

    /// Total features submitted.
    pub total_features: u32,

    /// Features that matched a factor.
    pub classified_features: u32,

    /// Features skipped (no matching factor).
    pub skipped_features: u32,

    /// ISO 8601 timestamp of calculation.
    pub calculated_at: String,

    /// Name of the auditor (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auditor: Option<String>,

    /// Methodology used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub methodology: Option<String>,

    // ── Multi-gas extension ──
    /// Total emissions broken down by greenhouse gas.
    #[serde(default)]
    pub gas_summary: GasBreakdown,

    /// Propagated total uncertainty (± tCO₂e).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncertainty_total_tco2e: Option<f64>,

    /// Audit trail for factor provenance and verification.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub audit_trail: Vec<AuditEntry>,

    /// Emissions by GHG Protocol scope (1/2/3).
    #[serde(default, skip_serializing_if = "ScopeSummary::is_empty")]
    pub scope_summary: ScopeSummary,
}

// ── Class Result ──────────────────────────────────────────────

/// Result for a single landcover class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassResult {
    /// Landcover class name (e.g., "forest", "grassland").
    pub landcover_class: String,

    /// Total area in hectares.
    pub area_ha: f64,

    /// Emission factor applied (tCO₂e/ha/yr).
    pub factor_value: f64,

    /// Total emissions = area × factor (tCO₂e).
    pub emission_tco2e: f64,

    /// Source of the emission factor.
    pub factor_source: FactorSourceUnit,

    /// Number of features in this class.
    pub feature_count: u32,

    // ── Multi-gas extension ──
    /// Per-gas breakdown for this class (tCO₂e each).
    #[serde(default, skip_serializing_if = "GasBreakdown::is_empty")]
    pub gas_breakdown: GasBreakdown,

    /// Propagated uncertainty for this class (± tCO₂e).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncertainty_tco2e: Option<f64>,

    // ── Scope extension ──
    /// GHG Protocol scope (1/2/3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<EmissionScope>,
}

// ── Factor Source ─────────────────────────────────────────────

/// Factor provenance information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorSourceUnit {
    /// Source identifier (e.g., "IPCC_2019").
    pub source: String,

    /// Unit of measurement (e.g., "tCO₂e/ha/yr").
    pub unit: String,

    /// GWP version used (if applicable).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub gwp_version: String,
}

// ── Report Methods ────────────────────────────────────────────

impl CarbonReport {
    /// Returns true if the AOI is a net carbon sink.
    pub fn is_net_sink(&self) -> bool {
        self.total_emission_tco2e < 0.0
    }

    /// Returns true if the AOI is a net carbon emitter.
    pub fn is_net_emitter(&self) -> bool {
        self.total_emission_tco2e > 0.0
    }

    /// Get classes sorted by absolute emission (largest impact first).
    pub fn top_classes(&self, n: usize) -> &[ClassResult] {
        let end = n.min(self.classes.len());
        &self.classes[..end]
    }

    /// Emission intensity: tCO₂e per hectare.
    pub fn emission_intensity(&self) -> f64 {
        if self.total_area_ha > 0.0 {
            self.total_emission_tco2e / self.total_area_ha
        } else {
            0.0
        }
    }

    /// Returns true if multi-gas breakdown is available.
    pub fn has_gas_breakdown(&self) -> bool {
        !self.gas_summary.is_empty()
    }

    /// Relative uncertainty (±%), if computed.
    pub fn relative_uncertainty_pct(&self) -> Option<f64> {
        self.uncertainty_total_tco2e.map(|u| {
            if self.total_emission_tco2e.abs() > f64::EPSILON {
                (u / self.total_emission_tco2e.abs()) * 100.0
            } else {
                0.0
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_net_sink_detection() {
        let report = CarbonReport {
            aoi_name: None,
            year: 2025,
            classes: vec![],
            total_area_ha: 100.0,
            total_emission_tco2e: -50.0,
            total_features: 0,
            classified_features: 0,
            skipped_features: 0,
            calculated_at: "2025-01-01T00:00:00Z".into(),
            auditor: None,
            methodology: None,
            gas_summary: GasBreakdown::default(),
            uncertainty_total_tco2e: None,
            audit_trail: vec![],
            scope_summary: ScopeSummary::default(),
        };
        assert!(report.is_net_sink());
        assert!(!report.is_net_emitter());
        assert!((report.emission_intensity() - -0.5).abs() < 0.01);
        assert!(!report.has_gas_breakdown());
    }

    #[test]
    fn test_gas_breakdown_total() {
        let bd = GasBreakdown {
            co2_tco2e: -10.0,
            ch4_tco2e: 2.8,
            n2o_tco2e: 0.8,
            other_tco2e: 0.0,
        };
        assert!((bd.total() - -6.4).abs() < 0.01);
    }

    #[test]
    fn test_gas_breakdown_merge() {
        let mut a = GasBreakdown {
            co2_tco2e: 5.0,
            ..Default::default()
        };
        let b = GasBreakdown {
            ch4_tco2e: 3.0,
            n2o_tco2e: 1.0,
            ..Default::default()
        };
        a.merge(&b);
        assert!((a.co2_tco2e - 5.0).abs() < 0.001);
        assert!((a.ch4_tco2e - 3.0).abs() < 0.001);
        assert!((a.n2o_tco2e - 1.0).abs() < 0.001);
        assert!((a.total() - 9.0).abs() < 0.001);
    }

    #[test]
    fn test_gas_breakdown_from_contributions() {
        let contributions = vec![
            (GreenhouseGas::CO2, -5.0),
            (GreenhouseGas::CH4, 2.8),
            (GreenhouseGas::N2O, 0.5),
            (GreenhouseGas::SF6, 1.0),
        ];
        let bd = GasBreakdown::from_gas_contributions(&contributions);
        assert!((bd.co2_tco2e - -5.0).abs() < 0.001);
        assert!((bd.ch4_tco2e - 2.8).abs() < 0.001);
        assert!((bd.n2o_tco2e - 0.5).abs() < 0.001);
        assert!((bd.other_tco2e - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_relative_uncertainty() {
        let report = CarbonReport {
            aoi_name: None,
            year: 2025,
            classes: vec![],
            total_area_ha: 100.0,
            total_emission_tco2e: -100.0,
            total_features: 0,
            classified_features: 0,
            skipped_features: 0,
            calculated_at: "".into(),
            auditor: None,
            methodology: None,
            gas_summary: GasBreakdown::default(),
            uncertainty_total_tco2e: Some(30.0),
            audit_trail: vec![],
            scope_summary: ScopeSummary::default(),
        };
        let pct = report.relative_uncertainty_pct().unwrap();
        assert!((pct - 30.0).abs() < 0.01);
    }
}
