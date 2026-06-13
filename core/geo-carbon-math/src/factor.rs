//! Emission factor types — multi-gas, GWP-aware, with uncertainty.
//!
//! Extends the simple tCO₂e/ha/yr model to the full GHG Protocol framework:
//! 1. Per-gas emission factors (CO₂, CH₄, N₂O, HFCs, PFCs, SF₆, NF₃)
//! 2. GWP conversion to CO₂-equivalent
//! 3. Uncertainty range (±X%) for Monte Carlo propagation

use serde::{Serialize, Deserialize};

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
    /// Human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            GreenhouseGas::CO2 => "Carbon dioxide",
            GreenhouseGas::CH4 => "Methane",
            GreenhouseGas::N2O => "Nitrous oxide",
            GreenhouseGas::HFCs => "Hydrofluorocarbons",
            GreenhouseGas::PFCs => "Perfluorocarbons",
            GreenhouseGas::SF6 => "Sulfur hexafluoride",
            GreenhouseGas::NF3 => "Nitrogen trifluoride",
        }
    }

    /// Chemical formula.
    pub fn formula(&self) -> &'static str {
        match self {
            GreenhouseGas::CO2 => "CO₂",
            GreenhouseGas::CH4 => "CH₄",
            GreenhouseGas::N2O => "N₂O",
            GreenhouseGas::HFCs => "HFCs",
            GreenhouseGas::PFCs => "PFCs",
            GreenhouseGas::SF6 => "SF₆",
            GreenhouseGas::NF3 => "NF₃",
        }
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

// ── Fuel Types & Combustion Parameters ────────────────────────

/// Fuel type with IPCC default combustion parameters.
///
/// NCV = Net Calorific Value, CC = Carbon Content, Ox = Oxidation Rate.
/// Values sourced from IPCC 2006 Guidelines and China national inventory.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FuelType {
    /// Raw coal (anthracite / bituminous)
    RawCoal,
    /// Cleaned/washed coal
    CleanedCoal,
    /// Coke
    Coke,
    /// Crude oil
    CrudeOil,
    /// Gasoline (motor petrol)
    Gasoline,
    /// Diesel oil
    Diesel,
    /// Fuel oil (heavy)
    FuelOil,
    /// Liquefied petroleum gas (LPG)
    LPG,
    /// Natural gas
    NaturalGas,
    /// Coke oven gas
    CokeOvenGas,
    /// Blast furnace gas
    BlastFurnaceGas,
    /// Biomass (wood, straw, etc.)
    Biomass,
    /// Other/unspecified fuel
    OtherFuel,
}

