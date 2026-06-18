//! 蓝碳核算 — 滨海湿地/红树林/海草床碳储量与年固碳量。
//!
//! 参照 IPCC 2013 Wetlands Supplement:
//! - 红树林 (Mangrove)
//! - 盐沼 (Salt marsh)
//! - 海草床 (Seagrass)
//!
//! 每个生态系统分 3 碳库: 地上生物量 (AGB)、地下生物量 (BGB)、土壤 (Soil, 1m)。

use serde::Serialize;

/// 蓝碳生态系统类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueCarbonEcosystem {
    /// 红树林
    Mangrove,
    /// 盐沼（芦苇/碱蓬等）
    SaltMarsh,
    /// 海草床
    Seagrass,
}

impl BlueCarbonEcosystem {
    pub fn name(&self) -> &str {
        match self {
            BlueCarbonEcosystem::Mangrove => "红树林",
            BlueCarbonEcosystem::SaltMarsh => "盐沼",
            BlueCarbonEcosystem::Seagrass => "海草床",
        }
    }
}

/// 单个碳库储量 (t CO₂e / ha)。
#[derive(Debug, Clone, Copy, Serialize)]
pub struct CarbonPool {
    /// 碳密度 (t C / ha)
    pub carbon_density_tc_ha: f64,
    /// 转换为 CO₂ 当量 (t CO₂e / ha)
    pub co2e_t_ha: f64,
}

/// 蓝碳估算结果。
#[derive(Debug, Clone, Serialize)]
pub struct BlueCarbonResult {
    /// 生态系统名称
    pub ecosystem: String,
    /// 面积 (ha)
    pub area_ha: f64,
    /// 总碳储量 (t CO₂e)
    pub total_stock_tco2e: f64,
    /// 年固碳量 (t CO₂e / yr)
    pub annual_seq_tco2e_yr: f64,
    /// 各碳库明细
    pub pools: BlueCarbonPoolBreakdown,
    /// 说明
    pub summary: String,
}

/// 三碳库分解。
#[derive(Debug, Clone, Copy, Serialize)]
pub struct BlueCarbonPoolBreakdown {
    pub agb: CarbonPool,
    pub bgb: CarbonPool,
    pub soil: CarbonPool,
}

impl BlueCarbonPoolBreakdown {
    fn total_tco2e(&self) -> f64 {
        self.agb.co2e_t_ha + self.bgb.co2e_t_ha + self.soil.co2e_t_ha
    }
}

// ─── IPCC Tier 1 default values ──────────────────────────────────────────
// 来源: IPCC 2013 Wetlands Supplement, Table 4.1–4.7
// 单位: t C / ha (碳密度), t C / ha / yr (固碳率)
// C→CO₂ 转换: × 44/12 = × 3.6667

const IPCC_MANGROVE_AGB: f64 = 118.3; // t C / ha
const IPCC_MANGROVE_BGB: f64 = 26.7;
const IPCC_MANGROVE_SOIL: f64 = 387.0;
const IPCC_MANGROVE_SEQ: f64 = 1.48; // t C / ha / yr (AGB accretion)

const IPCC_SALTMARSH_AGB: f64 = 3.0;
const IPCC_SALTMARSH_BGB: f64 = 10.2;
const IPCC_SALTMARSH_SOIL: f64 = 220.0;
const IPCC_SALTMARSH_SEQ: f64 = 1.19;

const IPCC_SEAGRASS_AGB: f64 = 1.2;
const IPCC_SEAGRASS_BGB: f64 = 4.8;
const IPCC_SEAGRASS_SOIL: f64 = 140.0;
const IPCC_SEAGRASS_SEQ: f64 = 0.65;

const C_TO_CO2: f64 = 44.0 / 12.0; // 3.6667

fn pool(tc_ha: f64) -> CarbonPool {
    CarbonPool {
        carbon_density_tc_ha: tc_ha,
        co2e_t_ha: tc_ha * C_TO_CO2,
    }
}

/// 估算蓝碳储量与年固碳量。
///
/// # Arguments
/// * `ecosystem` — 生态系统类型
/// * `area_ha` — 生态系统面积 (ha)
/// * `soil_factor` — 土壤碳密度缩放因子 (默认 1.0，可调)
///
/// # Returns
/// `BlueCarbonResult` 包含总储量、年固碳量、碳库分解。
pub fn assess_blue_carbon(
    ecosystem: BlueCarbonEcosystem,
    area_ha: f64,
    soil_factor: f64,
) -> BlueCarbonResult {
    let (agb_tc, bgb_tc, soil_tc, seq_tc_yr) = match ecosystem {
        BlueCarbonEcosystem::Mangrove => (
            IPCC_MANGROVE_AGB,
            IPCC_MANGROVE_BGB,
            IPCC_MANGROVE_SOIL * soil_factor,
            IPCC_MANGROVE_SEQ,
        ),
        BlueCarbonEcosystem::SaltMarsh => (
            IPCC_SALTMARSH_AGB,
            IPCC_SALTMARSH_BGB,
            IPCC_SALTMARSH_SOIL * soil_factor,
            IPCC_SALTMARSH_SEQ,
        ),
        BlueCarbonEcosystem::Seagrass => (
            IPCC_SEAGRASS_AGB,
            IPCC_SEAGRASS_BGB,
            IPCC_SEAGRASS_SOIL * soil_factor,
            IPCC_SEAGRASS_SEQ,
        ),
    };

    let pools = BlueCarbonPoolBreakdown {
        agb: pool(agb_tc),
        bgb: pool(bgb_tc),
        soil: pool(soil_tc),
    };

    let total_stock_tco2e = pools.total_tco2e() * area_ha;
    let annual_seq_tco2e_yr = seq_tc_yr * C_TO_CO2 * area_ha;

    let summary = format!(
        "{} {} ha | 碳储量 {:.0} t CO₂e | 年固碳 {:.0} t CO₂e/yr",
        ecosystem.name(),
        area_ha,
        total_stock_tco2e,
        annual_seq_tco2e_yr,
    );

    BlueCarbonResult {
        ecosystem: ecosystem.name().to_string(),
        area_ha,
        total_stock_tco2e,
        annual_seq_tco2e_yr,
        pools,
        summary,
    }
}

