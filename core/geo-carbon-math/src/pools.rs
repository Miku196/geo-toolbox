//! Five Carbon Pool Model — IPCC Guidelines for National GHG Inventories.
//!
//! Implements the five carbon pools per IPCC 2006/2019 guidelines:
//! - AGB (Above-Ground Biomass): living biomass above the soil surface
//! - BGB (Below-Ground Biomass): living root biomass
//! - Deadwood: dead standing + fallen trees
//! - Litter: dead organic matter on the soil surface
//! - SOC (Soil Organic Carbon): carbon in mineral and organic soils
//!
//! ## IPCC References
//! - 2006 GL Vol.4 Ch.2 (Forest Land)
//! - 2019 Refinement Vol.4 Ch.2
//! - 2006 GL Vol.4 Ch.4 (Cropland)
//! - 2006 GL Vol.4 Ch.5 (Grassland)
//! - 2006 GL Vol.4 Ch.6 (Wetlands)
//! - 2006 GL Vol.4 Ch.7 (Settlements)

use serde::{Deserialize, Serialize};

// ── Pool Enum ─────────────────────────────────────────────────

/// Five IPCC carbon pools plus an aggregate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CarbonPool {
    /// Above-ground biomass (trees, shrubs, herbs).
    #[serde(rename = "AGB")]
    AGB,
    /// Below-ground biomass (roots).
    #[serde(rename = "BGB")]
    BGB,
    /// Dead standing + fallen wood.
    #[serde(rename = "Deadwood")]
    Deadwood,
    /// Leaf litter + fine organic matter on the soil surface.
    #[serde(rename = "Litter")]
    Litter,
    /// Soil organic carbon (mineral + organic soils).
    #[serde(rename = "SOC")]
    SOC,
}

impl CarbonPool {
    /// Meta: returns (display label, short code).
    fn meta(&self) -> (&'static str, &'static str) {
        match self {
            CarbonPool::AGB => ("Above-Ground Biomass", "AGB"),
            CarbonPool::BGB => ("Below-Ground Biomass", "BGB"),
            CarbonPool::Deadwood => ("Deadwood", "DW"),
            CarbonPool::Litter => ("Litter", "LT"),
            CarbonPool::SOC => ("Soil Organic Carbon", "SOC"),
        }
    }

    /// IPCC pool display name.
    pub fn label(&self) -> &'static str {
        self.meta().0
    }

    /// Short IPCC pool code.
    pub fn code(&self) -> &'static str {
        self.meta().1
    }

    /// Default uncertainty (±%).
    pub fn default_uncertainty_pct(&self) -> f64 {
        match self {
            CarbonPool::AGB | CarbonPool::BGB => 30.0,
            CarbonPool::Deadwood => 90.0,
            CarbonPool::Litter => 60.0,
            CarbonPool::SOC => 50.0,
        }
    }

    /// All five pools in IPCC order.
    pub fn all() -> [CarbonPool; 5] {
        [
            CarbonPool::AGB,
            CarbonPool::BGB,
            CarbonPool::Deadwood,
            CarbonPool::Litter,
            CarbonPool::SOC,
        ]
    }
}

// ── Biomass Parameters ────────────────────────────────────────

/// IPCC ecological zone classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EcoZone {
    /// Tropical moist forest (warm, wet year-round).
    TropicalMoist,
    /// Temperate coniferous forest.
    TemperateConiferous,
    /// Temperate broadleaf forest.
    TemperateBroadleaf,
    /// Boreal / taiga forest.
    Boreal,
}

/// Biomass parameters for a forest type or ecological zone.
///
/// Defaults from IPCC 2006/2019 default tables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomassParams {
    /// Wood density (t d.m. / m³). IPCC default: 0.5 for tropical, 0.45 for temperate.
    pub wood_density: f64,
    /// Biomass Expansion Factor (dimensionless). Converts stem volume to total AGB.
    /// IPCC default: 1.5 for coniferous, 2.0 for broadleaf.
    pub bef: f64,
    /// Carbon fraction (t C / t d.m.). IPCC default: 0.47 (2006) or 0.50 (simplified).
    pub carbon_fraction: f64,
    /// Root-to-shoot ratio (R). Converts AGB to BGB.
    /// IPCC default: 0.26 for coniferous, 0.24 for broadleaf.
    pub root_shoot_ratio: f64,
    /// Deadwood ratio: deadwood biomass as fraction of AGB. IPCC default: 0.05–0.40.
    pub deadwood_ratio: f64,
    /// Litter ratio: litter as fraction of total above-ground biomass.
    pub litter_ratio: f64,
    /// Annual litterfall turnover rate (fraction/yr).
    pub litter_turnover: f64,
    /// Deadwood decomposition rate (k, fraction/yr). IPCC default: 0.05–0.10 for temperate.
    pub deadwood_decay_rate: f64,
}

