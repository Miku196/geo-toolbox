/// 波浪爬高与越浪模型。
///
/// 包含：
/// - Holman (1986) 波浪爬高公式
/// - Stockdon et al. (2006) 波浪爬高公式
/// - EurOtop (2018) 越浪量公式
/// - 波生增水（wave setup）简化模型

use serde::{Deserialize, Serialize};

/// 重力加速度 (m/s²)
const G: f64 = 9.81;

// ========== 基本数据结构 ==========

/// 波浪参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveParams {
    /// 有效波高 Hs (m)
    pub hs: f64,
    /// 谱峰周期 Tp (s)
    pub tp: f64,
    /// 深水波长 L0 = g·Tp²/(2π) (m)
    pub deep_water_wavelength: f64,
}

impl WaveParams {
    /// 由 Hs 和 Tp 构造，自动计算深水波长。
    /// L0 = g·Tp² / 2π
    pub fn new(hs: f64, tp: f64) -> Self {
        let l0 = G * tp * tp / (2.0 * std::f64::consts::PI);
        Self {
            hs,
            tp,
            deep_water_wavelength: l0,
        }
    }
}

/// 海滩/岸坡剖面参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeachProfile {
    /// 岸坡坡度 tanβ (垂直/水平)
    pub slope: f64,
    /// Iribarren 数（破波相似参数）
    /// ξ = tanβ / sqrt(Hs/L0)
    pub iribarren: f64,
}

impl BeachProfile {
    /// 由坡度和波浪参数构造，自动计算 Iribarren 数。
    pub fn new(slope: f64, wave: &WaveParams) -> Self {
        let iribarren = if wave.deep_water_wavelength > 0.0 && wave.hs > 0.0 {
            slope / (wave.hs / wave.deep_water_wavelength).sqrt()
        } else {
            0.0
        };
        Self { slope, iribarren }
    }
}

// ========== 爬高结果 ==========

/// 波浪爬高评估结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunupResult {
    /// Holman (1986) 2% 超越概率爬高 (m)
    pub r2_holman: f64,
    /// Stockdon et al. (2006) 2% 超越概率爬高 (m)
    pub r2_stockdon: f64,
    /// 波生增水高度 (m)
    pub setup: f64,
    /// Iribarren 数 ξ
    pub xi: f64,
    /// 岸坡坡度 tanβ
    pub slope: f64,
}

/// 越浪评估结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OvertoppingResult {
    /// 平均越浪单宽流量 (l/s/m)
    pub q_ls_m: f64,
    /// 胸墙超高 Rc (m) — 胸墙顶到静水位的距离
    pub rc: f64,
    /// 是否发生越浪
    pub overtopping: bool,
    /// EurOtop 危害等级
    pub hazard_level: OvertoppingHazard,
}

/// EurOtop (2018) 越浪危害等级。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OvertoppingHazard {
    /// q < 0.1 l/s/m — 极低
    VeryLow,
    /// 0.1 ≤ q < 1.0 l/s/m — 低
    Low,
    /// 1.0 ≤ q < 10 l/s/m — 中等
    Moderate,
    /// 10 ≤ q < 50 l/s/m — 高
    High,
    /// q ≥ 50 l/s/m — 极高
    VeryHigh,
}

// ========== 波生增水 ==========

/// 计算波生增水高度（shoreline setup）。
///
/// 简化公式（基于 Goda 经验修正）：
///   η_setup = min(0.25, 0.17 + 0.08·tanβ) × Hs
/// - 陡坡（tanβ > 0.1）趋近 0.25·Hs
/// - 缓坡（tanβ < 0.02）趋近 0.17·Hs
pub fn wave_setup(hs: f64, slope: f64) -> f64 {
    let factor = (0.17 + 0.08 * slope).min(0.25);
    factor * hs
}

// ========== 波浪爬高模型 ==========

