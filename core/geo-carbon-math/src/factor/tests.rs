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
    assert!(
        gwp100(GreenhouseGas::N2O, GwpVersion::AR4) > gwp100(GreenhouseGas::N2O, GwpVersion::AR5)
    );
}

#[test]
fn test_gas_factor_to_tco2e() {
    let gf = GasFactor::land_use(GreenhouseGas::CH4, 100.0, "kg CH₄/ha/yr");
    // 100 kg CH₄ × 28 GWP / 1000 = 2.8 tCO₂e
    assert!((gf.to_tco2e() - 2.8).abs() < 0.001);
}

#[test]
fn test_load_from_csv_simple() {
    let csv = "category,factor_value,source
forest,5.0,IPCC_2019
grassland,-1.0,IPCC_2019
";
    let factors = load_factors_from_csv(csv).unwrap();
    assert_eq!(factors.len(), 2);
    assert_eq!(factors[0].factor_value, 5.0);
    assert_eq!(factors[1].factor_value, -1.0);
    assert!(!factors[0].has_gas_breakdown());
}

#[test]
fn test_load_from_csv_multi_gas() {
    let csv = "category,source,gas_ch4_factor,gas_n2o_factor,uncertainty_pct
rice_paddy,IPCC_2019,150.0,3.0,40.0
wetland,IPCC_2019,200.0,1.0,50.0
";
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
    let csv = "name,value
forest,5.0
";
    assert!(load_factors_from_csv(csv).is_err());
}

#[test]
fn test_gwp_version_default() {
    assert_eq!(GwpVersion::default(), GwpVersion::AR5);
}

fn assert_all_fuels_positive<F: Fn(&FuelType) -> f64>(f: F, name: &str) {
    let variants = [
        FuelType::RawCoal,
        FuelType::CleanedCoal,
        FuelType::Coke,
        FuelType::CrudeOil,
        FuelType::Gasoline,
        FuelType::Diesel,
        FuelType::FuelOil,
        FuelType::LPG,
        FuelType::NaturalGas,
        FuelType::CokeOvenGas,
        FuelType::BlastFurnaceGas,
        FuelType::Biomass,
        FuelType::OtherFuel,
    ];
    for fuel in &variants {
        let val = f(fuel);
        assert!(val > 0.0, "{name} should be positive for {:?}", fuel);
    }
}

#[test]
fn test_fuel_type_default_ncv_all_variants() {
    assert_all_fuels_positive(|f| f.default_ncv(), "NCV");
}

#[test]
fn test_fuel_type_default_carbon_content_all_variants() {
    assert_all_fuels_positive(|f| f.default_carbon_content(), "Carbon content");
}

#[test]
fn test_fuel_type_default_oxidation_rate() {
    for fuel in &[
        FuelType::RawCoal,
        FuelType::CleanedCoal,
        FuelType::Coke,
        FuelType::CrudeOil,
        FuelType::Gasoline,
        FuelType::Diesel,
        FuelType::FuelOil,
        FuelType::LPG,
        FuelType::NaturalGas,
        FuelType::CokeOvenGas,
        FuelType::BlastFurnaceGas,
        FuelType::Biomass,
        FuelType::OtherFuel,
    ] {
        let ox = fuel.default_oxidation_rate();
        assert!(
            ox > 0.0 && ox <= 1.0,
            "Ox rate {ox} out of range for {:?}",
            fuel
        );
    }
}

#[test]
fn test_fuel_type_compute_co2() {
    // RawCoal: ncv=20.908, cc=26.37, ox=0.94
    // CO2 = 1 × 20.908 × 26.37 × 0.94 / 1000 × (44/12) ≈ 1.900
    let co2 = FuelType::RawCoal.compute_co2(1.0);
    assert!((co2 - 1.90).abs() < 0.05, "Expected ~1.9 tCO2, got {co2}");

    // NaturalGas: ncv=389.31, cc=15.32, ox=0.99
    let co2_gas = FuelType::NaturalGas.compute_co2(1.0);
    assert!(co2_gas > 0.0, "Gas CO2 should be positive");
}