impl Default for BiomassParams {
    fn default() -> Self {
        Self {
            wood_density: 0.50,
            bef: 1.75,
            carbon_fraction: 0.47,
            root_shoot_ratio: 0.25,
            deadwood_ratio: 0.15,
            litter_ratio: 0.05,
            litter_turnover: 0.5,
            deadwood_decay_rate: 0.07,
        }
    }
}

impl BiomassParams {
    /// Get IPCC default biomass parameters for a given ecological zone.
    pub fn for_eco_zone(zone: EcoZone) -> Self {
        match zone {
            EcoZone::TropicalMoist => Self {
                wood_density: 0.60,
                bef: 2.2,
                carbon_fraction: 0.47,
                root_shoot_ratio: 0.27,
                deadwood_ratio: 0.12,
                litter_ratio: 0.04,
                litter_turnover: 0.6,
                deadwood_decay_rate: 0.10,
            },
            EcoZone::TemperateConiferous => Self {
                wood_density: 0.45,
                bef: 1.5,
                carbon_fraction: 0.47,
                root_shoot_ratio: 0.26,
                deadwood_ratio: 0.18,
                litter_ratio: 0.06,
                litter_turnover: 0.4,
                deadwood_decay_rate: 0.05,
            },
            EcoZone::TemperateBroadleaf => Self {
                wood_density: 0.58,
                bef: 2.0,
                carbon_fraction: 0.47,
                root_shoot_ratio: 0.24,
                deadwood_ratio: 0.15,
                litter_ratio: 0.05,
                litter_turnover: 0.5,
                deadwood_decay_rate: 0.07,
            },
            EcoZone::Boreal => Self {
                wood_density: 0.40,
                bef: 1.3,
                carbon_fraction: 0.47,
                root_shoot_ratio: 0.29,
                deadwood_ratio: 0.25,
                litter_ratio: 0.08,
                litter_turnover: 0.3,
                deadwood_decay_rate: 0.03,
            },
        }
    }

    /// IPCC default for tropical moist forest.
    pub fn tropical_moist() -> Self {
        Self::for_eco_zone(EcoZone::TropicalMoist)
    }

    /// IPCC default for temperate coniferous.
    pub fn temperate_coniferous() -> Self {
        Self::for_eco_zone(EcoZone::TemperateConiferous)
    }

    /// IPCC default for temperate broadleaf.
    pub fn temperate_broadleaf() -> Self {
        Self::for_eco_zone(EcoZone::TemperateBroadleaf)
    }

    /// IPCC default for boreal forest.
    pub fn boreal() -> Self {
        Self::for_eco_zone(EcoZone::Boreal)
    }
}

// ── SOC Parameters ────────────────────────────────────────────

/// Land-use scenario for SOC parameter selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LandUseScenario {
    /// Native forest under default management.
    NativeForest,
    /// Degraded cropland, reduced input.
    DegradedCropland,
    /// Afforestation on former cropland.
    AfforestationCropland,
    /// Deforestation from forest to cropland.
    DeforestationCropland,
}

/// Soil organic carbon reference + land use factors.
///
/// SOC_stock = SOC_ref × FLU × FMG × FI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocParams {
    /// Reference soil carbon stock (t C / ha, 0-30 cm depth).
    pub soc_ref_tc_ha: f64,
    /// Land-use factor (FLU). Modifies SOC_ref based on land use type.
    pub flu: f64,
    /// Management factor (FMG). Modifies SOC_ref based on management regime.
    pub fmg: f64,
    /// Input factor (FI). Modifies SOC_ref based on carbon input level.
    pub fi: f64,
}