/// Holman (1986) 波浪爬高公式。
///
/// 公式：
///   R2% = α · ξ · Hs
///   其中 α = 0.27（沙滩），α = 0.20（粗糙护坡/结构物）
///
/// 等价写为：
///   R2% = α · (tanβ · sqrt(Hs·L0))
///
/// # 参考文献
/// Holman, R.A. (1986). Extreme runup statistics on natural beaches.
/// Coastal Engineering, 10(2), 113-136.
pub fn holman_runup(wave: &WaveParams, beach: &BeachProfile, is_structure: bool) -> f64 {
    let alpha = if is_structure { 0.20 } else { 0.27 };
    let sqrt_hs_l0 = (wave.hs * wave.deep_water_wavelength).sqrt();
    alpha * beach.slope * sqrt_hs_l0
}

/// Stockdon et al. (2006) 波浪爬高公式。
///
/// 将爬高分解为 setup 和 swash 两部分：
///
/// 1. Wave setup 分量：
///    η = 0.35 · β · sqrt(Hs·L0)
///
/// 2. Incident swash 分量（入射频段）：
///    S_inc = 0.75 · β · sqrt(Hs·L0)
///
/// 3. Infragravity swash 分量（低频）：
///    S_ig = 0.06 · sqrt(Hs·L0)
///
/// 4. 总 swash（RMS 合成）：
///    S = sqrt(S_inc² + S_ig²)
///
/// 5. R2% = 1.1 · (η + S/2)
///
/// 对消散型海滩（ξ < 0.3），使用：
///    R2% = 0.043 · sqrt(Hs·L0)
///
/// 返回 (R2%, 波生增水 η)。
///
/// # 参考文献
/// Stockdon, H.F., Holman, R.A., Howd, P.A., & Sallenger, A.H. (2006).
/// Empirical parameterization of setup, swash, and runup.
/// Coastal Engineering, 53(7), 573-588.
pub fn stockdon_runup(wave: &WaveParams, beach: &BeachProfile) -> (f64, f64) {
    let sqrt_hs_l0 = (wave.hs * wave.deep_water_wavelength).sqrt();

    if beach.iribarren < 0.3 {
        // 消散型海滩
        let r2 = 0.043 * sqrt_hs_l0;
        let setup = 0.17 * wave.hs.min(sqrt_hs_l0 * 0.05);
        (r2, setup)
    } else {
        // 反射/中间型海滩
        let setup = 0.35 * beach.slope * sqrt_hs_l0;
        let s_inc = 0.75 * beach.slope * sqrt_hs_l0;
        let s_ig = 0.06 * sqrt_hs_l0;
        let s_total = (s_inc * s_inc + s_ig * s_ig).sqrt();
        let r2 = 1.1 * (setup + s_total / 2.0);
        (r2, setup)
    }
}

/// 综合波浪爬高评估，同时输出 Holman 和 Stockdon 结果。
pub fn assess_runup(wave: &WaveParams, beach: &BeachProfile, is_structure: bool) -> RunupResult {
    let r2_holman = holman_runup(wave, beach, is_structure);
    let (r2_stockdon, setup) = stockdon_runup(wave, beach);
    RunupResult {
        r2_holman,
        r2_stockdon,
        setup,
        xi: beach.iribarren,
        slope: beach.slope,
    }
}

// ========== 越浪模型 ==========

/// 根据平均越浪流量确定危害等级（EurOtop 2018）。
fn classify_hazard(q_ls_m: f64) -> OvertoppingHazard {
    if q_ls_m < 0.1 {
        OvertoppingHazard::VeryLow
    } else if q_ls_m < 1.0 {
        OvertoppingHazard::Low
    } else if q_ls_m < 10.0 {
        OvertoppingHazard::Moderate
    } else if q_ls_m < 50.0 {
        OvertoppingHazard::High
    } else {
        OvertoppingHazard::VeryHigh
    }
}