impl FuelType {
    /// Fuel label (Chinese-friendly).
    pub fn label(&self) -> &'static str {
        match self {
            FuelType::RawCoal => "Raw Coal / 原煤",
            FuelType::CleanedCoal => "Cleaned Coal / 洗精煤",
            FuelType::Coke => "Coke / 焦炭",
            FuelType::CrudeOil => "Crude Oil / 原油",
            FuelType::Gasoline => "Gasoline / 汽油",
            FuelType::Diesel => "Diesel / 柴油",
            FuelType::FuelOil => "Fuel Oil / 燃料油",
            FuelType::LPG => "LPG / 液化石油气",
            FuelType::NaturalGas => "Natural Gas / 天然气",
            FuelType::CokeOvenGas => "Coke Oven Gas / 焦炉煤气",
            FuelType::BlastFurnaceGas => "Blast Furnace Gas / 高炉煤气",
            FuelType::Biomass => "Biomass / 生物质",
            FuelType::OtherFuel => "Other Fuel / 其他燃料",
        }
    }

    /// Net Calorific Value (GJ per unit). Unit depends on fuel type.
    pub fn default_ncv(&self) -> f64 {
        match self {
            FuelType::RawCoal => 20.908,    // GJ/t
            FuelType::CleanedCoal => 26.344,
            FuelType::Coke => 28.435,
            FuelType::CrudeOil => 41.816,   // GJ/t
            FuelType::Gasoline => 43.070,
            FuelType::Diesel => 42.652,
            FuelType::FuelOil => 41.816,
            FuelType::LPG => 50.179,
            FuelType::NaturalGas => 389.31,  // GJ/10⁴m³
            FuelType::CokeOvenGas => 167.26, // GJ/10⁴m³
            FuelType::BlastFurnaceGas => 33.35,
            FuelType::Biomass => 17.460,     // GJ/t
            FuelType::OtherFuel => 20.0,
        }
    }

    /// Carbon content per unit energy (tC/TJ).
    pub fn default_carbon_content(&self) -> f64 {
        match self {
            FuelType::RawCoal => 26.37,
            FuelType::CleanedCoal => 25.41,
            FuelType::Coke => 29.42,
            FuelType::CrudeOil => 20.08,
            FuelType::Gasoline => 18.90,
            FuelType::Diesel => 20.20,
            FuelType::FuelOil => 21.10,
            FuelType::LPG => 17.20,
            FuelType::NaturalGas => 15.32,
            FuelType::CokeOvenGas => 13.58,
            FuelType::BlastFurnaceGas => 70.80,
            FuelType::Biomass => 27.30,
            FuelType::OtherFuel => 20.0,
        }
    }

    /// Oxidation rate (fraction, 0–1).
    pub fn default_oxidation_rate(&self) -> f64 {
        match self {
            FuelType::RawCoal | FuelType::CleanedCoal | FuelType::Coke => 0.94,
            FuelType::CrudeOil | FuelType::Gasoline | FuelType::Diesel
                | FuelType::FuelOil | FuelType::LPG => 0.98,
            FuelType::NaturalGas | FuelType::CokeOvenGas | FuelType::BlastFurnaceGas => 0.99,
            FuelType::Biomass => 0.90,
            FuelType::OtherFuel => 0.95,
        }
    }

    /// Unit label for the fuel quantity.
    pub fn unit(&self) -> &'static str {
        match self {
            FuelType::RawCoal | FuelType::CleanedCoal | FuelType::Coke
                | FuelType::CrudeOil | FuelType::Gasoline | FuelType::Diesel
                | FuelType::FuelOil | FuelType::LPG | FuelType::Biomass
                | FuelType::OtherFuel => "t",
            FuelType::NaturalGas | FuelType::CokeOvenGas
                | FuelType::BlastFurnaceGas => "10⁴m³",
        }
    }

    /// Compute fuel CO₂ emission: qty × NCV × CC × Ox × (44/12).
    pub fn compute_co2(&self, quantity: f64) -> f64 {
        let ncv = self.default_ncv();
        let cc = self.default_carbon_content();
        let ox = self.default_oxidation_rate();
        // tCO₂ = qty × NCV(GJ/unit) × CC(tC/TJ) × Ox × (44/12)
        quantity * ncv * cc * ox / 1000.0 * (44.0 / 12.0)
    }
}

/// Grid emission factor for purchased electricity (Scope 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridEmissionFactor {
    /// Grid region (e.g., "CN-North", "CN-East", "US-WECC")
    pub region: String,
    /// Emission factor in tCO₂/MWh
    pub factor_tco2_per_mwh: f64,
    /// Year of the factor
    pub year: u16,
    /// Source (e.g., "MEE_2023", "EPA_eGRID")
    pub source: String,
}

impl GridEmissionFactor {
    /// China national average grid emission factor (2023, MEE).
    pub const CN_2023: f64 = 0.5703; // tCO₂/MWh
    /// China regional — North China grid (2023).
    pub const CN_NORTH_2023: f64 = 0.7204;
    /// China regional — East China grid (2023).
    pub const CN_EAST_2023: f64 = 0.5850;
    /// China regional — South China grid (2023).
    pub const CN_SOUTH_2023: f64 = 0.3907;
    /// US national average (eGRID 2022).
    pub const US_2022: f64 = 0.3719;

    /// Create a grid factor from a region code and year.
    pub fn for_china_region(region: &str, year: u16) -> Self {
        let factor = match region.to_lowercase().as_str() {
            "cn-north" | "north" => Self::CN_NORTH_2023,
            "cn-east" | "east" => Self::CN_EAST_2023,
            "cn-south" | "south" => Self::CN_SOUTH_2023,
            _ => Self::CN_2023,
        };
        Self { region: region.to_string(), factor_tco2_per_mwh: factor, year, source: "MEE_2023".into() }
    }
}

// ── GWP (Global Warming Potential) ─────────────────────────────

/// IPCC Assessment Report version for GWP values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GwpVersion {
    /// IPCC Fourth Assessment Report (2007)
    AR4,
    /// IPCC Fifth Assessment Report (2014) — **default**
    AR5,
    /// IPCC Sixth Assessment Report (2021)
    AR6,
}

