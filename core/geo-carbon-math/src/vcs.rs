//! VCS/CCB Methodology Mapping.
//!
//! Maps carbon scenarios to Verified Carbon Standard (Verra VCS) and
//! Climate, Community & Biodiversity (CCB) methodologies.
//!
//! Each methodology defines:
//! - Applicable scenario types (A/R, IFM, REDD+)
//! - Required pools (which of the 5 pools are mandatory)
//! - Default parameters (biomass defaults, SOC reference values)
//! - Permanence risk buffer (buffer withhold percentage)
//! - Monitoring requirements
//!
//! ## References
//! - VM0010: Methodology for Improved Forest Management v1.3
//! - VM0015: Methodology for Afforestation, Reforestation, Revegetation v1.1
//! - VM0007: REDD+ Methodology Framework v1.6
//! - VCS Standard v4.4
//! - CCB Standards v3.1

use serde::{Deserialize, Serialize};

use crate::pools::{BiomassParams, CarbonPool, SocParams};
use crate::scenarios::CarbonScenario;

// ── VCS Methodology ───────────────────────────────────────────

/// VCS methodology identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VcsMethodology {
    /// VM0010: Improved Forest Management (IFM)
    #[serde(rename = "VM0010")]
    VM0010,
    /// VM0015: Afforestation, Reforestation, Revegetation (ARR)
    #[serde(rename = "VM0015")]
    VM0015,
    /// VM0007: REDD+ Methodology Framework
    #[serde(rename = "VM0007")]
    VM0007,
    /// VM0006: Carbon Accounting for Mosaic REDD+ Projects
    #[serde(rename = "VM0006")]
    VM0006,
    /// VM0009: Methodology for Avoided Ecosystem Conversion
    #[serde(rename = "VM0009")]
    VM0009,
    /// VM0032: Methodology for Adoption of Sustainable Grasslands (SALM)
    #[serde(rename = "VM0032")]
    VM0032,
    /// VM0037: Methodology for REDD+ in Peatland Forest
    #[serde(rename = "VM0037")]
    VM0037,
    /// VM0042: Methodology for Improved Agricultural Land Management
    #[serde(rename = "VM0042")]
    VM0042,
    /// VM0046: Methodology for Avoided Deforestation
    #[serde(rename = "VM0046")]
    VM0046,
}