#[test]
fn test_fuel_type_labels() {
    for fuel in &[
        FuelType::RawCoal,
        FuelType::Diesel,
        FuelType::NaturalGas,
        FuelType::Biomass,
    ] {
        let label = fuel.label();
        assert!(!label.is_empty());
    }
}

#[test]
fn test_grid_factor_china_regions() {
    let north = GridEmissionFactor::for_china_region("cn-north", 2023);
    assert_eq!(north.factor_tco2_per_mwh, GridEmissionFactor::CN_NORTH_2023);

    let default = GridEmissionFactor::for_china_region("unknown", 2023);
    assert_eq!(default.factor_tco2_per_mwh, GridEmissionFactor::CN_2023);
}

#[test]
fn test_load_from_csv_industrial() {
    let csv = "category,factor_value,source,scope,activity_type,fuel_type,ncv,cc,ox
coal_boiler,5.0,IPCC_2006,scope1,fuel,RawCoal,20.9,26.37,0.94
";
    let factors = load_factors_from_csv(csv).unwrap();
    assert_eq!(factors.len(), 1);
    assert_eq!(factors[0].category, "coal_boiler");
    assert_eq!(factors[0].factor_value, 5.0);
    // fuel_type parsed
    assert_eq!(factors[0].fuel_type, Some(FuelType::RawCoal));
}

#[test]
fn test_load_from_csv_with_region() {
    let csv = "category,factor_value,source,region
forest,5.0,IPCC_2019,CN-51
";
    let factors = load_factors_from_csv(csv).unwrap();
    assert_eq!(factors.len(), 1);
    assert_eq!(factors[0].region, Some("CN-51".to_string()));
}

#[test]
fn test_emission_scope_label() {
    assert!(!EmissionScope::Scope1.label().is_empty());
    assert!(!EmissionScope::Scope2.label().is_empty());
    assert!(!EmissionScope::Scope3.label().is_empty());
}

#[test]
fn test_greenhouse_gas_name() {
    for gas in &[
        GreenhouseGas::CO2,
        GreenhouseGas::CH4,
        GreenhouseGas::N2O,
        GreenhouseGas::SF6,
    ] {
        assert!(!gas.name().is_empty());
        assert!(!gas.formula().is_empty());
    }
}

// ── GreenhouseGas::meta (via name/formula) ──

#[test]
fn test_greenhouse_gas_name_all_variants() {
    let cases = [
        (GreenhouseGas::CO2, "Carbon dioxide"),
        (GreenhouseGas::CH4, "Methane"),
        (GreenhouseGas::N2O, "Nitrous oxide"),
        (GreenhouseGas::HFCs, "Hydrofluorocarbons"),
        (GreenhouseGas::PFCs, "Perfluorocarbons"),
        (GreenhouseGas::SF6, "Sulfur hexafluoride"),
        (GreenhouseGas::NF3, "Nitrogen trifluoride"),
    ];
    for (gas, expected) in &cases {
        assert_eq!(gas.name(), *expected, "Wrong name for {:?}", gas);
    }
}

#[test]
fn test_greenhouse_gas_formula_all_variants() {
    let cases = [
        (GreenhouseGas::CO2, "CO₂"),
        (GreenhouseGas::CH4, "CH₄"),
        (GreenhouseGas::N2O, "N₂O"),
        (GreenhouseGas::HFCs, "HFCs"),
        (GreenhouseGas::PFCs, "PFCs"),
        (GreenhouseGas::SF6, "SF₆"),
        (GreenhouseGas::NF3, "NF₃"),
    ];
    for (gas, expected) in &cases {
        assert_eq!(gas.formula(), *expected, "Wrong formula for {:?}", gas);
    }
}

#[test]
fn test_greenhouse_gas_name_not_empty() {
    // All greenhouse gas names must be non-empty
    for gas in &[
        GreenhouseGas::CO2,
        GreenhouseGas::CH4,
        GreenhouseGas::N2O,
        GreenhouseGas::HFCs,
        GreenhouseGas::PFCs,
        GreenhouseGas::SF6,
        GreenhouseGas::NF3,
    ] {
        assert!(!gas.name().is_empty(), "{:?} has empty name", gas);
        assert!(!gas.formula().is_empty(), "{:?} has empty formula", gas);
    }
}