impl Default for SocParams {
    fn default() -> Self {
        Self {
            soc_ref_tc_ha: 60.0,
            flu: 1.0,
            fmg: 1.0,
            fi: 1.0,
        }
    }
}

impl SocParams {
    /// Get SOC parameters for a given land-use scenario.
    pub fn for_scenario(scenario: LandUseScenario, soc_ref: f64) -> Self {
        match scenario {
            LandUseScenario::NativeForest => Self {
                soc_ref_tc_ha: soc_ref,
                flu: 1.0,
                fmg: 1.0,
                fi: 1.0,
            },
            LandUseScenario::DegradedCropland => Self {
                soc_ref_tc_ha: soc_ref,
                flu: 0.80,
                fmg: 1.0,
                fi: 0.95,
            },
            LandUseScenario::AfforestationCropland => Self {
                soc_ref_tc_ha: soc_ref,
                flu: 1.0,
                fmg: 1.0,
                fi: 1.0,
            },
            LandUseScenario::DeforestationCropland => Self {
                soc_ref_tc_ha: soc_ref,
                flu: 0.80,
                fmg: 0.95,
                fi: 0.92,
            },
        }
    }

    /// SOC factors for native forest under default management.
    pub fn native_forest(soc_ref: f64) -> Self {
        Self::for_scenario(LandUseScenario::NativeForest, soc_ref)
    }

    /// SOC factors for degraded cropland.
    pub fn degraded_cropland(soc_ref: f64) -> Self {
        Self::for_scenario(LandUseScenario::DegradedCropland, soc_ref)
    }

    /// SOC factors for afforestation on cropland.
    pub fn afforestation_cropland(soc_ref: f64) -> Self {
        Self::for_scenario(LandUseScenario::AfforestationCropland, soc_ref)
    }

    /// SOC factors for deforestation.
    pub fn deforestation_cropland(soc_ref: f64) -> Self {
        Self::for_scenario(LandUseScenario::DeforestationCropland, soc_ref)
    }

    /// Compute SOC stock (t C / ha).
    pub fn compute_stock_tc_ha(&self) -> f64 {
        self.soc_ref_tc_ha * self.flu * self.fmg * self.fi
    }
}

// ── Scenario Matrix ────────────────────────────────────────────

/// Combined biomass + SOC parameters for a given (eco-zone, land-use) pair.
///
/// This is the single entry point for looking up IPCC Tier 1 default parameters.
/// Covers all 4×4 = 16 combinatorially-valid eco-zone × land-use scenarios.
///
/// ```rust,ignore
/// let params = scenario_matrix(EcoZone::TropicalMoist, LandUseScenario::NativeForest, 60.0);
/// let agb = compute_agb_tco2e_ha(vol, params.biomass.wood_density, params.biomass.bef, params.biomass.carbon_fraction);
/// let soc_stock = params.soc.compute_stock_tc_ha();
/// ```
#[derive(Debug, Clone)]
pub struct ScenarioParams {
    pub biomass: BiomassParams,
    pub soc: SocParams,
    pub eco_zone: EcoZone,
    pub land_use: LandUseScenario,
}

/// Lookup biomass and SOC parameters in a single dispatch.
///
/// Serves as the canonical data table for all IPCC Tier 1 eco-zone × land-use combos.
/// For new eco-zones or land-use scenarios: only this function + the two enums need updating.
pub fn scenario_matrix(
    eco_zone: EcoZone,
    land_use: LandUseScenario,
    soc_ref: f64,
) -> ScenarioParams {
    ScenarioParams {
        biomass: BiomassParams::for_eco_zone(eco_zone),
        soc: SocParams::for_scenario(land_use, soc_ref),
        eco_zone,
        land_use,
    }
}

// ── Pool Stock Result ─────────────────────────────────────────

/// Carbon stock for a single pool at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStock {
    /// Which pool.
    pub pool: CarbonPool,
    /// Carbon stock in tCO₂e per hectare.
    pub tco2e_per_ha: f64,
    /// Uncertainty (±%).
    pub uncertainty_pct: f64,
    /// Reference: where the parameters came from.
    pub source: String,
}

impl PoolStock {
    pub fn new(pool: CarbonPool, tco2e_per_ha: f64, uncertainty_pct: f64, source: &str) -> Self {
        Self {
            pool,
            tco2e_per_ha,
            uncertainty_pct,
            source: source.to_string(),
        }
    }
}

