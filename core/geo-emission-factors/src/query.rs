use crate::china::ChinaEfDb;
use crate::ipcc::IpccEfDb;
use geo_carbon_math::{EmissionFactor, FuelType, GwpVersion};

/// Unified emission factor database with tiered lookup.
///
/// Tier 1: Global IPCC default values
/// Tier 2: Country-specific (China provincial) values
/// Tier 3: Custom user-provided factors
#[derive(Debug, Clone)]
pub struct EfDatabase {
    /// User-defined overrides (Tier 3), keyed by category+subcategory.
    overrides: Vec<EmissionFactor>,
    /// GWP version for multi-gas calculations.
    gwp_version: GwpVersion,
    /// Default year for validity checks.
    year: u16,
    /// Default region/province for China-specific lookups.
    region: Option<String>,
}

impl Default for EfDatabase {
    fn default() -> Self {
        Self {
            overrides: Vec::new(),
            gwp_version: GwpVersion::AR5,
            year: 2025,
            region: None,
        }
    }
}

impl EfDatabase {
    /// Create a new database with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the GWP version for all calculations.
    pub fn with_gwp(mut self, version: GwpVersion) -> Self {
        self.gwp_version = version;
        self
    }

    /// Set the assessment year.
    pub fn with_year(mut self, year: u16) -> Self {
        self.year = year;
        self
    }

    /// Set the default region/province (e.g. "广东", "sichuan").
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Add a custom Tier 3 emission factor override.
    pub fn add_override(&mut self, factor: EmissionFactor) {
        self.overrides.push(factor);
    }

    /// Get GWP version.
    pub fn gwp_version(&self) -> GwpVersion {
        self.gwp_version
    }

    /// Lookup grid electricity emission factor.
    ///
    /// Uses China provincial factor if region is set, otherwise IPCC global default.
    pub fn grid_electricity(&self, kwh: f64) -> EmissionFactor {
        if let Some(ref region) = self.region {
            ChinaEfDb::china_grid_electricity(kwh, region, self.year)
        } else {
            IpccEfDb::grid_electricity(kwh, None)
        }
    }

    /// Lookup land-use carbon flux by class.
    ///
    /// Uses China provincial forest sink if region is set and class is forest.
    pub fn land_use_flux(&self, class: &str, area_ha: f64) -> Option<EmissionFactor> {
        // Check overrides first
        let cat = format!("land_use_{}", class.to_lowercase());
        for ef in &self.overrides {
            if ef.category == cat {
                let mut cloned = ef.clone();
                cloned.factor_value *= area_ha;
                return Some(cloned);
            }
        }

        // China-specific forest sink
        if class.eq_ignore_ascii_case("forest") {
            if let Some(ref region) = self.region {
                return Some(ChinaEfDb::china_forest_sink(area_ha, region, self.year));
            }
        }

        IpccEfDb::land_use_flux(class, area_ha)
    }

    /// Get fuel combustion emission factor.
    pub fn stationary_combustion(&self, fuel: FuelType, quantity: f64) -> EmissionFactor {
        let cat = format!("fuel_{:?}", fuel).to_lowercase();
        for ef in &self.overrides {
            if ef.category == cat {
                let mut cloned = ef.clone();
                cloned.factor_value *= quantity;
                return cloned;
            }
        }
        IpccEfDb::stationary_combustion(fuel, quantity)
    }

    /// Build an `EmissionFactor` from the simple CarbonParams style (class → tCO₂e/ha/yr).
    ///
    /// This bridges the old `CarbonParams::get_factor` API to the new emission factor database.
    pub fn simple_land_use(&self, class: &str, area_ha: f64) -> Option<EmissionFactor> {
        self.land_use_flux(class, area_ha)
    }

    /// Resolve category to EmissionFactor using the tiered database.
    pub fn resolve(&self, category: &str, quantity: f64) -> Option<EmissionFactor> {
        // Check overrides
        for ef in &self.overrides {
            if ef.category == category {
                let mut cloned = ef.clone();
                cloned.factor_value *= quantity;
                return Some(cloned);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_db() {
        let db = EfDatabase::new();
        let ef = db.grid_electricity(1000.0);
        assert_eq!(ef.category, "electricity");
        // No region → IPCC global
        assert!((ef.factor_value - 0.475).abs() < 1e-6);
    }

    #[test]
    fn test_with_region() {
        let db = EfDatabase::new().with_region("广东").with_year(2023);
        let ef = db.grid_electricity(1000.0);
        // 广东 2023 grid: 0.3907 tCO₂/MWh → 1000 kWh = 0.3907 tCO₂
        assert!((ef.factor_value - 0.3907).abs() < 1e-4);
    }

    #[test]
    fn test_land_use_with_region() {
        let db = EfDatabase::new().with_region("福建").with_year(2023);
        let ef = db.land_use_flux("forest", 10.0).unwrap();
        // 福建 forest sink: −8.0 tCO₂e/ha/yr × 10 ha = −80.0
        assert!((ef.factor_value - (-80.0)).abs() < 1e-6);
        assert_eq!(ef.source, "NFI_2019");
    }

    #[test]
    fn test_land_use_global() {
        let db = EfDatabase::new();
        let ef = db.land_use_flux("forest", 10.0).unwrap();
        assert!((ef.factor_value - (-50.0)).abs() < 1e-6);
        assert_eq!(ef.source, "IPCC_2019");
    }

    #[test]
    fn test_override() {
        let mut db = EfDatabase::new();
        let custom = EmissionFactor {
            category: "land_use_forest".into(),
            subcategory: Some("custom".into()),
            source: "local_study".into(),
            region: Some("test".into()),
            factor_value: -12.0,
            unit: "tCO₂e/ha/yr".into(),
            valid_from_year: 2020,
            valid_to_year: None,
            gas_factors: vec![],
            uncertainty_pct: None,
            scope: None,
            activity_type: None,
            fuel_type: None,
            ncv_override: None,
            cc_override: None,
            ox_override: None,
            grid_ef: None,
        };
        db.add_override(custom);
        let ef = db.land_use_flux("forest", 10.0).unwrap();
        assert!((ef.factor_value - (-120.0)).abs() < 1e-6);
    }

    #[test]
    fn test_unknown_class() {
        let db = EfDatabase::new();
        assert!(db.land_use_flux("tundra", 100.0).is_none());
    }
}
