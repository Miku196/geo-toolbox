use serde::{Deserialize, Serialize};

// ── Greenhouse Gas Types ──────────────────────────────────────────

/// The seven greenhouse gases covered by the Kyoto Protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GreenhouseGas {
    /// Carbon dioxide — baseline GWP = 1
    #[serde(rename = "CO2")]
    CO2,
    /// Methane — GWP 28 (AR5, 100yr)
    #[serde(rename = "CH4")]
    CH4,
    /// Nitrous oxide — GWP 265 (AR5, 100yr)
    #[serde(rename = "N2O")]
    N2O,
    /// Hydrofluorocarbons (group, use specific HFC subtype for exact GWP)
    #[serde(rename = "HFCs")]
    HFCs,
    /// Perfluorocarbons (group)
    #[serde(rename = "PFCs")]
    PFCs,
    /// Sulfur hexafluoride
    #[serde(rename = "SF6")]
    SF6,
    /// Nitrogen trifluoride
    #[serde(rename = "NF3")]
    NF3,
}

impl GreenhouseGas {
    /// Meta: returns (human-readable name, chemical formula).
    fn meta(&self) -> (&'static str, &'static str) {
        match self {
            GreenhouseGas::CO2 => ("Carbon dioxide", "CO₂"),
            GreenhouseGas::CH4 => ("Methane", "CH₄"),
            GreenhouseGas::N2O => ("Nitrous oxide", "N₂O"),
            GreenhouseGas::HFCs => ("Hydrofluorocarbons", "HFCs"),
            GreenhouseGas::PFCs => ("Perfluorocarbons", "PFCs"),
            GreenhouseGas::SF6 => ("Sulfur hexafluoride", "SF₆"),
            GreenhouseGas::NF3 => ("Nitrogen trifluoride", "NF₃"),
        }
    }

    /// Human-readable name.
    pub fn name(&self) -> &'static str {
        self.meta().0
    }

    /// Chemical formula.
    pub fn formula(&self) -> &'static str {
        self.meta().1
    }
}

// ── Emission Scope (GHG Protocol) ─────────────────────────────

/// GHG Protocol emission scope classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmissionScope {
    /// Scope 1: Direct emissions from owned/controlled sources
    /// (fuel combustion, company vehicles, process emissions, fugitive)
    #[serde(rename = "scope1")]
    Scope1,
    /// Scope 2: Indirect emissions from purchased electricity/heat/steam/cooling
    #[serde(rename = "scope2")]
    Scope2,
    /// Scope 3: All other indirect emissions (value chain, employee travel, waste)
    #[serde(rename = "scope3")]
    Scope3,
}

impl EmissionScope {
    pub fn label(&self) -> &'static str {
        match self {
            EmissionScope::Scope1 => "Scope 1 — Direct Emissions",
            EmissionScope::Scope2 => "Scope 2 — Energy Indirect",
            EmissionScope::Scope3 => "Scope 3 — Value Chain",
        }
    }
}

// ── GWP (Global Warming Potential) ─────────────────────────────

/// IPCC Assessment Report version for GWP values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GwpVersion {
    /// IPCC Fourth Assessment Report (2007)
    AR4,
    /// IPCC Fifth Assessment Report (2014) — **default**
    #[default]
    AR5,
    /// IPCC Sixth Assessment Report (2021)
    AR6,
}

/// Return the 100-year Global Warming Potential for a greenhouse gas.
///
/// Values sourced from IPCC Assessment Reports.
/// For HFCs/PFCs groups, returns a conservative average.
pub fn gwp100(gas: GreenhouseGas, version: GwpVersion) -> f64 {
    match version {
        GwpVersion::AR4 => match gas {
            GreenhouseGas::CO2 => 1.0,
            GreenhouseGas::CH4 => 25.0,
            GreenhouseGas::N2O => 298.0,
            GreenhouseGas::HFCs => 1600.0,
            GreenhouseGas::PFCs => 8300.0,
            GreenhouseGas::SF6 => 22800.0,
            GreenhouseGas::NF3 => 17200.0,
        },
        GwpVersion::AR5 => match gas {
            GreenhouseGas::CO2 => 1.0,
            GreenhouseGas::CH4 => 28.0,
            GreenhouseGas::N2O => 265.0,
            GreenhouseGas::HFCs => 1400.0,
            GreenhouseGas::PFCs => 7400.0,
            GreenhouseGas::SF6 => 23500.0,
            GreenhouseGas::NF3 => 16100.0,
        },
        GwpVersion::AR6 => match gas {
            GreenhouseGas::CO2 => 1.0,
            GreenhouseGas::CH4 => 27.0,
            GreenhouseGas::N2O => 273.0,
            GreenhouseGas::HFCs => 1500.0,
            GreenhouseGas::PFCs => 7800.0,
            GreenhouseGas::SF6 => 24300.0,
            GreenhouseGas::NF3 => 17400.0,
        },
    }
}

// ── Gas Factor ─────────────────────────────────────────────────

/// Per-gas emission factor, representing the amount of a single GHG
/// emitted per unit of activity (e.g., kg CH₄ / ha / yr).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GasFactor {
    /// Which greenhouse gas.
    pub gas: GreenhouseGas,
    /// Emission factor value in its native unit.
    pub factor: f64,
    /// Unit (e.g., "kg CH₄/ha/yr", "g N₂O/m²/yr").
    pub unit: String,
    /// GWP version to use for CO₂e conversion.
    #[serde(default)]
    pub gwp_version: GwpVersion,
    /// Uncertainty as ± percentage (e.g., 30.0 = ±30%).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncertainty_pct: Option<f64>,
}

impl GasFactor {
    /// Create a gas factor for land-use emissions (per hectare per year).
    pub fn land_use(gas: GreenhouseGas, factor: f64, unit: impl Into<String>) -> Self {
        Self {
            gas,
            factor,
            unit: unit.into(),
            gwp_version: GwpVersion::default(),
            uncertainty_pct: None,
        }
    }

    /// Convert this gas factor to tCO₂e using GWP.
    /// Assumes the factor is in a unit that converts to kg gas (e.g., kg CH₄).
    pub fn to_tco2e(&self) -> f64 {
        let gwp = gwp100(self.gas, self.gwp_version);
        // Convert kg gas → tCO₂e: factor_value × GWP / 1000
        (self.factor * gwp) / 1000.0
    }

    /// Convert with explicit GWP override.
    pub fn to_tco2e_with_gwp(&self, custom_gwp: f64) -> f64 {
        (self.factor * custom_gwp) / 1000.0
    }
}