impl Default for GwpVersion {
    fn default() -> Self { GwpVersion::AR5 }
}

/// Return the 100-year Global Warming Potential for a greenhouse gas.
///
/// Values sourced from IPCC Assessment Reports.
/// For HFCs/PFCs groups, returns a conservative average.
pub fn gwp100(gas: GreenhouseGas, version: GwpVersion) -> f64 {
    match version {
        GwpVersion::AR4 => gwp_ar4(gas),
        GwpVersion::AR5 => gwp_ar5(gas),
        GwpVersion::AR6 => gwp_ar6(gas),
    }
}

fn gwp_ar5(gas: GreenhouseGas) -> f64 {
    match gas {
        GreenhouseGas::CO2  => 1.0,
        GreenhouseGas::CH4  => 28.0,    // fossil; non-fossil is 27.2
        GreenhouseGas::N2O  => 265.0,
        GreenhouseGas::HFCs => 1400.0,  // aggregate
        GreenhouseGas::PFCs => 7400.0,  // aggregate
        GreenhouseGas::SF6  => 23500.0,
        GreenhouseGas::NF3  => 16100.0,
    }
}

fn gwp_ar4(gas: GreenhouseGas) -> f64 {
    match gas {
        GreenhouseGas::CO2  => 1.0,
        GreenhouseGas::CH4  => 25.0,
        GreenhouseGas::N2O  => 298.0,
        GreenhouseGas::HFCs => 1600.0,
        GreenhouseGas::PFCs => 8300.0,
        GreenhouseGas::SF6  => 22800.0,
        GreenhouseGas::NF3  => 17200.0,
    }
}

fn gwp_ar6(gas: GreenhouseGas) -> f64 {
    match gas {
        GreenhouseGas::CO2  => 1.0,
        GreenhouseGas::CH4  => 27.0,    // fossil; non-fossil is 27.2
        GreenhouseGas::N2O  => 273.0,
        GreenhouseGas::HFCs => 1500.0,  // aggregate
        GreenhouseGas::PFCs => 7800.0,  // aggregate
        GreenhouseGas::SF6  => 24300.0,
        GreenhouseGas::NF3  => 17400.0,
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

// ── Emission Factor (extended) ────────────────────────────────

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

fn default_unit() -> String { "tCO₂e/ha/yr".into() }

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
        Self { factor_value: total_tco2e, gas_factors, uncertainty_pct, ..Self::new(category, total_tco2e, source) }
    }

    /// Create a fuel combustion emission factor (Scope 1).
    pub fn for_fuel(
        fuel_type: FuelType,
        quantity: f64,  // in native units (t or 10⁴m³)
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
        let ef_mwh = grid.factor_tco2_per_mwh;  // tCO₂/MWh
        let ef_kwh = ef_mwh / 1000.0;           // tCO₂/kWh
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
        year >= self.valid_from_year
            && self.valid_to_year.is_none_or(|to| year <= to)
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
            self.gas_factors.iter()
                .map(|g| g.to_tco2e_with_gwp(gwp100(g.gas, version)))
                .sum()
        }
    }
}

// ── CSV Loader (split into parse / row / orchestrate) ─────────

/// Column index mapping computed from CSV headers once.
struct CsvColumnIndices {
    category: usize,
    value: Option<usize>,
    source: Option<usize>,
    subcategory: Option<usize>,
    unit: Option<usize>,
    valid_from_year: Option<usize>,
    valid_to_year: Option<usize>,
    region: Option<usize>,
    uncertainty_pct: Option<usize>,
    scope: Option<usize>,
    activity_type: Option<usize>,
    fuel_type: Option<usize>,
    grid_ef: Option<usize>,
    ncv: Option<usize>,
    cc: Option<usize>,
    ox: Option<usize>,
    gas_columns: Vec<(GreenhouseGas, Option<usize>)>,
}

