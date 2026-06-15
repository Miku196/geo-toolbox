//! Carbon Scenarios — Afforestation, IFM, Deforestation.
//!
//! Each scenario models land-use change with the 5-pool carbon model,
//! computing the difference between baseline (before) and project (after)
//! carbon stocks across AGB, BGB, Deadwood, Litter, and SOC pools.
//!
//! ## IPCC References
//! - 2006 GL Vol.4 Ch.2.3 (Forest Land Remaining Forest Land — IFM)
//! - 2006 GL Vol.4 Ch.2.4 (Land Converted to Forest Land — Afforestation)
//! - 2006 GL Vol.4 Ch.2.5 (Forest Land Converted to Other — Deforestation)

use serde::{Deserialize, Serialize};

use crate::pools::{
    compute_deadwood_decay, compute_litter_turnover, compute_soc_tco2e_ha,
    compute_soc_transition,
    BiomassParams, CarbonPool, MultiPoolChange,
    MultiPoolStock, PoolChange, PoolStock, SocParams,
};

// ── Scenario Enum ─────────────────────────────────────────────

/// Carbon accounting scenario types per VCS/CCB methodology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CarbonScenario {
    /// Afforestation / Reforestation: non-forest → forest.
    /// Land converted to forest land.
    #[serde(rename = "afforestation")]
    Afforestation,
    /// Improved Forest Management: forest → managed forest.
    /// Forest land remaining forest land with improved practices.
    #[serde(rename = "ifm")]
    IFM,
    /// Deforestation / Forest Degradation: forest → non-forest.
    /// Forest land converted to other land uses.
    #[serde(rename = "deforestation")]
    Deforestation,
}

impl CarbonScenario {
    pub fn label(&self) -> &'static str {
        match self {
            CarbonScenario::Afforestation => "Afforestation/Reforestation",
            CarbonScenario::IFM => "Improved Forest Management",
            CarbonScenario::Deforestation => "Deforestation",
        }
    }

    /// Returns whether the scenario typically results in sequestration.
    pub fn is_sequestration_scenario(&self) -> bool {
        matches!(self, CarbonScenario::Afforestation | CarbonScenario::IFM)
    }

    /// Returns whether the scenario typically results in emissions.
    pub fn is_emission_scenario(&self) -> bool {
        matches!(self, CarbonScenario::Deforestation)
    }
}

// ── Land-State Descriptor ─────────────────────────────────────

/// Describes a land parcel before or after a change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandState {
    /// Land cover classification (e.g., "forest", "grassland", "cropland").
    pub landcover_class: String,
    /// Merchantable stem volume (m³/ha). 0 for non-forest.
    pub stem_volume_m3_ha: f64,
    /// Ecological zone for biomass parameters.
    #[serde(default)]
    pub ecozone: EcoZone,
    /// Biomass parameters (if provided; defaults from ecozone).
    #[serde(default)]
    pub biomass_params: Option<BiomassParams>,
    /// SOC parameters (if provided; defaults from ecozone).
    #[serde(default)]
    pub soc_params: Option<SocParams>,
    /// Time since transition began (years). Used for SOC/deadwood/litter dynamics.
    #[serde(default)]
    pub years_since_transition: f64,
}

impl LandState {
    /// Non-forest land (e.g., grassland, cropland, bare land).
    pub fn non_forest(class: &str) -> Self {
        Self {
            landcover_class: class.to_string(),
            stem_volume_m3_ha: 0.0,
            ecozone: EcoZone::default(),
            biomass_params: None,
            soc_params: Some(SocParams::degraded_cropland(60.0)),
            years_since_transition: 0.0,
        }
    }

    /// Forest land with given stem volume.
    pub fn forest(class: &str, stem_volume_m3_ha: f64, ecozone: EcoZone) -> Self {
        Self {
            landcover_class: class.to_string(),
            stem_volume_m3_ha,
            ecozone,
            biomass_params: None,
            soc_params: Some(SocParams::native_forest(ecozone.soc_ref_tc_ha())),
            years_since_transition: 0.0,
        }
    }
}

// ── EcoZone ───────────────────────────────────────────────────

