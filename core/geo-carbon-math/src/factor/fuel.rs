use serde::{Deserialize, Serialize};

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
            FuelType::RawCoal => 20.908, // GJ/t
            FuelType::CleanedCoal => 26.344,
            FuelType::Coke => 28.435,
            FuelType::CrudeOil => 41.816, // GJ/t
            FuelType::Gasoline => 43.070,
            FuelType::Diesel => 42.652,
            FuelType::FuelOil => 41.816,
            FuelType::LPG => 50.179,
            FuelType::NaturalGas => 389.31,  // GJ/10⁴m³
            FuelType::CokeOvenGas => 167.26, // GJ/10⁴m³
            FuelType::BlastFurnaceGas => 33.35,
            FuelType::Biomass => 17.460, // GJ/t
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
            FuelType::CrudeOil
            | FuelType::Gasoline
            | FuelType::Diesel
            | FuelType::FuelOil
            | FuelType::LPG => 0.98,
            FuelType::NaturalGas | FuelType::CokeOvenGas | FuelType::BlastFurnaceGas => 0.99,
            FuelType::Biomass => 0.90,
            FuelType::OtherFuel => 0.95,
        }
    }

    /// Unit label for the fuel quantity.
    pub fn unit(&self) -> &'static str {
        match self {
            FuelType::RawCoal
            | FuelType::CleanedCoal
            | FuelType::Coke
            | FuelType::CrudeOil
            | FuelType::Gasoline
            | FuelType::Diesel
            | FuelType::FuelOil
            | FuelType::LPG
            | FuelType::Biomass
            | FuelType::OtherFuel => "t",
            FuelType::NaturalGas | FuelType::CokeOvenGas | FuelType::BlastFurnaceGas => "10⁴m³",
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
        Self {
            region: region.to_string(),
            factor_tco2_per_mwh: factor,
            year,
            source: "MEE_2023".into(),
        }
    }
}
