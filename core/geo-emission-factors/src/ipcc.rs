use geo_carbon_math::{
    gwp100, EmissionFactor, EmissionScope, FuelType, GasFactor, GreenhouseGas, GwpVersion,
};

/// IPCC Tier 1 default emission factor database.
///
/// Sources: IPCC 2006/2019 Refinement, IPCC AR5/AR6 GWP values.
pub struct IpccEfDb;

impl IpccEfDb {
    // ── Energy: Stationary Combustion ──

    /// Default CO₂ emission factor for a fuel type (kg CO₂/GJ NCV basis).
    pub fn fuel_co2(fuel: FuelType) -> f64 {
        fuel.compute_co2(1.0)
    }

    /// Full EmissionFactor for stationary combustion of a fuel.
    pub fn stationary_combustion(fuel: FuelType, quantity: f64) -> EmissionFactor {
        let _co2 = fuel.compute_co2(quantity);
        EmissionFactor::for_fuel(fuel, quantity)
    }

    // ── Energy: Electricity & Heat ──

    /// Global average grid emission factor (tCO₂/MWh).
    /// Source: IEA 2023 — 0.475 tCO₂/MWh world average.
    pub const GLOBAL_GRID_2023: f64 = 0.475;

    /// Factory default: use global average grid factor.
    pub fn grid_electricity(kwh: f64, grid_ef: Option<f64>) -> EmissionFactor {
        let ef = grid_ef.unwrap_or(0.475);
        let tco2 = kwh * ef / 1000.0;
        EmissionFactor {
            category: "electricity".into(),
            subcategory: Some("grid_supply".into()),
            source: "IPCC_2019".into(),
            region: Some("global".into()),
            factor_value: tco2,
            unit: "tCO₂".into(),
            valid_from_year: 2020,
            valid_to_year: None,
            gas_factors: vec![GasFactor::land_use(GreenhouseGas::CO2, tco2, "tCO₂")],
            uncertainty_pct: Some(30.0),
            scope: Some(EmissionScope::Scope2),
            activity_type: Some("purchased_electricity".into()),
            fuel_type: None,
            ncv_override: None,
            cc_override: None,
            ox_override: None,
            grid_ef: Some(ef),
        }
    }

    // ── Land Use / AFOLU ──

    /// Managed forest sink default: −5.0 tCO₂e/ha/yr (IPCC 2019, tropical/plantation).
    pub const FOREST_SINK: f64 = -5.0;
    /// Grassland sink default: −1.2 tCO₂e/ha/yr.
    pub const GRASSLAND_SINK: f64 = -1.2;
    /// Wetland sink default: −8.5 tCO₂e/ha/yr (peatland).
    pub const WETLAND_SINK: f64 = -8.5;
    /// Cropland source default: +0.5 tCO₂e/ha/yr.
    pub const CROPLAND_SOURCE: f64 = 0.5;
    /// Built-up land source default: +2.0 tCO₂e/ha/yr.
    pub const BUILT_UP_SOURCE: f64 = 2.0;

    /// Full EmissionFactor for land-use carbon flux.
    pub fn land_use_flux(class: &str, area_ha: f64) -> Option<EmissionFactor> {
        let (factor_value, label) = match class.to_lowercase().as_str() {
            "forest" => (Self::FOREST_SINK, "Forest land remaining forest land"),
            "grassland" => (Self::GRASSLAND_SINK, "Grassland remaining grassland"),
            "wetland" => (Self::WETLAND_SINK, "Wetland remaining wetland"),
            "cropland" | "crop" => (Self::CROPLAND_SOURCE, "Cropland remaining cropland"),
            "built_up" | "builtup" | "urban" => {
                (Self::BUILT_UP_SOURCE, "Settlements remaining settlements")
            }
            "bare" | "bareland" | "mining" => (0.0, "Other land remaining other land"),
            "water" => (0.0, "Water bodies"),
            _ => return None,
        };
        let total = factor_value * area_ha;
        Some(EmissionFactor {
            category: "land_use".into(),
            subcategory: Some(label.into()),
            source: "IPCC_2019".into(),
            region: Some("global".into()),
            factor_value: total,
            unit: "tCO₂e".into(),
            valid_from_year: 2015,
            valid_to_year: None,
            gas_factors: vec![GasFactor::land_use(GreenhouseGas::CO2, total, "tCO₂e")],
            uncertainty_pct: Some(if factor_value < 0.0 { 40.0 } else { 30.0 }),
            scope: Some(EmissionScope::Scope1),
            activity_type: Some("land_management".into()),
            fuel_type: None,
            ncv_override: None,
            cc_override: None,
            ox_override: None,
            grid_ef: None,
        })
    }

    // ── Agriculture ──

    /// Rice cultivation CH4 emission factor (kg CH₄/ha/day flooding).
    /// Default: 2.5 kg CH₄/ha/day for continuously flooded fields without organic amendment.
    pub const RICE_CH4_PER_HA_DAY: f64 = 2.5;

    /// Default N₂O emission from synthetic fertiliser (kg N₂O-N / kg N input).
    /// IPCC Tier 1: 0.01 kg N₂O-N / kg N (direct + indirect).
    pub const N2O_FERTILISER_EF: f64 = 0.01;