/// Simplified ecological zone classification for IPCC defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EcoZone {
    /// Tropical rainforest / moist deciduous.
    #[serde(rename = "tropical_moist")]
    #[default]
    TropicalMoist,
    /// Tropical dry forest.
    #[serde(rename = "tropical_dry")]
    TropicalDry,
    /// Temperate coniferous.
    #[serde(rename = "temperate_coniferous")]
    TemperateConiferous,
    /// Temperate broadleaf / mixed.
    #[serde(rename = "temperate_broadleaf")]
    TemperateBroadleaf,
    /// Boreal coniferous / mixed.
    #[serde(rename = "boreal")]
    Boreal,
    /// Subtropical humid.
    #[serde(rename = "subtropical_humid")]
    SubtropicalHumid,
}

impl EcoZone {
    /// IPCC label for this zone.
    pub fn label(&self) -> &'static str {
        match self {
            EcoZone::TropicalMoist => "Tropical Moist Forest",
            EcoZone::TropicalDry => "Tropical Dry Forest",
            EcoZone::TemperateConiferous => "Temperate Coniferous Forest",
            EcoZone::TemperateBroadleaf => "Temperate Broadleaf Forest",
            EcoZone::Boreal => "Boreal Forest",
            EcoZone::SubtropicalHumid => "Subtropical Humid Forest",
        }
    }

    /// Default biomass parameters for this ecozone.
    pub fn biomass_params(&self) -> BiomassParams {
        match self {
            EcoZone::TropicalMoist | EcoZone::SubtropicalHumid => BiomassParams::tropical_moist(),
            EcoZone::TropicalDry => BiomassParams {
                wood_density: 0.65,
                bef: 2.0,
                carbon_fraction: 0.47,
                root_shoot_ratio: 0.28,
                deadwood_ratio: 0.10,
                litter_ratio: 0.03,
                litter_turnover: 0.55,
                deadwood_decay_rate: 0.09,
            },
            EcoZone::TemperateConiferous => BiomassParams::temperate_coniferous(),
            EcoZone::TemperateBroadleaf => BiomassParams::temperate_broadleaf(),
            EcoZone::Boreal => BiomassParams::boreal(),
        }
    }

    /// Default SOC reference stock (tC/ha, 0-30cm).
    pub fn soc_ref_tc_ha(&self) -> f64 {
        match self {
            EcoZone::TropicalMoist | EcoZone::SubtropicalHumid => 60.0,
            EcoZone::TropicalDry => 40.0,
            EcoZone::TemperateConiferous | EcoZone::TemperateBroadleaf => 70.0,
            EcoZone::Boreal => 80.0,
        }
    }

    /// SOC transition time constant (years). IPCC default: 20 for mineral soils.
    pub fn soc_transition_years(&self) -> f64 {
        20.0
    }
}

// ── Scenario Input ────────────────────────────────────────────

/// Input parameters for a carbon scenario calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioInput {
    /// Which scenario to compute.
    pub scenario: CarbonScenario,
    /// Area in hectares.
    pub area_ha: f64,
    /// Land state before the change (baseline).
    pub before: LandState,
    /// Land state after the change (project).
    pub after: LandState,
    /// Time horizon for the project (years). Default: 30.
    #[serde(default = "default_time_horizon")]
    pub time_horizon_years: f64,
    /// Methodology reference string.
    #[serde(default)]
    pub methodology: String,
}

fn default_time_horizon() -> f64 {
    30.0
}

impl ScenarioInput {
    pub fn new(
        scenario: CarbonScenario,
        area_ha: f64,
        before: LandState,
        after: LandState,
    ) -> Self {
        Self {
            scenario,
            area_ha,
            before,
            after,
            time_horizon_years: 30.0,
            methodology: String::new(),
        }
    }

    /// Set methodology reference.
    pub fn with_methodology(mut self, m: &str) -> Self {
        self.methodology = m.to_string();
        self
    }
}

// ── Scenario Result ───────────────────────────────────────────

/// Output of a scenario calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    /// Scenario type.
    pub scenario: CarbonScenario,
    /// Area in hectares.
    pub area_ha: f64,
    /// Time horizon (years).
    pub time_horizon_years: f64,
    /// Multi-pool stock change.
    pub pool_change: MultiPoolChange,
    /// Stock before the change (tCO₂e).
    pub stock_before_tco2e: f64,
    /// Stock after the change (tCO₂e).
    pub stock_after_tco2e: f64,
    /// Annualized net change (tCO₂e/yr).
    pub annual_delta_tco2e: f64,
    /// Per-hectare annual change (tCO₂e/ha/yr).
    pub annual_delta_tco2e_per_ha: f64,
    /// Baseline scenario label.
    pub baseline_label: String,
    /// Project scenario label.
    pub project_label: String,
    /// Methodology reference.
    pub methodology: String,
}