/// 多生态系统蓝碳汇总。
#[derive(Debug, Clone, Serialize)]
pub struct BlueCarbonAggregate {
    pub items: Vec<BlueCarbonResult>,
    pub total_area_ha: f64,
    pub total_stock_tco2e: f64,
    pub total_seq_tco2e_yr: f64,
}

/// 聚合评估多个蓝碳斑块。
pub fn aggregate_blue_carbon(items: Vec<BlueCarbonResult>) -> BlueCarbonAggregate {
    let total_area_ha = items.iter().map(|x| x.area_ha).sum();
    let total_stock_tco2e = items.iter().map(|x| x.total_stock_tco2e).sum();
    let total_seq_tco2e_yr = items.iter().map(|x| x.annual_seq_tco2e_yr).sum();
    BlueCarbonAggregate {
        items,
        total_area_ha,
        total_stock_tco2e,
        total_seq_tco2e_yr,
    }
}

// ─── CoastalPlugin methods ──────────────────────────────────────────────

use crate::CoastalPlugin;
use geo_core::errors::GeoResult;

impl CoastalPlugin {
    /// 蓝碳储量评估。
    pub fn assess_blue_carbon(
        &self,
        ecosystem: &str,
        area_ha: f64,
        soil_factor: f64,
    ) -> GeoResult<BlueCarbonResult> {
        let eco = match ecosystem {
            "mangrove" => BlueCarbonEcosystem::Mangrove,
            "salt_marsh" | "saltmarsh" => BlueCarbonEcosystem::SaltMarsh,
            "seagrass" => BlueCarbonEcosystem::Seagrass,
            other => {
                return Err(geo_core::GeoError::InvalidInput {
                    field: "ecosystem".into(),
                    reason: format!(
                        "unknown ecosystem type: {other}, expected mangrove|salt_marsh|seagrass"
                    ),
                });
            }
        };
        Ok(assess_blue_carbon(eco, area_ha, soil_factor))
    }

    /// 聚合评估多个蓝碳斑块。
    pub fn aggregate_blue_carbon(&self, items: Vec<BlueCarbonResult>) -> BlueCarbonAggregate {
        aggregate_blue_carbon(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mangrove_default() {
        let r = assess_blue_carbon(BlueCarbonEcosystem::Mangrove, 100.0, 1.0);
        let expected_stock =
            (IPCC_MANGROVE_AGB + IPCC_MANGROVE_BGB + IPCC_MANGROVE_SOIL) * C_TO_CO2 * 100.0;
        assert!((r.total_stock_tco2e - expected_stock).abs() < 1.0);
        assert!(r.area_ha == 100.0);
        assert!(r.annual_seq_tco2e_yr > 0.0);
        assert!(r.pools.agb.co2e_t_ha > 0.0);
        assert!(r.pools.bgb.co2e_t_ha > 0.0);
        assert!(r.pools.soil.co2e_t_ha > 0.0);
    }

    #[test]
    fn test_saltmarsh_default() {
        let r = assess_blue_carbon(BlueCarbonEcosystem::SaltMarsh, 50.0, 0.8);
        let expected =
            (IPCC_SALTMARSH_AGB + IPCC_SALTMARSH_BGB + IPCC_SALTMARSH_SOIL * 0.8) * C_TO_CO2 * 50.0;
        assert!((r.total_stock_tco2e - expected).abs() < 1.0);
        assert!(r.ecosystem == "盐沼");
    }

    #[test]
    fn test_seagrass_default() {
        let r = assess_blue_carbon(BlueCarbonEcosystem::Seagrass, 200.0, 1.0);
        assert!(r.annual_seq_tco2e_yr > 0.0);
    }

    #[test]
    fn test_soil_factor_scales() {
        let r1 = assess_blue_carbon(BlueCarbonEcosystem::Mangrove, 1.0, 1.0);
        let r2 = assess_blue_carbon(BlueCarbonEcosystem::Mangrove, 1.0, 2.0);
        // soil doubles → stock should be larger
        assert!(r2.total_stock_tco2e > r1.total_stock_tco2e);
        // agb/bgb unchanged
        assert!((r1.pools.agb.co2e_t_ha - r2.pools.agb.co2e_t_ha).abs() < 0.01);
    }

    #[test]
    fn test_aggregate() {
        let items = vec![
            assess_blue_carbon(BlueCarbonEcosystem::Mangrove, 100.0, 1.0),
            assess_blue_carbon(BlueCarbonEcosystem::SaltMarsh, 50.0, 1.0),
        ];
        let agg = aggregate_blue_carbon(items);
        assert!((agg.total_area_ha - 150.0).abs() < 0.01);
        assert!(agg.total_stock_tco2e > 0.0);
        assert!(agg.items.len() == 2);
    }

    #[test]
    fn test_plugin_method() {
        let p = CoastalPlugin::new();
        let r = p.assess_blue_carbon("mangrove", 10.0, 1.0).unwrap();
        assert!(r.ecosystem == "红树林");
        assert!(r.total_stock_tco2e > 0.0);
    }

    #[test]
    fn test_unknown_ecosystem_error() {
        let p = CoastalPlugin::new();
        let err = p.assess_blue_carbon("coral", 10.0, 1.0).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("unknown ecosystem"), "got: {msg}");
    }
}
