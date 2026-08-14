use super::fuel::{FuelType, GridEmissionFactor};
use super::gas::{EmissionScope, GasFactor, GwpVersion, gwp100};
use serde::{Deserialize, Serialize};

/// A single emission factor entry for a landcover class or activity.
///
/// Supports three modes:
/// 1. **Land-use** (backward-compat): single `factor_value` in tCO₂e/ha/yr.
/// 2. **Multi-gas**: `gas_factors` vector with per-gas factors + GWP conversion.
/// 3. **Industrial**: fuel combustion parameters or grid electricity factors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmissionFactor {
    // ── Identity & Metadata ──
    /// Activity category (landcover class or activity type).
    pub category: String,
    /// Optional subcategory for finer matching.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subcategory: Option<String>,
    /// Source of the factor (e.g., "IPCC_2019", "MEE_2023").
    #[serde(default)]
    pub source: String,
    /// Geographic region code (e.g., "CN-51", None = global).
    #[serde(default)]
    pub region: Option<String>,

    // ── Value & Unit ──
    /// Total emission factor value in tCO₂e per activity unit.
    pub factor_value: f64,
    /// Unit of measurement.
    #[serde(default = "default_unit")]
    pub unit: String,

    // ── Temporal Validity ──
    /// Valid from year (inclusive).
    #[serde(default)]
    pub valid_from_year: i32,
    /// Valid to year (inclusive, None = no expiry).
    #[serde(default)]
    pub valid_to_year: Option<i32>,

    // ── Multi-Gas Breakdown ──
    /// Per-gas emission factors.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gas_factors: Vec<GasFactor>,
    /// Overall uncertainty as ± percentage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncertainty_pct: Option<f64>,

    // ── Activity Classification ──
    /// GHG Protocol emission scope (1/2/3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<EmissionScope>,
    /// Activity data type hint: "landuse", "fuel", "electricity", "material", "transport".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity_type: Option<String>,

    // ── Fuel Combustion (Scope 1) ──
    /// Fuel type (for Scope 1 combustion).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuel_type: Option<FuelType>,
    /// Custom Net Calorific Value override (GJ/unit).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ncv_override: Option<f64>,
    /// Custom carbon content override (tC/TJ).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cc_override: Option<f64>,
    /// Custom oxidation rate override (0–1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ox_override: Option<f64>,

    // ── Grid Electricity (Scope 2) ──
    /// Grid emission factor for electricity (tCO₂/MWh).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid_ef: Option<f64>,
}

fn default_unit() -> String {
    "tCO₂e/ha/yr".into()
}

impl EmissionFactor {
    /// Create a new emission factor with minimal required fields (backward compat).
    pub fn new(category: impl Into<String>, factor_value: f64, source: impl Into<String>) -> Self {
        Self {
            category: category.into(),
            factor_value,
            source: source.into(),
            subcategory: None,
            unit: "tCO₂e/ha/yr".into(),
            valid_from_year: 2000,
            valid_to_year: None,
            region: None,
            gas_factors: Vec::new(),
            uncertainty_pct: None,
            scope: None,
            fuel_type: None,
            ncv_override: None,
            cc_override: None,
            ox_override: None,
            grid_ef: None,
            activity_type: None,
        }
    }

    /// Create a multi-gas emission factor.
    pub fn with_gases(
        category: impl Into<String>,
        source: impl Into<String>,
        gas_factors: Vec<GasFactor>,
        uncertainty_pct: Option<f64>,
    ) -> Self {
        let total_tco2e: f64 = gas_factors.iter().map(|g| g.to_tco2e()).sum();
        Self {
            factor_value: total_tco2e,
            gas_factors,
            uncertainty_pct,
            ..Self::new(category, total_tco2e, source)
        }
    }

    /// Create a fuel combustion emission factor (Scope 1).
    pub fn for_fuel(
        fuel_type: FuelType,
        quantity: f64, // in native units (t or 10⁴m³)
    ) -> Self {
        let co2 = fuel_type.compute_co2(quantity);
        let category = format!("fuel_{}", format!("{fuel_type:?}").to_lowercase());
        Self {
            category,
            factor_value: co2,
            source: "IPCC_2006".into(),
            unit: "tCO₂".into(),
            scope: Some(EmissionScope::Scope1),
            fuel_type: Some(fuel_type),
            activity_type: Some("fuel".into()),
            ..Self::new("fuel", co2, "IPCC_2006")
        }
    }

    /// Create an electricity emission factor (Scope 2).
    pub fn for_electricity(kwh: f64, grid_region: Option<&str>) -> Self {
        let grid = if let Some(region) = grid_region {
            GridEmissionFactor::for_china_region(region, 2023)
        } else {
            GridEmissionFactor {
                region: "CN".into(),
                factor_tco2_per_mwh: GridEmissionFactor::CN_2023,
                year: 2023,
                source: "MEE_2023".into(),
            }
        };
        let ef_mwh = grid.factor_tco2_per_mwh; // tCO₂/MWh
        let ef_kwh = ef_mwh / 1000.0; // tCO₂/kWh
        let total = kwh * ef_kwh;
        Self {
            category: "electricity".into(),
            factor_value: total,
            source: grid.source,
            unit: "tCO₂".into(),
            scope: Some(EmissionScope::Scope2),
            grid_ef: Some(ef_kwh),
            activity_type: Some("electricity".into()),
            ..Self::new("electricity", total, "MEE_2023")
        }
    }

    /// Returns true if this factor is valid for the given year.
    pub fn is_valid_for_year(&self, year: i32) -> bool {
        year >= self.valid_from_year && self.valid_to_year.is_none_or(|to| year <= to)
    }

    /// Returns true if this is a carbon sink (negative emission factor).
    pub fn is_sink(&self) -> bool {
        self.factor_value < 0.0
    }

    /// Returns true if this factor has per-gas breakdown data.
    pub fn has_gas_breakdown(&self) -> bool {
        !self.gas_factors.is_empty()
    }

    /// Compute CO₂e from gas factors using a specific GWP version.
    pub fn compute_tco2e(&self, version: GwpVersion) -> f64 {
        if self.gas_factors.is_empty() {
            self.factor_value
        } else {
            self.gas_factors
                .iter()
                .map(|g| g.to_tco2e_with_gwp(gwp100(g.gas, version)))
                .sum()
        }
    }
}