/// EurOtop (2018) 平均越浪量计算。
///
/// 斜坡式护岸（ξ < 5）：
///   q = 0.067 / sqrt(tanα) · ξ · exp(-4.75 · Rc / (Hs · ξ))
///   适用条件：0.1 < Rc/Hs < ξ
///   单位：m³/s/m，输出转换为 l/s/m
///
/// 直立式防波堤：
///   q = 0.047 · exp(-2.35 · Rc / Hs)
///
/// # 参数
/// - `wave`: 波浪参数
/// - `beach`: 岸坡参数（提供坡度 tanβ）
/// - `rc`: 胸墙超高 Rc (m)，即胸墙顶到静水位的距离
/// - `is_vertical`: 是否为直立式结构
///
/// # 参考文献
/// EurOtop (2018). Manual on wave overtopping of sea defences and related structures.
/// van der Meer, J.W., Allsop, N.W.H., et al.
pub fn eurotop_overtopping(
    wave: &WaveParams,
    beach: &BeachProfile,
    rc: f64,
    is_vertical: bool,
) -> Option<OvertoppingResult> {
    if wave.hs <= 0.0 {
        return None;
    }

    let q: f64;
    if rc <= 0.0 {
        // Weir flow regime: crest at or below SWL
        // Use simplified weir equation for continuous overtopping
        let h = (-rc).max(0.001); // water head above crest (m)
        q = 0.544 * (9.81_f64).sqrt() * h.powf(1.5); // broad-crested weir
    } else if is_vertical {
        q = 0.047 * (-2.35 * rc / wave.hs).exp();
    } else {
        let xi = beach.iribarren;
        let rc_hs = rc / wave.hs;
        if xi >= 5.0 || rc_hs < 0.1 || rc_hs > xi {
            return None; // parameters outside EurOtop validity range
        }
        let tan_alpha = beach.slope.max(0.01);
        q = 0.067 / tan_alpha.sqrt() * xi * (-4.75 * rc_hs / xi).exp();
    }

    let q_ls_m = q * 1000.0; // m³/s/m → l/s/m
    let overtopping = if rc <= 0.0 {
        true
    } else if is_vertical {
        q_ls_m > 0.01
    } else {
        q_ls_m > 0.001
    };

    Some(OvertoppingResult {
        q_ls_m,
        rc,
        overtopping,
        hazard_level: classify_hazard(q_ls_m),
    })
}

/// 天然海岸越浪评估（简化版）。
///
/// Rc = dune_crest_m - (swl_m + storm_surge_m + wave_setup_m)
/// 以 Rc 为输入调用 `eurotop_overtopping`，取斜坡式（is_vertical=false）。
///
/// # 参数
/// - `wave`: 波浪参数
/// - `beach`: 岸坡参数
/// - `dune_crest_m`: 沙丘/岸堤顶高度 (m，相对于基准面)
/// - `swl_m`: 静水位 (m，相对于基准面)
/// - `storm_surge_m`: 风暴增水 (m，来自风场模型)
pub fn natural_coast_overtopping(
    wave: &WaveParams,
    beach: &BeachProfile,
    dune_crest_m: f64,
    swl_m: f64,
    storm_surge_m: f64,
) -> OvertoppingResult {
    let setup = wave_setup(wave.hs, beach.slope);
    let total_surge = swl_m + storm_surge_m + setup;
    let rc = dune_crest_m - total_surge;

    // 尝试用 EurOtop 斜坡公式
    if let Some(result) = eurotop_overtopping(wave, beach, rc.max(0.0), false) {
        result
    } else {
        // 参数越界时的回退估计
        let q_ls_m = if rc <= 0.0 {
            // 胸墙被淹没（weir 流）
            100.0
        } else {
            0.001
        };
        OvertoppingResult {
            q_ls_m,
            rc,
            overtopping: rc <= 0.0 || q_ls_m > 0.001,
            hazard_level: classify_hazard(q_ls_m),
        }
    }
}