impl VcsMethodology {
    /// Full name of the methodology.
    pub fn label(&self) -> &'static str {
        match self {
            VcsMethodology::VM0010 => "VM0010 — Improved Forest Management v1.3",
            VcsMethodology::VM0015 => "VM0015 — Afforestation, Reforestation, Revegetation v1.1",
            VcsMethodology::VM0007 => "VM0007 — REDD+ Methodology Framework v1.6",
            VcsMethodology::VM0006 => "VM0006 — Carbon Accounting for Mosaic REDD+ Projects v2.2",
            VcsMethodology::VM0009 => "VM0009 — Avoided Ecosystem Conversion v3.0",
            VcsMethodology::VM0032 => "VM0032 — Sustainable Grasslands (SALM) v1.0",
            VcsMethodology::VM0037 => "VM0037 — REDD+ in Peatland Forest v1.0",
            VcsMethodology::VM0042 => "VM0042 — Improved Agricultural Land Management v2.0",
            VcsMethodology::VM0046 => "VM0046 — Avoided Deforestation v1.1",
        }
    }

    /// Applicable carbon scenarios for this methodology.
    pub fn applicable_scenarios(&self) -> &[CarbonScenario] {
        match self {
            VcsMethodology::VM0010 => &[CarbonScenario::IFM],
            VcsMethodology::VM0015 => &[CarbonScenario::Afforestation],
            VcsMethodology::VM0007 => &[CarbonScenario::Deforestation],
            VcsMethodology::VM0006 => &[CarbonScenario::Deforestation],
            VcsMethodology::VM0009 => &[CarbonScenario::Deforestation],
            VcsMethodology::VM0032 => &[CarbonScenario::Afforestation],
            VcsMethodology::VM0037 => &[CarbonScenario::Deforestation],
            VcsMethodology::VM0042 => &[CarbonScenario::Afforestation],
            VcsMethodology::VM0046 => &[CarbonScenario::Deforestation],
        }
    }

    /// Carbon pools required by this methodology.
    /// Some methodologies require all 5, others require a subset.
    pub fn required_pools(&self) -> &[CarbonPool] {
        match self {
            VcsMethodology::VM0010 => &[
                CarbonPool::AGB,
                CarbonPool::BGB,
                CarbonPool::Deadwood,
                CarbonPool::Litter,
                CarbonPool::SOC,
            ],
            VcsMethodology::VM0015 => &[
                CarbonPool::AGB,
                CarbonPool::BGB,
                CarbonPool::Deadwood,
                CarbonPool::Litter,
                CarbonPool::SOC,
            ],
            VcsMethodology::VM0007 => &[
                CarbonPool::AGB,
                CarbonPool::BGB,
                CarbonPool::Deadwood,
                CarbonPool::SOC, // Litter optional
            ],
            VcsMethodology::VM0006 => &[
                CarbonPool::AGB,
                CarbonPool::BGB,
                CarbonPool::Deadwood,
                CarbonPool::SOC,
            ],
            VcsMethodology::VM0009 => &[CarbonPool::AGB, CarbonPool::BGB, CarbonPool::SOC],
            VcsMethodology::VM0032 => &[CarbonPool::AGB, CarbonPool::BGB, CarbonPool::SOC],
            VcsMethodology::VM0037 => &[
                CarbonPool::AGB,
                CarbonPool::BGB,
                CarbonPool::Deadwood,
                CarbonPool::Litter,
                CarbonPool::SOC,
            ],
            VcsMethodology::VM0042 => &[
                CarbonPool::AGB,
                CarbonPool::BGB,
                CarbonPool::SOC,
            ],
            VcsMethodology::VM0046 => &[
                CarbonPool::AGB,
                CarbonPool::BGB,
                CarbonPool::Deadwood,
                CarbonPool::SOC,
            ],
        }
    }

    /// Default non-permanence risk buffer withholding percentage.
    ///
    /// VCS requires a percent of credits to be withheld in the buffer pool
    /// to cover potential reversals.
    pub fn buffer_withhold_pct(&self) -> f64 {
        match self {
            VcsMethodology::VM0010 => 15.0,       // IFM: 10-20%
            VcsMethodology::VM0015 => 20.0,       // A/R: 10-30%
            VcsMethodology::VM0007 => 15.0,       // REDD+: 10-20%
            VcsMethodology::VM0006 => 18.0,       // Mosaic REDD+
            VcsMethodology::VM0009 => 20.0,       // AEC
            VcsMethodology::VM0032 => 20.0,       // SALM
            VcsMethodology::VM0037 => 25.0,       // Peatland
            VcsMethodology::VM0042 => 15.0,       // IALM
            VcsMethodology::VM0046 => 12.0,       // Avoided DF
        }
    }

    /// Crediting period (years). VCS standard: 20-100 years for A/R, 20-60 for IFM.
    pub fn crediting_period_years(&self) -> u16 {
        match self {
            VcsMethodology::VM0010 => 30,
            VcsMethodology::VM0015 => 40,
            VcsMethodology::VM0007 => 30,
            VcsMethodology::VM0006 => 30,
            VcsMethodology::VM0009 => 30,
            VcsMethodology::VM0032 => 30,
            VcsMethodology::VM0037 => 30,
            VcsMethodology::VM0042 => 20,
            VcsMethodology::VM0046 => 30,
        }
    }

    /// Default biomass parameters for this methodology.
    pub fn default_biomass_params(&self) -> BiomassParams {
        match self {
            VcsMethodology::VM0010 | VcsMethodology::VM0015 => BiomassParams::default(),
            VcsMethodology::VM0007 | VcsMethodology::VM0006 | VcsMethodology::VM0046 => {
                BiomassParams::tropical_moist()
            }
            VcsMethodology::VM0009 => BiomassParams::temperate_broadleaf(),
            VcsMethodology::VM0032 => BiomassParams {
                wood_density: 0.40,
                bef: 1.2,
                carbon_fraction: 0.47,
                root_shoot_ratio: 0.30,
                deadwood_ratio: 0.05,
                litter_ratio: 0.03,
                litter_turnover: 0.6,
                deadwood_decay_rate: 0.10,
            },
            VcsMethodology::VM0037 => BiomassParams {
                wood_density: 0.35,
                bef: 1.8,
                carbon_fraction: 0.50, // higher for peat
                root_shoot_ratio: 0.20,
                deadwood_ratio: 0.20,
                litter_ratio: 0.10,
                litter_turnover: 0.3,
                deadwood_decay_rate: 0.03,
            },
            VcsMethodology::VM0042 => BiomassParams {
                wood_density: 0.50,
                bef: 1.5,
                carbon_fraction: 0.47,
                root_shoot_ratio: 0.25,
                deadwood_ratio: 0.05,
                litter_ratio: 0.04,
                litter_turnover: 0.6,
                deadwood_decay_rate: 0.08,
            },
        }
    }

    /// Default SOC parameters for this methodology.
    pub fn default_soc_params(&self) -> SocParams {
        match self {
            VcsMethodology::VM0010 | VcsMethodology::VM0015 => SocParams::default(),
            VcsMethodology::VM0007 | VcsMethodology::VM0006 | VcsMethodology::VM0046 => {
                SocParams {
                    soc_ref_tc_ha: 60.0,
                    flu: 1.0,
                    fmg: 1.0,
                    fi: 1.0,
                }
            }
            VcsMethodology::VM0009 => SocParams {
                soc_ref_tc_ha: 70.0,
                flu: 1.0,
                fmg: 1.0,
                fi: 1.0,
            },
            VcsMethodology::VM0032 => SocParams {
                soc_ref_tc_ha: 50.0,
                flu: 0.90,
                fmg: 1.0,
                fi: 0.95,
            },
            VcsMethodology::VM0037 => SocParams {
                soc_ref_tc_ha: 200.0, // peat soils have very high SOC
                flu: 1.0,
                fmg: 1.0,
                fi: 1.0,
            },
            VcsMethodology::VM0042 => SocParams {
                soc_ref_tc_ha: 50.0,
                flu: 0.92,
                fmg: 1.05,
                fi: 1.10,
            },
        }
    }

    /// CCS (Carbon Capture and Storage) compatibility.
    pub fn supports_ccs(&self) -> bool {
        false // No VCS methodology currently supports CCS
    }

    /// Whether CCB (Climate, Community & Biodiversity) co-certification is supported.
    pub fn supports_ccb(&self) -> bool {
        matches!(
            self,
            VcsMethodology::VM0015
                | VcsMethodology::VM0007
                | VcsMethodology::VM0006
                | VcsMethodology::VM0037
                | VcsMethodology::VM0046
        )
    }
}