// ── Pool Change Result ────────────────────────────────────────

/// Carbon stock change for a single pool between two time periods.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolChange {
    /// Which pool.
    pub pool: CarbonPool,
    /// Before stock (tCO₂e/ha).
    pub before_tco2e_ha: f64,
    /// After stock (tCO₂e/ha).
    pub after_tco2e_ha: f64,
    /// Net change (tCO₂e/ha). Positive = emissions, negative = sequestration.
    pub delta_tco2e_ha: f64,
    /// Total change over the area (tCO₂e).
    pub delta_total_tco2e: f64,
    /// IPCC Tier level used (1-3).
    pub tier: u8,
    /// Methodology reference.
    pub methodology: String,
}

impl PoolChange {
    pub fn emission_tco2e(&self, _area_ha: f64) -> f64 {
        self.delta_total_tco2e
    }
}

// ── Pool Computation Functions ─────────────────────────────────

/// Compute AGB stock (tC/ha) from stand-level parameters.
///
/// Formula (IPCC 2006 GL Vol.4 Eq.2.8):
///   AGB = V × WD × BEF
///
/// Where:
/// - V = merchantable stem volume (m³/ha)
/// - WD = wood density (t d.m. / m³)
/// - BEF = biomass expansion factor (dimensionless)
///
/// Returns AGB in t d.m./ha (dry matter). Convert to tC by multiplying by CF.
/// Convert to tCO₂e by multiplying by 44/12.
pub fn compute_agb_biomass_tdm_ha(stem_volume_m3_ha: f64, wd: f64, bef: f64) -> f64 {
    stem_volume_m3_ha * wd * bef
}

/// Compute AGB carbon stock (tC/ha).
pub fn compute_agb_carbon_tc_ha(stem_volume_m3_ha: f64, wd: f64, bef: f64, cf: f64) -> f64 {
    compute_agb_biomass_tdm_ha(stem_volume_m3_ha, wd, bef) * cf
}

/// Compute AGB stock in tCO₂e/ha.
pub fn compute_agb_tco2e_ha(stem_volume_m3_ha: f64, wd: f64, bef: f64, cf: f64) -> f64 {
    tc_to_tco2e(compute_agb_carbon_tc_ha(stem_volume_m3_ha, wd, bef, cf))
}

/// Compute BGB stock as a fraction of AGB.
///
/// Formula: BGB = AGB × R
/// Where R = root-to-shoot ratio.
pub fn compute_bgb_tco2e_ha(agb_tco2e_ha: f64, root_shoot_ratio: f64) -> f64 {
    agb_tco2e_ha * root_shoot_ratio
}

/// Compute deadwood stock from AGB.
///
/// Simplified IPCC Tier 1: deadwood = AGB × deadwood_ratio
pub fn compute_deadwood_tco2e_ha(agb_tco2e_ha: f64, deadwood_ratio: f64) -> f64 {
    agb_tco2e_ha * deadwood_ratio
}

/// Compute deadwood stock change over time using first-order decay.
///
/// C_dw(t+1) = C_dw(t) × e^(-k) + C_input × (1 - e^(-k)) / k
///
/// Where:
/// - k = decay rate constant (fraction/yr)
/// - C_input = annual deadwood input
pub fn compute_deadwood_decay(
    previous_tco2e_ha: f64,
    annual_input_tco2e_ha: f64,
    decay_rate: f64,
    years: f64,
) -> f64 {
    let k = decay_rate;
    let decay_factor = (-k * years).exp();
    previous_tco2e_ha * decay_factor + annual_input_tco2e_ha * (1.0 - decay_factor) / k
}

/// Compute litter stock from AGB.
///
/// Simplified IPCC Tier 1: litter = AGB × litter_ratio
pub fn compute_litter_tco2e_ha(agb_tco2e_ha: f64, litter_ratio: f64) -> f64 {
    agb_tco2e_ha * litter_ratio
}

/// Compute litter stock change using turnover model.
///
/// C_litter(t+1) = C_litter(t) × e^(-turnover) + (AGB × litter_turnover) × (1 - e^(-turnover)) / turnover
pub fn compute_litter_turnover(
    previous_litter_tco2e_ha: f64,
    agb_tco2e_ha: f64,
    litter_turnover_rate: f64,
    years: f64,
) -> f64 {
    let k = litter_turnover_rate;
    let decay = (-k * years).exp();
    let input = agb_tco2e_ha * litter_turnover_rate;
    previous_litter_tco2e_ha * decay + input * (1.0 - decay) / k
}