impl ScenarioResult {
    pub fn is_net_sink(&self) -> bool {
        self.pool_change.is_net_sink()
    }

    /// Mean annual sequestration (tCO₂e/yr). Only meaningful for sink scenarios.
    pub fn annual_sequestration(&self) -> f64 {
        if self.annual_delta_tco2e < 0.0 {
            -self.annual_delta_tco2e
        } else {
            0.0
        }
    }

    /// Mean annual emissions (tCO₂e/yr). Only meaningful for emission scenarios.
    pub fn annual_emissions(&self) -> f64 {
        if self.annual_delta_tco2e > 0.0 {
            self.annual_delta_tco2e
        } else {
            0.0
        }
    }
}

// ── Scenario Computation ──────────────────────────────────────

/// Compute carbon stock change for a scenario.
///
/// Applies the 5-pool model to compute stock before and after the land-use change,
/// then calculates the net delta.
pub fn compute_scenario(input: &ScenarioInput) -> ScenarioResult {
    // Own temporary values so we can take references to them
    let before_biomass = input.before.ecozone.biomass_params();
    let before_soc = SocParams::native_forest(input.before.ecozone.soc_ref_tc_ha());
    let after_biomass = input.after.ecozone.biomass_params();
    let after_soc = SocParams::native_forest(input.after.ecozone.soc_ref_tc_ha());

    let biomass_before = input.before.biomass_params.as_ref().unwrap_or(&before_biomass);
    let soc_before = input.before.soc_params.as_ref().unwrap_or(&before_soc);
    let biomass_after = input.after.biomass_params.as_ref().unwrap_or(&after_biomass);
    let soc_after = input.after.soc_params.as_ref().unwrap_or(&after_soc);

    // Compute before stock
    let stock_before = compute_stock(&input.before, biomass_before, soc_before);
    // Compute after stock
    let stock_after = compute_stock(&input.after, biomass_after, soc_after);

    let methodology = if input.methodology.is_empty() {
        match input.scenario {
            CarbonScenario::Afforestation => "IPCC Tier 1 — A/R (2006 GL Vol.4 Ch.2.4)".to_string(),
            CarbonScenario::IFM => "IPCC Tier 1 — IFM (2006 GL Vol.4 Ch.2.3)".to_string(),
            CarbonScenario::Deforestation => "IPCC Tier 1 — Deforestation (2006 GL Vol.4 Ch.2.5)".to_string(),
        }
    } else {
        input.methodology.clone()
    };

    let mut change = MultiPoolChange::new(input.area_ha, &methodology, 1);

    for pool in CarbonPool::all() {
        let before_stock = find_pool_value(&stock_before, pool);
        let after_stock = find_pool_value(&stock_after, pool);

        // For SOC: apply transition dynamics over the time horizon
        let (effective_before, effective_after) = if pool == CarbonPool::SOC && input.time_horizon_years > 0.0 {
            // SOC transition to new equilibrium
            let target = after_stock;
            let k = 1.0 / input.after.ecozone.soc_transition_years();
            let soc_after_transitioned = compute_soc_transition(
                before_stock,
                target,
                k,
                input.time_horizon_years,
            );
            (before_stock, soc_after_transitioned)
        } else if pool == CarbonPool::Deadwood && input.time_horizon_years > 0.0 {
            // When AGB changes, deadwood input changes too
            let _agb_before = find_pool_value(&stock_before, CarbonPool::AGB);
            let agb_after = find_pool_value(&stock_after, CarbonPool::AGB);
            let dw_input_after = agb_after * biomass_after.deadwood_ratio * biomass_after.deadwood_decay_rate;

            let dw_after_decayed = compute_deadwood_decay(
                before_stock,
                dw_input_after,
                biomass_after.deadwood_decay_rate,
                input.time_horizon_years,
            );
            (before_stock, dw_after_decayed)
        }
        else if pool == CarbonPool::Litter && input.time_horizon_years > 0.0 {
            let agb_after = find_pool_value(&stock_after, CarbonPool::AGB);
            let lt_after_turnover = compute_litter_turnover(
                before_stock,
                agb_after,
                biomass_after.litter_turnover,
                input.time_horizon_years,
            );
            (before_stock, lt_after_turnover)
        }
        else {
            (before_stock, after_stock)
        };

        let delta_per_ha = effective_before - effective_after;
        let delta_total = delta_per_ha * input.area_ha;

        change.add_pool_change(PoolChange {
            pool,
            before_tco2e_ha: effective_before,
            after_tco2e_ha: effective_after,
            delta_tco2e_ha: delta_per_ha,
            delta_total_tco2e: delta_total,
            tier: 1,
            methodology: methodology.clone(),
        });
    }

    change.finalize();

    let stock_before_total = stock_before.pools.iter().map(|p| p.tco2e_per_ha).sum::<f64>() * input.area_ha;
    let stock_after_total = stock_after.pools.iter().map(|p| p.tco2e_per_ha).sum::<f64>() * input.area_ha;

    let annual = if input.time_horizon_years > 0.0 {
        change.total_delta_tco2e / input.time_horizon_years
    } else {
        change.total_delta_tco2e
    };

    ScenarioResult {
        scenario: input.scenario,
        area_ha: input.area_ha,
        time_horizon_years: input.time_horizon_years,
        pool_change: change,
        stock_before_tco2e: crate::pools::round2(stock_before_total),
        stock_after_tco2e: crate::pools::round2(stock_after_total),
        annual_delta_tco2e: crate::pools::round2(annual),
        annual_delta_tco2e_per_ha: crate::pools::round2(
            if input.area_ha > 0.0 { annual / input.area_ha } else { 0.0 }
        ),
        baseline_label: input.before.landcover_class.clone(),
        project_label: input.after.landcover_class.clone(),
        methodology,
    }
}