// ── Methodology Config ────────────────────────────────────────

/// Full configuration for a VCS methodology application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyConfig {
    /// VCS methodology.
    pub methodology: VcsMethodology,
    /// Carbon scenario.
    pub scenario: CarbonScenario,
    /// Required carbon pools.
    pub required_pools: Vec<CarbonPool>,
    /// Biomass parameters.
    pub biomass_params: BiomassParams,
    /// SOC parameters.
    pub soc_params: SocParams,
    /// Buffer withhold percentage.
    pub buffer_pct: f64,
    /// Crediting period (years).
    pub crediting_period_years: u16,
    /// CCB co-certification enabled.
    pub ccb_enabled: bool,
    /// Method-specific notes.
    pub notes: Vec<String>,
}

impl MethodologyConfig {
    /// Create a default configuration for a given methodology and scenario.
    pub fn for_methodology(method: VcsMethodology, scenario: CarbonScenario) -> Result<Self, String> {
        if !method.applicable_scenarios().contains(&scenario) {
            return Err(format!(
                "Methodology {} does not support scenario {:?}",
                method.label(),
                scenario
            ));
        }

        Ok(Self {
            methodology: method,
            scenario,
            required_pools: method.required_pools().to_vec(),
            biomass_params: method.default_biomass_params(),
            soc_params: method.default_soc_params(),
            buffer_pct: method.buffer_withhold_pct(),
            crediting_period_years: method.crediting_period_years(),
            ccb_enabled: method.supports_ccb(),
            notes: Vec::new(),
        })
    }

