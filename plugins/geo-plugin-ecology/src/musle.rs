/// MUSLE — Modified Universal Soil Loss Equation
///
/// 与 RUSLE（年均）不同，MUSLE 计算**单次暴雨**的土壤流失量。
/// 输入来自水文模块 SCS-CN 的径流量 Q 和洪峰 qp，
/// 复用现有 RUSLE 的 K、LS、C、P 因子。
///
/// 公式 (Williams & Berndt, 1977):
///   A = 11.8 × (Q × qp)^0.56 × K × LS × C × P
///
/// 其中:
///   A  = 单次暴雨土壤流失量 (metric tons)
///   Q  = 暴雨径流总量 (m³)
///   qp = 洪峰流量 (m³/s)
///   K  = 土壤可蚀性因子
///   LS = 坡长-坡度因子
///   C  = 覆盖管理因子
///   P  = 水土保持措施因子
///
/// # 参考文献
/// Williams, J.R., & Berndt, H.D. (1977). Sediment yield prediction based on
/// watershed hydrology. Transactions of the ASAE, 20(6), 1100-1104.
/// Williams, J.R. (1975). Sediment-yield prediction with universal equation
/// using runoff energy factor. ARS-S-40, USDA.

use serde::{Deserialize, Serialize};

// ─── MUSLE 结果 ───

/// MUSLE 单次暴雨侵蚀结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusleResult {
    /// 总土壤流失量 (metric tons)
    pub soil_loss_t: f64,
    /// 径流总量 (m³)
    pub runoff_vol_m3: f64,
    /// 洪峰流量 (m³/s)
    pub peak_flow_m3s: f64,
    /// 径流能量因子 (Q × qp)^0.56
    pub runoff_energy_factor: f64,
    /// K 因子
    pub k_factor: f64,
    /// LS 因子
    pub ls_factor: f64,
    /// C 因子
    pub c_factor: f64,
    /// P 因子
    pub p_factor: f64,
    /// 单位面积侵蚀量 (t/ha)
    pub soil_loss_per_ha: f64,
}

// ─── MUSLE 核心 ───

/// 计算单次暴雨 MUSLE 土壤流失量。
///
/// # 参数
/// * `runoff_m3` — 暴雨径流总量 (m³)，来自 SCS-CN 产流
/// * `peak_flow_m3s` — 洪峰流量 (m³/s)，来自单位线或经验公式
/// * `k` — 土壤可蚀性因子
/// * `ls` — 坡长-坡度因子
/// * `c` — 覆盖管理因子 (0-1)
/// * `p` — 水土保持措施因子 (0-1)
/// * `area_ha` — 流域面积 (ha)，用于计算单位面积侵蚀量
pub fn musle_soil_loss(
    runoff_m3: f64,
    peak_flow_m3s: f64,
    k: f64,
    ls: f64,
    c: f64,
    p: f64,
    area_ha: f64,
) -> f64 {
    let energy_factor = (runoff_m3 * peak_flow_m3s).powf(0.56);
    11.8 * energy_factor * k * ls * c * p
}

/// 完整 MUSLE 评估，返回结构化结果。
pub fn assess_musle(
    runoff_m3: f64,
    peak_flow_m3s: f64,
    k: f64,
    ls: f64,
    c: f64,
    p: f64,
    area_ha: f64,
) -> MusleResult {
    let energy_factor = (runoff_m3 * peak_flow_m3s).powf(0.56);
    let soil_loss_t = 11.8 * energy_factor * k * ls * c * p;
    MusleResult {
        soil_loss_t,
        runoff_vol_m3: runoff_m3,
        peak_flow_m3s,
        runoff_energy_factor: energy_factor,
        k_factor: k,
        ls_factor: ls,
        c_factor: c,
        p_factor: p,
        soil_loss_per_ha: if area_ha > 0.0 {
            soil_loss_t / area_ha
        } else {
            0.0
        },
    }
}

/// 从 SCS-CN 产流结果估算洪峰流量 (简化三角形单位线法)。
///
/// 适用于 A < 50 km² 的小流域。
///
/// # 参数
/// * `runoff_m3` — 径流总量
/// * `tc_hours` — 汇流时间 (h)
/// * `rain_distribution` — 暴雨时程分布因子 (SCS Type II ≈ 0.75)
pub fn estimate_peak_scs_triangular(
    runoff_m3: f64,
    tc_hours: f64,
    rain_distribution: f64,
) -> f64 {
    // SCS 三角单位线峰值: qp = 2.083 * A * Q / tp
    // 其中 tp = 0.6 * tc, Q = runoff depth (mm over area)
    // 简化：qp = rain_distribution * runoff_m3 / (tc_hours * 3600.0)
    if tc_hours <= 0.0 {
        return runoff_m3;
    }
    rain_distribution * runoff_m3 / (tc_hours * 3600.0)
}