/// Compute SOC stock (tC/ha).
///
/// Formula (IPCC 2006 GL Vol.4 Eq.2.25):
///   SOC = SOC_ref × FLU × FMG × FI
pub fn compute_soc_tc_ha(soc_ref_tc_ha: f64, flu: f64, fmg: f64, fi: f64) -> f64 {
    soc_ref_tc_ha * flu * fmg * fi
}

/// Compute SOC stock (tCO₂e/ha).
pub fn compute_soc_tco2e_ha(soc_ref_tc_ha: f64, flu: f64, fmg: f64, fi: f64) -> f64 {
    tc_to_tco2e(compute_soc_tc_ha(soc_ref_tc_ha, flu, fmg, fi))
}

/// Compute SOC change over time (first-order decay to new equilibrium).
///
/// C_soil(t) = C_target - (C_target - C_initial) × e^(-k × t)
///
/// Where:
/// - C_target = SOC stock at new equilibrium (from FLU × FMG × FI)
/// - C_initial = current SOC stock
/// - k = transition rate (default IPCC: 1/20 yr⁻¹ for forest land)
/// - t = years elapsed
pub fn compute_soc_transition(
    soc_initial_tco2e_ha: f64,
    soc_target_tco2e_ha: f64,
    transition_rate: f64,
    years: f64,
) -> f64 {
    soc_target_tco2e_ha
        - (soc_target_tco2e_ha - soc_initial_tco2e_ha) * (-transition_rate * years).exp()
}

// ── Utility ───────────────────────────────────────────────────

/// Convert tonnes carbon → tonnes CO₂-equivalent.
/// CO₂ molecular weight = 44, C atomic weight = 12, so factor = 44/12.
pub fn tc_to_tco2e(tc: f64) -> f64 {
    tc * (44.0 / 12.0)
}

/// Convert tonnes CO₂-equivalent → tonnes carbon.
pub fn tco2e_to_tc(tco2e: f64) -> f64 {
    tco2e / (44.0 / 12.0)
}

pub(crate) fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

// ── Full Multi-Pool Stock ─────────────────────────────────────

/// Complete 5-pool carbon stock estimate for a land parcel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiPoolStock {
    /// Area in hectares.
    pub area_ha: f64,
    /// Per-pool stock results.
    pub pools: Vec<PoolStock>,
    /// Total carbon stock (tCO₂e).
    pub total_tco2e: f64,
    /// Per-hectare total (tCO₂e/ha).
    pub total_tco2e_per_ha: f64,
}

impl MultiPoolStock {
    pub fn new(area_ha: f64) -> Self {
        Self {
            area_ha,
            pools: Vec::with_capacity(5),
            total_tco2e: 0.0,
            total_tco2e_per_ha: 0.0,
        }
    }

    /// Add a pool result.
    pub fn add_pool(&mut self, stock: PoolStock) {
        self.total_tco2e_per_ha += stock.tco2e_per_ha;
        self.pools.push(stock);
    }

    /// Finalize: compute area-scaled total (do this before rounding per_ha).
    pub fn finalize(&mut self) {
        self.total_tco2e = round2(self.total_tco2e_per_ha * self.area_ha);
        self.total_tco2e_per_ha = round2(self.total_tco2e_per_ha);
    }