/// Parse column indices from CSV headers.
fn parse_csv_columns(headers: &[String]) -> Result<CsvColumnIndices, String> {
    let idx = |name: &str| headers.iter().position(|h| h == name);

    let gas_columns = vec![
        (GreenhouseGas::CO2,  idx("gas_co2_factor")),
        (GreenhouseGas::CH4,  idx("gas_ch4_factor")),
        (GreenhouseGas::N2O,  idx("gas_n2o_factor")),
        (GreenhouseGas::HFCs, idx("gas_hfcs_factor")),
        (GreenhouseGas::PFCs, idx("gas_pfcs_factor")),
        (GreenhouseGas::SF6,  idx("gas_sf6_factor")),
        (GreenhouseGas::NF3,  idx("gas_nf3_factor")),
    ];

    Ok(CsvColumnIndices {
        category: idx("category").ok_or("CSV must have 'category' column")?,
        value: idx("factor_value"),
        source: idx("source"),
        subcategory: idx("subcategory"),
        unit: idx("unit"),
        valid_from_year: idx("valid_from_year"),
        valid_to_year: idx("valid_to_year"),
        region: idx("region"),
        uncertainty_pct: idx("uncertainty_pct"),
        scope: idx("scope"),
        activity_type: idx("activity_type"),
        fuel_type: idx("fuel_type"),
        grid_ef: idx("grid_ef"),
        ncv: idx("ncv"),
        cc: idx("cc"),
        ox: idx("ox"),
        gas_columns,
    })
}

/// Parse a fuel type string from a CSV cell.
fn parse_fuel_type(s: &str) -> Option<FuelType> {
    match s.to_lowercase().as_str() {
        "rawcoal" | "raw_coal" | "原煤" => Some(FuelType::RawCoal),
        "cleanedcoal" | "cleaned_coal" | "洗精煤" => Some(FuelType::CleanedCoal),
        "coke" | "焦炭" => Some(FuelType::Coke),
        "crudeoil" | "crude_oil" | "原油" => Some(FuelType::CrudeOil),
        "gasoline" | "汽油" => Some(FuelType::Gasoline),
        "diesel" | "柴油" => Some(FuelType::Diesel),
        "fueloil" | "fuel_oil" | "燃料油" => Some(FuelType::FuelOil),
        "lpg" | "液化石油气" => Some(FuelType::LPG),
        "naturalgas" | "natural_gas" | "天然气" => Some(FuelType::NaturalGas),
        "cokeovengas" | "coke_oven_gas" | "焦炉煤气" => Some(FuelType::CokeOvenGas),
        "blastfurnacegas" | "blast_furnace_gas" | "高炉煤气" => Some(FuelType::BlastFurnaceGas),
        "biomass" | "生物质" => Some(FuelType::Biomass),
        _ => None,
    }
}