    /// Compute net creditable carbon after buffer withholding.
    pub fn creditable_tco2e(&self, gross_tco2e: f64) -> f64 {
        gross_tco2e * (1.0 - self.buffer_pct / 100.0)
    }

    /// Check if a pool is required by this methodology.
    pub fn requires_pool(&self, pool: CarbonPool) -> bool {
        self.required_pools.contains(&pool)
    }
}

// ── Methodology Matching ──────────────────────────────────────

/// Match a scenario to the most appropriate VCS methodology.
///
/// Returns a list of compatible methodologies ordered by suitability.
pub fn match_methodologies(scenario: CarbonScenario) -> Vec<VcsMethodology> {
    let all = VcsMethodology::all();
    all.into_iter()
        .filter(|m| m.applicable_scenarios().contains(&scenario))
        .collect()
}

/// Get the default (primary) methodology for a scenario.
pub fn default_methodology(scenario: CarbonScenario) -> Option<VcsMethodology> {
    match scenario {
        CarbonScenario::Afforestation => Some(VcsMethodology::VM0015),
        CarbonScenario::IFM => Some(VcsMethodology::VM0010),
        CarbonScenario::Deforestation => Some(VcsMethodology::VM0007),
    }
}

impl VcsMethodology {
    /// All registered VCS methodologies.
    pub fn all() -> Vec<VcsMethodology> {
        vec![
            VcsMethodology::VM0010,
            VcsMethodology::VM0015,
            VcsMethodology::VM0007,
            VcsMethodology::VM0006,
            VcsMethodology::VM0009,
            VcsMethodology::VM0032,
            VcsMethodology::VM0037,
            VcsMethodology::VM0042,
            VcsMethodology::VM0046,
        ]
    }
}

// ── CCB Standards ─────────────────────────────────────────────

/// CCB co-benefit categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CcbBenefit {
    /// Climate adaptation benefits.
    ClimateAdaptation,
    /// Biodiversity conservation.
    Biodiversity,
    /// Community livelihoods.
    CommunityLivelihood,
    /// Water and soil conservation.
    WaterSoil,
    /// Indigenous peoples and cultural heritage.
    IndigenousCulture,
}

impl CcbBenefit {
    pub fn label(&self) -> &'static str {
        match self {
            CcbBenefit::ClimateAdaptation => "Climate Adaptation",
            CcbBenefit::Biodiversity => "Biodiversity Conservation",
            CcbBenefit::CommunityLivelihood => "Community Livelihoods",
            CcbBenefit::WaterSoil => "Water & Soil Conservation",
            CcbBenefit::IndigenousCulture => "Indigenous Peoples & Culture",
        }
    }
}

/// CCB certification status for a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CcbCertification {
    /// Whether CCB is applicable.
    pub applicable: bool,
    /// Co-benefits claimed.
    pub benefits: Vec<CcbBenefit>,
    /// Gold level status.
    pub gold_level: bool,
    /// Verification body.
    pub verifier: Option<String>,
}

impl Default for CcbCertification {
    fn default() -> Self {
        Self {
            applicable: false,
            benefits: Vec::new(),
            gold_level: false,
            verifier: None,
        }
    }
}

// ── VCS Project Summary ───────────────────────────────────────

/// Summary of VCS/CCB applicability for a given scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcsProjectSummary {
    /// Recommended methodology.
    pub methodology: VcsMethodology,
    /// Methodology label.
    pub methodology_label: String,
    /// Scenario type.
    pub scenario: CarbonScenario,
    /// CCB certification info.
    pub ccb: CcbCertification,
    /// Crediting period (years).
    pub crediting_period_years: u16,
    /// Buffer withholding percentage.
    pub buffer_pct: f64,
    /// Alternative methodologies.
    pub alternatives: Vec<String>,
}