    /// Compute all 5 pools from stand parameters.
    ///
    /// Uses simplified IPCC Tier 1 defaults for Deadwood/Litter/SOC.
    pub fn compute_all(
        area_ha: f64,
        stem_volume_m3_ha: f64,
        biomass: &BiomassParams,
        soc: &SocParams,
        source: &str,
    ) -> Self {
        let mut stock = Self::new(area_ha);

        // 1. AGB
        let agb = compute_agb_tco2e_ha(
            stem_volume_m3_ha,
            biomass.wood_density,
            biomass.bef,
            biomass.carbon_fraction,
        );
        stock.add_pool(PoolStock::new(
            CarbonPool::AGB,
            agb,
            CarbonPool::AGB.default_uncertainty_pct(),
            source,
        ));

        // 2. BGB
        let bgb = compute_bgb_tco2e_ha(agb, biomass.root_shoot_ratio);
        stock.add_pool(PoolStock::new(
            CarbonPool::BGB,
            bgb,
            CarbonPool::BGB.default_uncertainty_pct(),
            source,
        ));

        // 3. Deadwood
        let dw = compute_deadwood_tco2e_ha(agb, biomass.deadwood_ratio);
        stock.add_pool(PoolStock::new(
            CarbonPool::Deadwood,
            dw,
            CarbonPool::Deadwood.default_uncertainty_pct(),
            source,
        ));

        // 4. Litter
        let lt = compute_litter_tco2e_ha(agb, biomass.litter_ratio);
        stock.add_pool(PoolStock::new(
            CarbonPool::Litter,
            lt,
            CarbonPool::Litter.default_uncertainty_pct(),
            source,
        ));

        // 5. SOC
        let soc_val = compute_soc_tco2e_ha(soc.soc_ref_tc_ha, soc.flu, soc.fmg, soc.fi);
        stock.add_pool(PoolStock::new(
            CarbonPool::SOC,
            soc_val,
            CarbonPool::SOC.default_uncertainty_pct(),
            source,
        ));

        stock.finalize();
        stock
    }
}

// ── Multi-Pool Change ─────────────────────────────────────────

/// Complete 5-pool carbon stock change between two land-use states.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiPoolChange {
    /// Area in hectares.
    pub area_ha: f64,
    /// Per-pool stock change results.
    pub pool_changes: Vec<PoolChange>,
    /// Total net change (tCO₂e). Positive = emissions, negative = sequestration.
    pub total_delta_tco2e: f64,
    /// Methodology reference.
    pub methodology: String,
    /// IPCC Tier level.
    pub tier: u8,
}

impl MultiPoolChange {
    pub fn new(area_ha: f64, methodology: &str, tier: u8) -> Self {
        Self {
            area_ha,
            pool_changes: Vec::with_capacity(5),
            total_delta_tco2e: 0.0,
            methodology: methodology.to_string(),
            tier,
        }
    }

    pub fn add_pool_change(&mut self, change: PoolChange) {
        self.total_delta_tco2e += change.delta_total_tco2e;
        self.pool_changes.push(change);
    }

    pub fn finalize(&mut self) {
        self.total_delta_tco2e = round2(self.total_delta_tco2e);
        for c in &mut self.pool_changes {
            c.before_tco2e_ha = round2(c.before_tco2e_ha);
            c.after_tco2e_ha = round2(c.after_tco2e_ha);
            c.delta_tco2e_ha = round2(c.delta_tco2e_ha);
            c.delta_total_tco2e = round2(c.delta_total_tco2e);
        }
    }

    /// Returns true if this is a net carbon sink (sequestration).
    pub fn is_net_sink(&self) -> bool {
        self.total_delta_tco2e < 0.0
    }