/// Parse a single CSV record into an EmissionFactor.
fn parse_emission_factor_row(
    record: &csv::StringRecord,
    cols: &CsvColumnIndices,
) -> Result<EmissionFactor, String> {
    let get = |idx: Option<usize>| -> Option<&str> {
        idx.and_then(|i| record.get(i))
    };
    let get_f64 = |idx: Option<usize>| -> Option<Result<f64, String>> {
        let s = record.get(idx?)?;
        Some(s.parse().map_err(|e| format!("Bad float '{s}': {e}")))
    };

    let category = record.get(cols.category).ok_or("Missing category")?.to_string();
    let has_multi_gas = cols.gas_columns.iter().any(|(_, idx)| idx.is_some());

    let factor_value: f64 = if let Some(vi) = cols.value {
        record.get(vi).ok_or("Missing factor_value")?
            .parse().map_err(|e| format!("Bad factor_value: {e}"))?
    } else if has_multi_gas {
        0.0 // computed from gas columns below
    } else {
        return Err("CSV must have 'factor_value' or gas columns".into());
    };

    let source = get(cols.source).unwrap_or("IPCC_2019").to_string();
    let subcategory = get(cols.subcategory).map(|s| s.to_string());
    let unit = get(cols.unit).unwrap_or("tCO₂e/ha/yr").to_string();
    let valid_from_year: i32 = get(cols.valid_from_year)
        .unwrap_or("2000").parse().unwrap_or(2000);
    let valid_to_year: Option<i32> = get(cols.valid_to_year)
        .and_then(|s| s.parse().ok());
    let region = get(cols.region).map(|s| s.to_string());
    let uncertainty_pct: Option<f64> = get_f64(cols.uncertainty_pct).transpose()?;
    let activity_type = get(cols.activity_type).map(|s| s.to_string());
    let grid_ef: Option<f64> = get_f64(cols.grid_ef).transpose()?;
    let ncv_override: Option<f64> = get_f64(cols.ncv).transpose()?;
    let cc_override: Option<f64> = get_f64(cols.cc).transpose()?;
    let ox_override: Option<f64> = get_f64(cols.ox).transpose()?;

    // Scope
    let scope = get(cols.scope).and_then(|s| match s.to_lowercase().as_str() {
        "scope1" | "1" => Some(EmissionScope::Scope1),
        "scope2" | "2" => Some(EmissionScope::Scope2),
        "scope3" | "3" => Some(EmissionScope::Scope3),
        _ => None,
    });

    // Fuel type
    let fuel_type = get(cols.fuel_type).and_then(parse_fuel_type);

    // Multi-gas columns
    let gas_factors: Vec<GasFactor> = cols.gas_columns.iter()
        .filter_map(|(gas, col_idx)| {
            let ci = (*col_idx)?;
            let val: f64 = get_f64(Some(ci))?.ok()?;
            if val == 0.0 { return None; }
            let unit_str = match gas {
                GreenhouseGas::CO2 => "kg CO₂/ha/yr",
                GreenhouseGas::CH4 => "kg CH₄/ha/yr",
                GreenhouseGas::N2O => "kg N₂O/ha/yr",
                GreenhouseGas::HFCs => "kg HFCs/ha/yr",
                GreenhouseGas::PFCs => "kg PFCs/ha/yr",
                GreenhouseGas::SF6 => "kg SF₆/ha/yr",
                GreenhouseGas::NF3 => "kg NF₃/ha/yr",
            };
            Some(GasFactor::land_use(*gas, val, unit_str))
        })
        .collect();

    let computed_value = if !gas_factors.is_empty() {
        let computed: f64 = gas_factors.iter().map(|g| g.to_tco2e()).sum();
        if cols.value.is_none() { computed } else { factor_value }
    } else {
        factor_value
    };

    Ok(EmissionFactor {
        category,
        factor_value: computed_value,
        source,
        subcategory,
        unit,
        valid_from_year,
        valid_to_year,
        region,
        gas_factors,
        uncertainty_pct,
        scope,
        fuel_type,
        ncv_override,
        cc_override,
        ox_override,
        grid_ef,
        activity_type,
    })
}