    /// Enteric fermentation EF (kg CH₄/head/yr) for cattle (developing countries).
    pub const ENTERIC_CATTLE: f64 = 49.0;
    pub const ENTERIC_SHEEP: f64 = 6.0;
    pub const ENTERIC_SWINE: f64 = 1.5;

    /// Manure management CH4 EF (kg CH₄/head/yr) for cattle (lagoon system).
    pub const MANURE_CATTLE_CH4: f64 = 16.0;

    // ── Industrial Processes ──

    /// Cement clinker emission factor (t CO₂ / t clinker).
    /// IPCC 2019: 0.525 t CO₂ / t clinker (global average).
    pub const CEMENT_CLINKER: f64 = 0.525;

    /// Lime production EF (t CO₂ / t lime).
    pub const LIME_PRODUCTION: f64 = 0.75;

    /// Steel — basic oxygen furnace (t CO₂ / t crude steel).
    pub const STEEL_BOF: f64 = 1.6;

    // ── Waste ──

    /// Municipal solid waste landfill CH4 generation potential (t CH₄ / t waste).
    /// IPCC default degradable organic carbon: 0.15 — 0.20.
    pub const LANDFILL_CH4_POTENTIAL: f64 = 0.06;

    /// Default methane oxidation factor for managed aerobic landfills.
    pub const LANDFILL_OXIDATION_FACTOR: f64 = 0.1;

    /// Wastewater CH4 emission factor (kg CH₄/kg BOD) — industrial.
    pub const WASTEWATER_CH4: f64 = 0.25;

    // ── Transport ──

    /// Default emission factor for gasoline cars (kg CO₂/km).
    pub const GASOLINE_CAR: f64 = 0.120;
    /// Default for diesel trucks (kg CO₂/km).
    pub const DIESEL_TRUCK: f64 = 0.250;
    /// Aviation kerosene (kg CO₂/km per passenger).
    pub const AVIATION_PER_PAX_KM: f64 = 0.120;
    /// Rail — diesel (kg CO₂/km per passenger).
    pub const RAIL_DIESEL_PER_PAX_KM: f64 = 0.041;

    // ── GWP Constants ──

    /// AR5 100-year GWP values.
    pub const GWP_CH4_AR5: f64 = 28.0;
    pub const GWP_N2O_AR5: f64 = 265.0;
    /// AR6 100-year GWP values (IPCC 2021).
    pub const GWP_CH4_AR6: f64 = 27.0;
    pub const GWP_N2O_AR6: f64 = 273.0;

    /// Build GasFactor for methane, converting CH₄ mass to CO₂e.
    pub fn ch4_factor(mass_ch4: f64, version: GwpVersion) -> GasFactor {
        let gwp = gwp100(GreenhouseGas::CH4, version);
        GasFactor {
            gas: GreenhouseGas::CH4,
            factor: mass_ch4 * gwp,
            unit: "tCO₂e".into(),
            gwp_version: version,
            uncertainty_pct: Some(30.0),
        }
    }

    /// Build GasFactor for N₂O.
    pub fn n2o_factor(mass_n2o: f64, version: GwpVersion) -> GasFactor {
        let gwp = gwp100(GreenhouseGas::N2O, version);
        GasFactor {
            gas: GreenhouseGas::N2O,
            factor: mass_n2o * gwp,
            unit: "tCO₂e".into(),
            gwp_version: version,
            uncertainty_pct: Some(30.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuel_co2() {
        let co2 = IpccEfDb::fuel_co2(FuelType::Diesel);
        assert!(co2 > 0.0, "Diesel should produce CO₂");
        let co2_gas = IpccEfDb::fuel_co2(FuelType::Gasoline);
        assert!(co2_gas > 0.0);
    }

    #[test]
    fn test_land_use_flux() {
        let ef = IpccEfDb::land_use_flux("forest", 10.0).unwrap();
        assert!(ef.factor_value < 0.0, "Forest should be a sink");
        assert!(ef.unit.contains("tCO₂"));
        // 10 ha × −5.0 = −50.0
        assert!((ef.factor_value - (-50.0)).abs() < 1e-6);
    }

    #[test]
    fn test_land_use_unknown() {
        assert!(IpccEfDb::land_use_flux("tundra", 100.0).is_none());
    }

    #[test]
    fn test_grid_electricity() {
        let ef = IpccEfDb::grid_electricity(1000.0, None);
        assert_eq!(ef.scope, Some(EmissionScope::Scope2));
        // 1000 kWh * 0.475 tCO₂/MWh / 1000 = 0.475 tCO₂
        assert!((ef.factor_value - 0.475).abs() < 1e-6);
    }

    #[test]
    fn test_ch4_factor() {
        let gf = IpccEfDb::ch4_factor(1.0, GwpVersion::AR5);
        assert!((gf.factor - 28.0).abs() < 1e-6);
    }

    #[test]
    fn test_stationary_combustion() {
        let ef = IpccEfDb::stationary_combustion(FuelType::Diesel, 1000.0);
        assert_eq!(ef.category, "fuel_diesel");
        assert!(ef.factor_value > 0.0);
    }
}