/// Compute multi-pool stock for a land state.
fn compute_stock(
    state: &LandState,
    biomass: &BiomassParams,
    soc: &SocParams,
) -> MultiPoolStock {
    let source = match state.ecozone {
        EcoZone::TropicalMoist => "IPCC_Tropical",
        EcoZone::TropicalDry => "IPCC_Tropical_Dry",
        EcoZone::TemperateConiferous => "IPCC_Temperate_Conif",
        EcoZone::TemperateBroadleaf => "IPCC_Temperate_Broad",
        EcoZone::Boreal => "IPCC_Boreal",
        EcoZone::SubtropicalHumid => "IPCC_Subtropical",
    };

    if state.stem_volume_m3_ha <= 0.0 {
        // Non-forest: AGB=0, BGB=0, deadwood=residual, litter=minimal
        let mut stock = MultiPoolStock::new(1.0); // per-hectare
        stock.add_pool(PoolStock::new(CarbonPool::AGB, 0.0, CarbonPool::AGB.default_uncertainty_pct(), source));
        stock.add_pool(PoolStock::new(CarbonPool::BGB, 0.0, CarbonPool::BGB.default_uncertainty_pct(), source));
        stock.add_pool(PoolStock::new(CarbonPool::Deadwood, 0.0, CarbonPool::Deadwood.default_uncertainty_pct(), source));
        stock.add_pool(PoolStock::new(CarbonPool::Litter, 0.0, CarbonPool::Litter.default_uncertainty_pct(), source));
        let soc_val = compute_soc_tco2e_ha(soc.soc_ref_tc_ha, soc.flu, soc.fmg, soc.fi);
        stock.add_pool(PoolStock::new(CarbonPool::SOC, soc_val, CarbonPool::SOC.default_uncertainty_pct(), source));
        stock.finalize();
        stock
    } else {
        MultiPoolStock::compute_all(1.0, state.stem_volume_m3_ha, biomass, soc, source)
    }
}

