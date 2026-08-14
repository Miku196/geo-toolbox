use super::emission_factor::EmissionFactor;
use super::fuel::FuelType;
use super::gas::{EmissionScope, GasFactor, GreenhouseGas};

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
        (GreenhouseGas::CO2, idx("gas_co2_factor")),
        (GreenhouseGas::CH4, idx("gas_ch4_factor")),
        (GreenhouseGas::N2O, idx("gas_n2o_factor")),
        (GreenhouseGas::HFCs, idx("gas_hfcs_factor")),
        (GreenhouseGas::PFCs, idx("gas_pfcs_factor")),
        (GreenhouseGas::SF6, idx("gas_sf6_factor")),
        (GreenhouseGas::NF3, idx("gas_nf3_factor")),
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
    let get = |idx: Option<usize>| -> Option<&str> { idx.and_then(|i| record.get(i)) };
    let get_f64 = |idx: Option<usize>| -> Option<Result<f64, String>> {
        let s = record.get(idx?)?;
        Some(s.parse().map_err(|e| format!("Bad float '{s}': {e}")))
    };

    let category = record
        .get(cols.category)
        .ok_or("Missing category")?
        .to_string();
    let has_multi_gas = cols.gas_columns.iter().any(|(_, idx)| idx.is_some());

    let factor_value: f64 = if let Some(vi) = cols.value {
        record
            .get(vi)
            .ok_or("Missing factor_value")?
            .parse()
            .map_err(|e| format!("Bad factor_value: {e}"))?
    } else if has_multi_gas {
        0.0 // computed from gas columns below
    } else {
        return Err("CSV must have 'factor_value' or gas columns".into());
    };

    let source = get(cols.source).unwrap_or("IPCC_2019").to_string();
    let subcategory = get(cols.subcategory).map(|s| s.to_string());
    let unit = get(cols.unit).unwrap_or("tCO₂e/ha/yr").to_string();
    let valid_from_year: i32 = get(cols.valid_from_year)
        .unwrap_or("2000")
        .parse()
        .unwrap_or(2000);
    let valid_to_year: Option<i32> = get(cols.valid_to_year).and_then(|s| s.parse().ok());
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
    let gas_factors: Vec<GasFactor> = cols
        .gas_columns
        .iter()
        .filter_map(|(gas, col_idx)| {
            let ci = (*col_idx)?;
            let val: f64 = get_f64(Some(ci))?.ok()?;
            if val == 0.0 {
                return None;
            }
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
        if cols.value.is_none() {
            computed
        } else {
            factor_value
        }
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
/// \`\`\`csv
/// category,factor_value,source
/// forest,-5.0,IPCC_2019
/// \`\`\`
///
/// ### Multi-gas mode:
/// \`\`\`csv
/// category,source,gas_ch4_factor,gas_n2o_factor,uncertainty_pct
/// rice_paddy,IPCC_2019,0.0,150.0,3.0,40.0
/// \`\`\`
///
/// ### Industrial mode:
/// \`\`\`csv
/// category,source,scope,activity_type,fuel_type,grid_ef,ncv,cc,ox,uncertainty_pct
/// coal_boiler,IPCC_2006,scope1,fuel,RawCoal,,20.9,26.37,0.94,15.0
/// grid_power,MEE_2023,scope2,electricity,,0.5703,,,,10.0
/// \`\`\`
/// Supports columns: `gas_CO2_factor`, `gas_CH4_factor`, `gas_N2O_factor`,
/// `gas_HFCs_factor`, `gas_PFCs_factor`, `gas_SF6_factor`, `gas_NF3_factor`.
/// Industrial columns: `scope`, `activity_type`, `fuel_type`, `grid_ef`,
/// `ncv`, `cc`, `ox`.
pub fn load_factors_from_csv(csv_text: &str) -> Result<Vec<EmissionFactor>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(csv_text.as_bytes());

    let headers: Vec<String> = reader
        .headers()
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
    fn test_parse_emission_factor_row_simple() {
        // Build a minimal column index and record
        let headers: Vec<String> = vec!["category", "factor_value", "source"]
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        let cols = parse_csv_columns(&headers).unwrap();
        let rec = csv::StringRecord::from(vec!["forest", "-5.0", "IPCC_2019"]);
        let ef = parse_emission_factor_row(&rec, &cols).unwrap();
        assert_eq!(ef.category, "forest");
        assert_eq!(ef.factor_value, -5.0);
        assert_eq!(ef.source, "IPCC_2019");
        assert!(ef.is_sink());
    }

    #[test]
    fn test_parse_emission_factor_row_with_gases() {
        let headers: Vec<String> = vec![
            "category",
            "factor_value",
            "source",
            "gas_co2_factor",
            "gas_ch4_factor",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect();
        let cols = parse_csv_columns(&headers).unwrap();
        let rec = csv::StringRecord::from(vec!["wetland", "0.0", "IPCC_2006", "10.0", "2.5"]);
        let ef = parse_emission_factor_row(&rec, &cols).unwrap();
        assert_eq!(ef.category, "wetland");
        assert!(!ef.gas_factors.is_empty());
    }

    #[test]
    fn test_parse_emission_factor_row_with_scope() {
        let headers: Vec<String> = vec!["category", "factor_value", "source", "scope"]
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        let cols = parse_csv_columns(&headers).unwrap();
        let rec = csv::StringRecord::from(vec!["cropland", "3.0", "IPCC", "scope1"]);
        let ef = parse_emission_factor_row(&rec, &cols).unwrap();
        assert_eq!(ef.scope, Some(EmissionScope::Scope1));
    }
}
