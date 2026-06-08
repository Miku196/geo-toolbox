//! Carbon report types — the output of a calculation.

use serde::{Serialize, Deserialize};

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
}

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
}

/// Factor provenance information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorSourceUnit {
    /// Source identifier (e.g., "IPCC_2019").
    pub source: String,

    /// Unit of measurement (e.g., "tCO₂e/ha/yr").
    pub unit: String,
}

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
        };
        assert!(report.is_net_sink());
        assert!(!report.is_net_emitter());
        assert!((report.emission_intensity() - -0.5).abs() < 0.01);
    }
}