/// 事件暴雨侵蚀等级（按 MUSLE 单位面积侵蚀量）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MusleSeverity {
    /// < 2 t/ha — 极低
    VeryLow,
    /// 2-10 t/ha — 低
    Low,
    /// 10-50 t/ha — 中等
    Moderate,
    /// 50-100 t/ha — 高
    High,
    /// > 100 t/ha — 极高
    VeryHigh,
}

impl MusleSeverity {
    pub fn from_soil_loss_per_ha(per_ha: f64) -> Self {
        if per_ha < 2.0 {
            MusleSeverity::VeryLow
        } else if per_ha < 10.0 {
            MusleSeverity::Low
        } else if per_ha < 50.0 {
            MusleSeverity::Moderate
        } else if per_ha < 100.0 {
            MusleSeverity::High
        } else {
            MusleSeverity::VeryHigh
        }
    }
}

// ─── 批量评估 ───

/// 对一系列暴雨事件进行 MUSLE 评估。
pub fn musle_event_assessment(
    events: &[(f64, f64)], // (runoff_m3, peak_flow_m3s) 每场暴雨
    k: f64,
    ls: f64,
    c: f64,
    p: f64,
    area_ha: f64,
) -> Vec<MusleResult> {
    events
        .iter()
        .map(|&(q, qp)| assess_musle(q, qp, k, ls, c, p, area_ha))
        .collect()
}

/// 多场暴雨年均侵蚀量。
pub fn musle_annual_average(
    events: &[(f64, f64)],
    k: f64,
    ls: f64,
    c: f64,
    p: f64,
    area_ha: f64,
) -> f64 {
    let results = musle_event_assessment(events, k, ls, c, p, area_ha);
    let total: f64 = results.iter().map(|r| r.soil_loss_t).sum();
    if results.is_empty() {
        0.0
    } else {
        total / results.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_musle_basic() {
        // 小型农业流域典型参数
        let a = musle_soil_loss(
            5000.0,  // Q = 5000 m³
            2.5,     // qp = 2.5 m³/s
            0.25,    // K — silt loam
            3.0,     // LS — 10% slope, 100m length
            0.3,     // C — row crops
            0.5,     // P — contour farming
            5.0,     // 5 ha
        );
        // Williams & Berndt formula: should yield significant loss
        assert!(a > 10.0, "Expected significant soil loss, got {}", a);
        assert!(a < 1000.0, "Expected bounded soil loss, got {}", a);
    }

    #[test]
    fn test_musle_zero_runoff() {
        let a = musle_soil_loss(0.0, 0.0, 0.25, 1.0, 0.1, 1.0, 1.0);
        assert!((a - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_musle_result() {
        let result = assess_musle(10000.0, 5.0, 0.28, 2.5, 0.25, 0.6, 10.0);
        assert!(result.soil_loss_t > 0.0);
        assert!(result.soil_loss_per_ha > 0.0);
        assert_eq!(result.k_factor, 0.28);
    }

    #[test]
    fn test_peak_estimate() {
        let qp = estimate_peak_scs_triangular(5000.0, 1.5, 0.75);
        assert!(qp > 0.0);
        assert!(qp < 10.0); // reasonable peak for small watershed
    }

    #[test]
    fn test_musle_severity() {
        assert_eq!(MusleSeverity::from_soil_loss_per_ha(1.0), MusleSeverity::VeryLow);
        assert_eq!(MusleSeverity::from_soil_loss_per_ha(5.0), MusleSeverity::Low);
        assert_eq!(MusleSeverity::from_soil_loss_per_ha(30.0), MusleSeverity::Moderate);
        assert_eq!(MusleSeverity::from_soil_loss_per_ha(80.0), MusleSeverity::High);
        assert_eq!(MusleSeverity::from_soil_loss_per_ha(200.0), MusleSeverity::VeryHigh);
    }

    #[test]
    fn test_musle_multi_event() {
        let events = vec![
            (3000.0, 1.5),
            (8000.0, 4.0),
            (2000.0, 1.0),
        ];
        let results = musle_event_assessment(&events, 0.25, 2.0, 0.3, 0.5, 5.0);
        assert_eq!(results.len(), 3);
        // Larger event should produce more erosion
        assert!(results[1].soil_loss_t > results[0].soil_loss_t);
        assert!(results[1].soil_loss_t > results[2].soil_loss_t);
    }

    #[test]
    fn test_musle_annual() {
        let events = vec![
            (3000.0, 1.5),
            (8000.0, 4.0),
            (2000.0, 1.0),
        ];
        let avg = musle_annual_average(&events, 0.25, 2.0, 0.3, 0.5, 5.0);
        assert!(avg > 0.0);
    }
}