fn find_pool_value(stock: &MultiPoolStock, pool: CarbonPool) -> f64 {
    stock.pools.iter().find(|p| p.pool == pool).map(|p| p.tco2e_per_ha).unwrap_or(0.0)
}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: create a grassland-to-forest afforestation input
    fn afforestation_input() -> ScenarioInput {
        ScenarioInput::new(
            CarbonScenario::Afforestation,
            100.0, // 100 hectares
            LandState::non_forest("grassland"),
            LandState {
                landcover_class: "forest".to_string(),
                stem_volume_m3_ha: 150.0,
                ecozone: EcoZone::TemperateBroadleaf,
                biomass_params: None,
                soc_params: Some(SocParams::afforestation_cropland(70.0)),
                years_since_transition: 0.0,
            },
        )
    }

    fn deforestation_input() -> ScenarioInput {
        ScenarioInput::new(
            CarbonScenario::Deforestation,
            50.0,
            LandState::forest("forest", 200.0, EcoZone::TropicalMoist),
            LandState::non_forest("cropland"),
        )
    }

    fn ifm_input() -> ScenarioInput {
        ScenarioInput::new(
            CarbonScenario::IFM,
            200.0,
            LandState::forest("forest", 150.0, EcoZone::TemperateConiferous),
            LandState {
                landcover_class: "managed_forest".to_string(),
                stem_volume_m3_ha: 250.0, // improved: higher stocking
                ecozone: EcoZone::TemperateConiferous,
                biomass_params: None,
                soc_params: Some(SocParams::native_forest(70.0)),
                years_since_transition: 0.0,
            },
        )
    }

    #[test]
    fn test_afforestation_is_sink() {
        let result = compute_scenario(&afforestation_input());
        assert!(result.is_net_sink(), "Afforestation should be net sink");
        assert!(result.annual_sequestration() > 0.0);
        assert_eq!(result.annual_emissions(), 0.0);
        assert_eq!(result.pool_change.pool_changes.len(), 5);
    }

    #[test]
    fn test_deforestation_is_emitter() {
        let result = compute_scenario(&deforestation_input());
        assert!(!result.is_net_sink(), "Deforestation should be net emitter");
        assert!(result.annual_emissions() > 0.0);
        assert_eq!(result.annual_sequestration(), 0.0);
    }

    #[test]
    fn test_ifm_has_positive_agb_change() {
        let result = compute_scenario(&ifm_input());
        // IFM: increased stocking → AGB growth
        let agb_change = result.pool_change.pool_changes
            .iter()
            .find(|c| c.pool == CarbonPool::AGB)
            .unwrap();
        assert!(agb_change.delta_tco2e_ha < 0.0, "IFM should increase AGB stock (negative delta = sequestration)");
    }

    #[test]
    fn test_non_forest_has_zero_biomass() {
        let state = LandState::non_forest("bare");
        assert_eq!(state.stem_volume_m3_ha, 0.0);
        let stock = MultiPoolStock::compute_all(
            1.0, 0.0,
            &BiomassParams::default(),
            &SocParams::default(),
            "test",
        );
        // With 0 stem volume, AGB pool should be 0
        let agb = stock.pools.iter().find(|p| p.pool == CarbonPool::AGB).unwrap();
        assert!((agb.tco2e_per_ha - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_scenario_result_annualized() {
        let mut input = afforestation_input();
        input.time_horizon_years = 30.0;
        let result = compute_scenario(&input);
        assert!((result.time_horizon_years - 30.0).abs() < 0.01);
        assert!((result.annual_delta_tco2e - result.pool_change.total_delta_tco2e / 30.0).abs() < 1.0);
    }

    #[test]
    fn test_ecozone_presets() {
        let tropical = EcoZone::TropicalMoist.biomass_params();
        assert!(tropical.bef > 1.0);

        let boreal = EcoZone::Boreal.biomass_params();
        assert!(boreal.deadwood_decay_rate < tropical.deadwood_decay_rate,
            "Boreal decay rate should be slower than tropical");
    }

    #[test]
    fn test_carbon_scenario_labels() {
        assert_eq!(CarbonScenario::Afforestation.label(), "Afforestation/Reforestation");
        assert_eq!(CarbonScenario::IFM.label(), "Improved Forest Management");
        assert_eq!(CarbonScenario::Deforestation.label(), "Deforestation");
    }

    #[test]
    fn test_scenario_is_sequestration() {
        assert!(CarbonScenario::Afforestation.is_sequestration_scenario());
        assert!(CarbonScenario::IFM.is_sequestration_scenario());
        assert!(!CarbonScenario::Deforestation.is_sequestration_scenario());
    }

    #[test]
    fn test_scenario_is_emission() {
        assert!(!CarbonScenario::Afforestation.is_emission_scenario());
        assert!(!CarbonScenario::IFM.is_emission_scenario());
        assert!(CarbonScenario::Deforestation.is_emission_scenario());
    }
}