    /// Emission intensity (tCO₂e/ha).
    pub fn intensity_tco2e_ha(&self) -> f64 {
        if self.area_ha > 0.0 {
            round2(self.total_delta_tco2e / self.area_ha)
        } else {
            0.0
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tc_to_tco2e() {
        assert!((tc_to_tco2e(1.0) - 3.66666666).abs() < 0.01);
        assert!((tco2e_to_tc(44.0 / 12.0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_agb() {
        // V=200 m³/ha, WD=0.5, BEF=1.75 → AGB = 175 t d.m./ha
        let agb = compute_agb_biomass_tdm_ha(200.0, 0.5, 1.75);
        assert!((agb - 175.0).abs() < 0.01);

        // With CF=0.47 → 82.25 tC/ha → 301.58 tCO₂e/ha
        let agb_tco2e = compute_agb_tco2e_ha(200.0, 0.5, 1.75, 0.47);
        let expected = 175.0 * 0.47 * 44.0 / 12.0;
        assert!((agb_tco2e - expected).abs() < 0.01);
    }

    #[test]
    fn test_compute_bgb() {
        // AGB = 300 tCO₂e/ha, R = 0.25 → BGB = 75 tCO₂e/ha
        let bgb = compute_bgb_tco2e_ha(300.0, 0.25);
        assert!((bgb - 75.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_deadwood_simple() {
        let dw = compute_deadwood_tco2e_ha(300.0, 0.15);
        assert!((dw - 45.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_litter_simple() {
        let lt = compute_litter_tco2e_ha(300.0, 0.05);
        assert!((lt - 15.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_soc() {
        // SOC_ref=60, FLU=1.0, FMG=1.0, FI=1.0 → 60 tC/ha
        let soc_tc = compute_soc_tc_ha(60.0, 1.0, 1.0, 1.0);
        assert!((soc_tc - 60.0).abs() < 0.01);

        // SOC_ref=60, FLU=0.80, FMG=0.95, FI=0.92 → 60*0.8*0.95*0.92 = 41.95 tC/ha
        let soc_degraded = compute_soc_tc_ha(60.0, 0.80, 0.95, 0.92);
        assert!((soc_degraded - 41.952).abs() < 0.01);
    }

    #[test]
    fn test_soc_transition() {
        // Initial = 100, target = 150, k=0.05, t=20 years
        // C(20) = 150 - (150-100)*e^(-1) = 150 - 50*0.368 = 131.6
        let result = compute_soc_transition(100.0, 150.0, 0.05, 20.0);
        let expected = 150.0 - 50.0 * (-1.0f64).exp();
        assert!((result - expected).abs() < 0.1);
    }

    #[test]
    fn test_multi_pool_stock_compute_all() {
        let stock = MultiPoolStock::compute_all(
            10.0,  // 10 hectares
            200.0, // 200 m³/ha stem volume
            &BiomassParams::default(),
            &SocParams::default(),
            "IPCC_2019",
        );

        assert_eq!(stock.pools.len(), 5);
        // AGB should be largest component
        let agb = stock
            .pools
            .iter()
            .find(|p| p.pool == CarbonPool::AGB)
            .unwrap();
        assert!(agb.tco2e_per_ha > 0.0);

        // BGB should be smaller than AGB
        let bgb = stock
            .pools
            .iter()
            .find(|p| p.pool == CarbonPool::BGB)
            .unwrap();
        assert!(bgb.tco2e_per_ha < agb.tco2e_per_ha);

        // Total scales by area (total_tco2e is computed before per_ha is rounded)
        assert!((stock.total_tco2e - stock.total_tco2e_per_ha * 10.0).abs() < 0.1);
    }

    #[test]
    fn test_deadwood_decay() {
        // Previous=100, input=5, k=0.07, t=10 years
        let result = compute_deadwood_decay(100.0, 5.0, 0.07, 10.0);
        assert!(result > 0.0);
        assert!(result < 100.0); // should decay below previous
    }

    #[test]
    fn test_litter_turnover() {
        let result = compute_litter_turnover(10.0, 300.0, 0.5, 5.0);
        assert!(result > 0.0);
    }

    #[test]
    fn test_biomass_params_defaults() {
        let bp = BiomassParams::default();
        assert!((bp.wood_density - 0.50).abs() < 0.01);
        assert!((bp.carbon_fraction - 0.47).abs() < 0.01);
        assert!((bp.root_shoot_ratio - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_biomass_params_presets() {
        let tropical = BiomassParams::tropical_moist();
        assert!(tropical.bef > BiomassParams::default().bef); // tropical BEF > average

        let boreal = BiomassParams::boreal();
        assert!(boreal.wood_density < BiomassParams::default().wood_density); // boreal WD < average
    }

    #[test]
    fn test_carbon_pool_all() {
        let all = CarbonPool::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&CarbonPool::AGB));
        assert!(all.contains(&CarbonPool::SOC));
    }

    #[test]
    fn test_multi_pool_change() {
        let mut change = MultiPoolChange::new(5.0, "IPCC_Tier1", 1);

        let pool_change = PoolChange {
            pool: CarbonPool::AGB,
            before_tco2e_ha: 0.0,
            after_tco2e_ha: 300.0,
            delta_tco2e_ha: -300.0, // sequestration
            delta_total_tco2e: -1500.0,
            tier: 1,
            methodology: "IPCC A/R".to_string(),
        };
        change.add_pool_change(pool_change);
        change.finalize();

        assert!(change.is_net_sink());
        assert!((change.intensity_tco2e_ha() - -300.0).abs() < 0.01);
    }
}
