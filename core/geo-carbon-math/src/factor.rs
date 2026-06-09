//! Emission factor types — pure data, no DB.

use serde::{Serialize, Deserialize};

/// A single emission factor entry for a landcover class.
///
/// Factor values represent tCO₂e per hectare per year.
/// Negative values indicate carbon sinks (sequestration).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmissionFactor {
    /// Landcover category (e.g., "forest", "grassland", "cropland").
    pub category: String,
    /// Optional subcategory for finer matching (e.g., "evergreen_broadleaf").
    /// When present, feature's `properties.subcategory` is also checked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subcategory: Option<String>,
    /// Emission factor value in tCO₂e/ha/yr.
    /// Positive = emission source, Negative = carbon sink.
    pub factor_value: f64,
    /// Source of the factor (e.g., "IPCC_2019", "custom_climate_action").
    #[serde(default)]
    pub source: String,
    /// Unit of measurement.
    #[serde(default = "default_unit")]
    pub unit: String,
    /// Valid from year (inclusive).
    #[serde(default)]
    pub valid_from_year: i32,
    /// Valid to year (inclusive, None = no expiry).
    #[serde(default)]
    pub valid_to_year: Option<i32>,
    /// Geographic region code (e.g., "CN-51", None = global).
    #[serde(default)]
    pub region: Option<String>,
}

fn default_unit() -> String { "tCO₂e/ha/yr".into() }

impl EmissionFactor {
    /// Create a new emission factor with minimal required fields.
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
        }
    }

    /// Returns true if this factor is valid for the given year.
    pub fn is_valid_for_year(&self, year: i32) -> bool {
        year >= self.valid_from_year
            && self.valid_to_year.map_or(true, |to| year <= to)
    }

    /// Returns true if this is a carbon sink (negative emission factor).
    pub fn is_sink(&self) -> bool {
        self.factor_value < 0.0
    }
}

/// Load emission factors from CSV text (header-based, order-independent).
///
/// Required columns: `category`, `factor_value`.
/// Optional: `source`, `subcategory`, `unit`, `valid_from_year`, `valid_to_year`, `region`, `citation`.
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

    // Find column indices by header name
    let idx = |name: &str| headers.iter().position(|h| h == name);
    
    let cat_idx = idx("category").ok_or("CSV must have 'category' column")?;
    let val_idx = idx("factor_value").ok_or("CSV must have 'factor_value' column")?;
    let src_idx = idx("source");
    let sub_idx = idx("subcategory");
    let unit_idx = idx("unit");
    let vfy_idx = idx("valid_from_year");
    let vty_idx = idx("valid_to_year");
    let reg_idx = idx("region");

    let mut factors = Vec::new();

    for result in reader.records() {
        let record = result.map_err(|e| format!("CSV row: {e}"))?;

        let category = record.get(cat_idx).ok_or("Missing category")?.to_string();
        let factor_value: f64 = record.get(val_idx)
            .ok_or("Missing factor_value")?
            .parse()
            .map_err(|e| format!("Bad factor_value '{:?}': {e}", record.get(val_idx)))?;

        let source = src_idx.and_then(|i| record.get(i)).unwrap_or("IPCC_2019").to_string();
        let subcategory = sub_idx.and_then(|i| record.get(i)).map(|s| s.to_string());
        let unit = unit_idx.and_then(|i| record.get(i)).unwrap_or("tCO₂e/ha/yr").to_string();
        let valid_from_year: i32 = vfy_idx
            .and_then(|i| record.get(i))
            .unwrap_or("2000")
            .parse()
            .unwrap_or(2000);
        let valid_to_year: Option<i32> = vty_idx
            .and_then(|i| record.get(i))
            .and_then(|s| s.parse().ok());
        let region = reg_idx.and_then(|i| record.get(i)).map(|s| s.to_string());

        factors.push(EmissionFactor {
            category,
            factor_value,
            source,
            subcategory,
            unit,
            valid_from_year,
            valid_to_year,
            region,
        });
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
    fn test_load_from_csv() {
        let csv = "category,factor_value,source\nforest,5.0,IPCC_2019\ngrassland,-1.0,IPCC_2019\n";
        let factors = load_factors_from_csv(csv).unwrap();
        assert_eq!(factors.len(), 2);
        assert_eq!(factors[0].factor_value, 5.0);
        assert_eq!(factors[1].factor_value, -1.0);
    }

    #[test]
    fn test_load_from_csv_missing_columns() {
        let csv = "name,value\nforest,5.0\n";
        assert!(load_factors_from_csv(csv).is_err());
    }
}