// ========== 测试 ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wave_params_new() {
        let w = WaveParams::new(2.0, 10.0);
        assert!((w.hs - 2.0).abs() < 1e-6);
        assert!((w.tp - 10.0).abs() < 1e-6);
        // L0 = 9.81 * 100 / (2*PI) ≈ 156.1
        let expected_l0 = 9.81 * 100.0 / (2.0 * std::f64::consts::PI);
        assert!((w.deep_water_wavelength - expected_l0).abs() < 0.1);
    }

    #[test]
    fn test_beach_profile() {
        let w = WaveParams::new(2.0, 10.0);
        let b = BeachProfile::new(0.05, &w);
        assert!((b.slope - 0.05).abs() < 1e-6);
        // ξ = 0.05 / sqrt(2.0 / 156.1) ≈ 0.05 / 0.1132 ≈ 0.4416
        assert!((b.iribarren - 0.4416).abs() < 0.01);
    }

    #[test]
    fn test_wave_setup() {
        let hs = 2.0;
        // steep slope: (0.17 + 0.08*0.1) * 2.0 = 0.356
        let setup_steep = wave_setup(hs, 0.1);
        assert!((setup_steep - 0.356).abs() < 0.01);

        // flat slope: (0.17 + 0.08*0.01) * 2.0 ≈ 0.3416
        let setup_flat = wave_setup(hs, 0.01);
        assert!((setup_flat - 0.34).abs() < 0.01);
    }

    #[test]
    fn test_holman_runup_typical() {
        let w = WaveParams::new(2.0, 10.0);
        let b = BeachProfile::new(0.05, &w);
        let r2 = holman_runup(&w, &b, false);
        // R2% = 0.27 * 0.05 * sqrt(2.0 * 156.1) ≈ 0.27 * 0.05 * 17.67 ≈ 0.2385
        assert!(r2 > 0.1 && r2 < 0.5);
    }

    #[test]
    fn test_holman_runup_structure() {
        let w = WaveParams::new(3.0, 12.0);
        let b = BeachProfile::new(0.08, &w);
        let r2 = holman_runup(&w, &b, true);
        // structure: α = 0.20
        // R2% = 0.20 * 0.08 * sqrt(3.0 * (9.81*144/2PI)) ≈ some positive value
        assert!(r2 > 0.0);
        // structure should give lower value than beach
        let r2_beach = holman_runup(&w, &b, false);
        assert!(r2 < r2_beach);
    }

    #[test]
    fn test_stockdon_runup_reflective() {
        let w = WaveParams::new(1.0, 8.0);
        let b = BeachProfile::new(0.15, &w);
        // ξ = 0.15 / sqrt(1.0/100) = 0.15/0.1 = 1.5 > 0.3 → 反射型
        let (r2, setup) = stockdon_runup(&w, &b);
        assert!(r2 > 0.0);
        assert!(setup > 0.0);
        assert!(r2 > setup); // runup > setup
    }

    #[test]
    fn test_stockdon_runup_dissipative() {
        let w = WaveParams::new(3.0, 12.0);
        let b = BeachProfile::new(0.01, &w);
        // ξ very small → dissipative
        let (r2, _setup) = stockdon_runup(&w, &b);
        // dissipative: R2% = 0.043 * sqrt(3 * 224.4) ≈ 0.043 * 25.94 ≈ 1.115
        assert!(r2 > 0.5 && r2 < 2.0);
    }

    #[test]
    fn test_assess_runup_contains_both() {
        let w = WaveParams::new(2.5, 11.0);
        let b = BeachProfile::new(0.06, &w);
        let result = assess_runup(&w, &b, false);
        assert!(result.r2_holman > 0.0);
        assert!(result.r2_stockdon > 0.0);
        assert!(result.setup > 0.0);
        assert!((result.xi - b.iribarren).abs() < 1e-6);
        assert!((result.slope - b.slope).abs() < 1e-6);
    }

    #[test]
    fn test_eurotop_overtopping_weir() {
        // rc=-0.5 → 漫顶流，流量应很大
        let w = WaveParams::new(2.0, 10.0);
        let b = BeachProfile::new(0.05, &w);
        let result = eurotop_overtopping(&w, &b, -0.5, false);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.q_ls_m > 10.0);
        assert!(r.overtopping);
    }

    #[test]
    fn test_eurotop_overtopping_no_overtopping() {
        // rc >> Hs → 无越浪
        let w = WaveParams::new(2.0, 10.0);
        let b = BeachProfile::new(0.30, &w);
        let result = eurotop_overtopping(&w, &b, 10.0, false);
        // Rc/Hs = 5.0, ξ = 0.3/sqrt(2/156) ≈ 2.65, 5 > 2.65 → 超出 ξ 范围
        // 应返回 None
        assert!(result.is_none() || result.unwrap().q_ls_m < 0.01);
    }

    #[test]
    fn test_eurotop_overtopping_vertical() {
        let w = WaveParams::new(3.0, 10.0);
        let b = BeachProfile::new(0.05, &w);
        let result = eurotop_overtopping(&w, &b, 2.0, true);
        assert!(result.is_some());
        let r = result.unwrap();
        // 直立式：q = 0.047 * exp(-2.35 * 2/3) ≈ 0.047 * 0.209 ≈ 0.0098 → 9.8 l/s/m
        assert!(r.q_ls_m > 0.1 && r.q_ls_m < 50.0);
    }

    #[test]
    fn test_eurotop_overtopping_negative_rc() {
        let w = WaveParams::new(2.0, 10.0);
        let b = BeachProfile::new(0.05, &w);
        let result = eurotop_overtopping(&w, &b, -1.0, false);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.q_ls_m > 100.0);
        assert!(r.overtopping);
    }

    #[test]
    fn test_natural_coast_no_overtopping() {
        let w = WaveParams::new(2.0, 10.0);
        let b = BeachProfile::new(0.05, &w);
        // dune 很高，无越浪
        let result = natural_coast_overtopping(&w, &b, 10.0, 0.0, 1.0);
        assert!(!result.overtopping || result.q_ls_m < 0.1);
    }

    #[test]
    fn test_natural_coast_overtopping_occurs() {
        let w = WaveParams::new(2.0, 10.0);
        let b = BeachProfile::new(0.05, &w);
        // dune 很低 + 风暴增水 → 越浪
        let result = natural_coast_overtopping(&w, &b, 1.0, 0.0, 2.5);
        assert!(result.overtopping);
    }

    #[test]
    fn test_wave_setup_edge_cases() {
        // zero wave height
        assert!((wave_setup(0.0, 0.05) - 0.0).abs() < 1e-6);
        // very steep
        let s = wave_setup(2.0, 1.0);
        assert!((s - 0.5).abs() < 0.01); // capped at 0.25 * 2.0
    }

    #[test]
    fn test_hazard_classification() {
        assert_eq!(classify_hazard(0.05), OvertoppingHazard::VeryLow);
        assert_eq!(classify_hazard(0.5), OvertoppingHazard::Low);
        assert_eq!(classify_hazard(5.0), OvertoppingHazard::Moderate);
        assert_eq!(classify_hazard(25.0), OvertoppingHazard::High);
        assert_eq!(classify_hazard(100.0), OvertoppingHazard::VeryHigh);
    }

    #[test]
    fn test_serialize_roundtrip() {
        let w = WaveParams::new(2.0, 10.0);
        let json = serde_json::to_string(&w).unwrap();
        let w2: WaveParams = serde_json::from_str(&json).unwrap();
        assert!((w2.hs - w.hs).abs() < 1e-6);
        assert!((w2.tp - w.tp).abs() < 1e-6);

        let result = assess_runup(&w, &BeachProfile::new(0.05, &w), false);
        let json = serde_json::to_string(&result).unwrap();
        let result2: RunupResult = serde_json::from_str(&json).unwrap();
        assert!((result2.r2_holman - result.r2_holman).abs() < 1e-6);

        let ot = OvertoppingResult {
            q_ls_m: 5.0,
            rc: 2.0,
            overtopping: true,
            hazard_level: OvertoppingHazard::Moderate,
        };
        let json = serde_json::to_string(&ot).unwrap();
        let ot2: OvertoppingResult = serde_json::from_str(&json).unwrap();
        assert_eq!(ot2.hazard_level, OvertoppingHazard::Moderate);
    }
}