impl VcsProjectSummary {
    pub fn new(scenario: CarbonScenario) -> Option<Self> {
        let method = default_methodology(scenario)?;
        let alternatives = match_methodologies(scenario)
            .into_iter()
            .filter(|m| *m != method)
            .map(|m| m.label().to_string())
            .collect();

        Some(Self {
            methodology: method,
            methodology_label: method.label().to_string(),
            scenario,
            ccb: CcbCertification {
                applicable: method.supports_ccb(),
                benefits: if method.supports_ccb() {
                    vec![CcbBenefit::Biodiversity, CcbBenefit::CommunityLivelihood]
                } else {
                    Vec::new()
                },
                gold_level: false,
                verifier: None,
            },
            crediting_period_years: method.crediting_period_years(),
            buffer_pct: method.buffer_withhold_pct(),
            alternatives,
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_afforestation() {
        let methods = match_methodologies(CarbonScenario::Afforestation);
        assert!(!methods.is_empty());
        assert!(methods.contains(&VcsMethodology::VM0015));
        assert!(methods.contains(&VcsMethodology::VM0032));
        assert!(methods.contains(&VcsMethodology::VM0042));
    }

    #[test]
    fn test_match_ifm() {
        let methods = match_methodologies(CarbonScenario::IFM);
        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0], VcsMethodology::VM0010);
    }

    #[test]
    fn test_match_deforestation() {
        let methods = match_methodologies(CarbonScenario::Deforestation);
        assert!(methods.len() >= 3);
        assert!(methods.contains(&VcsMethodology::VM0007));
    }

    #[test]
    fn test_default_methodology() {
        assert_eq!(
            default_methodology(CarbonScenario::Afforestation),
            Some(VcsMethodology::VM0015)
        );
        assert_eq!(
            default_methodology(CarbonScenario::IFM),
            Some(VcsMethodology::VM0010)
        );
        assert_eq!(
            default_methodology(CarbonScenario::Deforestation),
            Some(VcsMethodology::VM0007)
        );
    }

    #[test]
    fn test_required_pools() {
        // Most methodologies require AGB + BGB
        for method in VcsMethodology::all() {
            let pools = method.required_pools();
            assert!(pools.contains(&CarbonPool::AGB));
            assert!(pools.contains(&CarbonPool::BGB));
            assert!(!pools.is_empty());
        }
    }

    #[test]
    fn test_buffer_withhold() {
        let method = VcsMethodology::VM0015;
        assert!((method.buffer_withhold_pct() - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_methodology_config_creation() {
        let config =
            MethodologyConfig::for_methodology(VcsMethodology::VM0015, CarbonScenario::Afforestation)
                .unwrap();
        assert_eq!(config.required_pools.len(), 5);
        assert!(config.ccb_enabled);
        assert_eq!(config.crediting_period_years, 40);
    }

    #[test]
    fn test_methodology_config_wrong_scenario() {
        let result =
            MethodologyConfig::for_methodology(VcsMethodology::VM0010, CarbonScenario::Afforestation);
        assert!(result.is_err());
    }

    #[test]
    fn test_creditable_tco2e() {
        let config = MethodologyConfig::for_methodology(
            VcsMethodology::VM0015,
            CarbonScenario::Afforestation,
        )
        .unwrap();
        let gross = -100.0; // 100 tCO₂e sequestered
        let creditable = config.creditable_tco2e(gross);
        assert!((creditable - -80.0).abs() < 0.01); // 20% buffer
    }

    #[test]
    fn test_vcs_project_summary() {
        let summary = VcsProjectSummary::new(CarbonScenario::Afforestation).unwrap();
        assert_eq!(summary.methodology, VcsMethodology::VM0015);
        assert!(summary.ccb.applicable);
        assert_eq!(summary.ccb.benefits.len(), 2);
        assert!(!summary.alternatives.is_empty());
    }

    #[test]
    fn test_vcs_methodology_labels() {
        for method in VcsMethodology::all() {
            let label = method.label();
            assert!(!label.is_empty());
            assert!(label.contains('—'));
        }
    }

    #[test]
    fn test_ccb_benefit_labels() {
        assert_eq!(CcbBenefit::Biodiversity.label(), "Biodiversity Conservation");
        assert_eq!(CcbBenefit::WaterSoil.label(), "Water & Soil Conservation");
    }
}