/// Load emission factors from CSV text (header-based, order-independent).
///
/// ### Simple mode (backward compat):
/// ```csv
/// category,factor_value,source
/// forest,-5.0,IPCC_2019
/// ```
///
/// ### Multi-gas mode:
/// ```csv
/// category,source,gas_ch4_factor,gas_n2o_factor,uncertainty_pct
/// rice_paddy,IPCC_2019,0.0,150.0,3.0,40.0
/// ```
///
/// ### Industrial mode:
/// ```csv
/// category,source,scope,activity_type,fuel_type,grid_ef,ncv,cc,ox,uncertainty_pct
/// coal_boiler,IPCC_2006,scope1,fuel,RawCoal,,20.9,26.37,0.94,15.0
/// grid_power,MEE_2023,scope2,electricity,,0.5703,,,,10.0
/// ```
/// Supports columns: `gas_CO2_factor`, `gas_CH4_factor`, `gas_N2O_factor`,
/// `gas_HFCs_factor`, `gas_PFCs_factor`, `gas_SF6_factor`, `gas_NF3_factor`.
/// Industrial columns: `scope`, `activity_type`, `fuel_type`, `grid_ef`,
/// `ncv`, `cc`, `ox`.
pub fn load_factors_from_csv(csv_text: &str) -> Result<Vec<EmissionFactor>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(csv_text.as_bytes());

    let headers: Vec<String> = reader.headers()
        .map_err(|e| format!("CSV headers: {e}"))?
        .iter()
        .map(|h| h.to_lowercase().trim().to_string())
        .collect();

    let cols = parse_csv_columns(&headers)?;

    let mut factors = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| format!("CSV row: {e}"))?;
        factors.push(parse_emission_factor_row(&record, &cols)?);
    }

    if factors.is_empty() {
        return Err("CSV parsed but no emission factors found".into());
    }

    Ok(factors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emission_factor_creation() {
        let ef = EmissionFactor::new("forest", 5.0, "IPCC_2019");
        assert_eq!(ef.category, "forest");
        assert_eq!(ef.factor_value, 5.0);
        assert!(ef.is_valid_for_year(2025));
        assert!(!ef.is_sink());
        assert!(!ef.has_gas_breakdown());
    }

    #[test]
    fn test_sink_detection() {
        let ef = EmissionFactor::new("grassland", -1.0, "IPCC");
        assert!(ef.is_sink());
    }

    #[test]
    fn test_valid_for_year() {
        let mut ef = EmissionFactor::new("crop", 2.0, "TEST");
        ef.valid_from_year = 2020;
        ef.valid_to_year = Some(2030);
        assert!(!ef.is_valid_for_year(2019));
        assert!(ef.is_valid_for_year(2025));
        assert!(!ef.is_valid_for_year(2031));
    }

    #[test]
    fn test_multi_gas_creation() {
        let ef = EmissionFactor::with_gases(
            "rice_paddy",
            "IPCC_2019",
            vec![
                GasFactor::land_use(GreenhouseGas::CH4, 150.0, "kg CH₄/ha/yr"),
                GasFactor::land_use(GreenhouseGas::N2O, 3.0, "kg N₂O/ha/yr"),
            ],
            Some(40.0),
        );
        assert!(ef.has_gas_breakdown());
        assert_eq!(ef.gas_factors.len(), 2);
        // CH4: 150 × 28 / 1000 = 4.2, N2O: 3 × 265 / 1000 = 0.795, total ≈ 4.995
        let total = ef.factor_value;
        assert!((total - 4.995).abs() < 0.01, "Expected ~4.995, got {total}");
        assert_eq!(ef.uncertainty_pct, Some(40.0));
    }

    #[test]
    fn test_gwp_values() {
        assert_eq!(gwp100(GreenhouseGas::CO2, GwpVersion::AR5), 1.0);
        assert_eq!(gwp100(GreenhouseGas::CH4, GwpVersion::AR5), 28.0);
        assert_eq!(gwp100(GreenhouseGas::N2O, GwpVersion::AR5), 265.0);
        assert_eq!(gwp100(GreenhouseGas::SF6, GwpVersion::AR5), 23500.0);
    }

    #[test]
    fn test_gwp_ar4_vs_ar5() {
        // AR4 had higher GWP for N₂O (298 vs 265)
        assert!(gwp100(GreenhouseGas::N2O, GwpVersion::AR4) > gwp100(GreenhouseGas::N2O, GwpVersion::AR5));
    }

    #[test]
    fn test_gas_factor_to_tco2e() {
        let gf = GasFactor::land_use(GreenhouseGas::CH4, 100.0, "kg CH₄/ha/yr");
        // 100 kg CH₄ × 28 GWP / 1000 = 2.8 tCO₂e
        assert!((gf.to_tco2e() - 2.8).abs() < 0.001);
    }

    #[test]
    fn test_load_from_csv_simple() {
        let csv = "category,factor_value,source\nforest,5.0,IPCC_2019\ngrassland,-1.0,IPCC_2019\n";
        let factors = load_factors_from_csv(csv).unwrap();
        assert_eq!(factors.len(), 2);
        assert_eq!(factors[0].factor_value, 5.0);
        assert_eq!(factors[1].factor_value, -1.0);
        assert!(!factors[0].has_gas_breakdown());
    }

    #[test]
    fn test_load_from_csv_multi_gas() {
        let csv = "category,source,gas_ch4_factor,gas_n2o_factor,uncertainty_pct\nrice_paddy,IPCC_2019,150.0,3.0,40.0\nwetland,IPCC_2019,200.0,1.0,50.0\n";
        let factors = load_factors_from_csv(csv).unwrap();
        assert_eq!(factors.len(), 2);
        assert!(factors[0].has_gas_breakdown());
        assert_eq!(factors[0].gas_factors.len(), 2);
        assert_eq!(factors[0].uncertainty_pct, Some(40.0));
        // CH4: 150×28/1000=4.2, N2O: 3×265/1000=0.795, total≈4.995
        assert!((factors[0].factor_value - 4.995).abs() < 0.01);
    }

    #[test]
    fn test_load_from_csv_missing_columns() {
        let csv = "name,value\nforest,5.0\n";
        assert!(load_factors_from_csv(csv).is_err());
    }

    #[test]
    fn test_gwp_version_default() {
        assert_eq!(GwpVersion::default(), GwpVersion::AR5);
    }
}
