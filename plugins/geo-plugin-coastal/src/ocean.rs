/// 海洋物理过程：潮汐调和分析、Ekman 输运、SST/ENSO 指数、简化 SWAN 波浪传播。
///
/// 全部纯 Rust 实现，复用 coastal 插件现有基础设施。
use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────
// 1. 潮汐调和分析
// ──────────────────────────────────────────────

/// 天文分潮。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TidalConstituent {
    /// 分潮名称 (M2/S2/K1/O1)
    pub name: String,
    /// 角速度 (°/hour)
    pub speed_deg_hr: f64,
    /// 振幅 (m)
    pub amplitude_m: f64,
    /// 相位滞后 (°)
    pub phase_deg: f64,
}

/// 潮汐预报结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TidalPrediction {
    /// 分潮列表
    pub constituents: Vec<TidalConstituent>,
    /// 时间序列 (hours_from_ref, water_level_m)
    pub predictions: Vec<(f64, f64)>,
    /// 平均大潮高潮位
    pub mhws: f64,
    /// 平均大潮低潮位
    pub mlws: f64,
    /// 平均小潮高潮位
    pub mhwn: f64,
    /// 平均小潮低潮位
    pub mlwn: f64,
    /// 平均海平面
    pub msl: f64,
}

/// 4 个主要分潮标准角速度 (°/hr)。
pub fn constituent_speed(name: &str) -> f64 {
    match name {
        "M2" => 28.984104,  // 主太阴半日潮
        "S2" => 30.000000,  // 主太阳半日潮
        "N2" => 28.439730,  // 椭率太阴半日潮
        "K1" => 15.041069,  // 太阴太阳全日潮
        "O1" => 13.943036,  // 主太阴全日潮
        "P1" => 14.958931,  // 主太阳全日潮
        "K2" => 30.082138,  // 椭率太阴太阳半日潮
        "M4" => 57.968208,  // 浅水分潮
        "MS4" => 58.984104, // 浅水分潮
        _ => 0.0,
    }
}

/// 构建标准分潮列表。
pub fn standard_constituents(
    ampl_m2: f64,
    phase_m2: f64,
    ampl_s2: f64,
    phase_s2: f64,
    ampl_k1: f64,
    phase_k1: f64,
    ampl_o1: f64,
    phase_o1: f64,
) -> Vec<TidalConstituent> {
    vec![
        TidalConstituent {
            name: "M2".into(),
            speed_deg_hr: constituent_speed("M2"),
            amplitude_m: ampl_m2,
            phase_deg: phase_m2,
        },
        TidalConstituent {
            name: "S2".into(),
            speed_deg_hr: constituent_speed("S2"),
            amplitude_m: ampl_s2,
            phase_deg: phase_s2,
        },
        TidalConstituent {
            name: "K1".into(),
            speed_deg_hr: constituent_speed("K1"),
            amplitude_m: ampl_k1,
            phase_deg: phase_k1,
        },
        TidalConstituent {
            name: "O1".into(),
            speed_deg_hr: constituent_speed("O1"),
            amplitude_m: ampl_o1,
            phase_deg: phase_o1,
        },
    ]
}

/// 潮汐调和预报。
///
/// η(t) = Σ A_i · cos(ω_i · t + φ_i)
/// 对每个分潮 i，振幅 A_i、角速度 ω_i、相位 φ_i。
pub fn predict_tide(
    constituents: &[TidalConstituent],
    time_step_hours: f64,
    n_steps: usize,
    ref_time_hours: f64,
) -> TidalPrediction {
    let mut predictions = Vec::with_capacity(n_steps);
    for i in 0..n_steps {
        let t = ref_time_hours + i as f64 * time_step_hours;
        let mut level = 0.0;
        for c in constituents {
            let omega_rad = c.speed_deg_hr.to_radians();
            let phase_rad = c.phase_deg.to_radians();
            level += c.amplitude_m * (omega_rad * t + phase_rad).cos();
        }
        predictions.push((t, level));
    }

    // 统计潮位
    let msl = predictions.iter().map(|p| p.1).sum::<f64>() / predictions.len() as f64;

    TidalPrediction {
        constituents: constituents.to_vec(),
        predictions,
        mhws: 0.0,
        mlws: 0.0,
        mhwn: 0.0,
        mlwn: 0.0,
        msl,
    }
}

/// 计算大潮/小潮振幅。
/// 大潮 ≈ M2 + S2 振幅和，小潮 ≈ |M2 - S2|
pub fn spring_neap_heights(constituents: &[TidalConstituent]) -> (f64, f64) {
    let mut ampl_m2 = 0.0;
    let mut ampl_s2 = 0.0;
    for c in constituents {
        if c.name == "M2" {
            ampl_m2 = c.amplitude_m;
        }
        if c.name == "S2" {
            ampl_s2 = c.amplitude_m;
        }
    }
    let spring = ampl_m2 + ampl_s2;
    let neap = (ampl_m2 - ampl_s2).abs();
    (spring, neap)
}

/// 潮汐范围统计。
pub fn tidal_range_stats(predictions: &[(f64, f64)]) -> (f64, f64, f64) {
    let min = predictions
        .iter()
        .map(|p| p.1)
        .fold(f64::INFINITY, f64::min);
    let max = predictions
        .iter()
        .map(|p| p.1)
        .fold(f64::NEG_INFINITY, f64::max);
    (min, max, max - min)
}

// ──────────────────────────────────────────────
// 2. Ekman 输运
// ──────────────────────────────────────────────

/// Ekman 输运计算结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EkmanResult {
    /// 表层流速 (m/s)
    pub surface_speed_ms: f64,
    /// 表层流向 (° from north)
    pub direction_deg: f64,
    /// 单位宽度输运量 (m³/s/m)
    pub transport_m3_s_m: f64,
    /// 东向输运分量 (m³/s/m)
    pub u_transport: f64,
    /// 北向输运分量 (m³/s/m)
    pub v_transport: f64,
    /// Ekman 深度 (m)
    pub ekman_depth_m: f64,
}

/// 计算 Ekman 输运和表层流。
///
/// 风应力: τ = ρ_air · Cd · U10²
/// ρ_air = 1.225 kg/m³, Cd = 0.0013
/// Ekman 深度: De = √(2·Az/f)，Az=0.1 m²/s
/// 输运: Mx = τ_y/(ρ·f), My = -τ_x/(ρ·f)
/// 表层流: u_s = √2·τ_x/(ρ·f·De), v_s = √2·τ_y/(ρ·f·De)
pub fn ekman_transport(wind_speed_ms: f64, wind_direction_deg: f64, lat: f64) -> EkmanResult {
    let rho_air = 1.225;
    let cd = 0.0013;
    let omega = 7.2921e-5; // Earth rotation rate (rad/s)
    let f = 2.0 * omega * lat.to_radians().sin().abs().max(1e-10); // Coriolis
    let rho_water = 1025.0;
    let az = 0.1;

    let wind_rad = wind_direction_deg.to_radians();
    let tau_x = rho_air * cd * wind_speed_ms.powi(2) * wind_rad.sin(); // eastward
    let tau_y = rho_air * cd * wind_speed_ms.powi(2) * wind_rad.cos(); // northward

    let de = (2.0 * az / f).sqrt();
    let transport_u = tau_y / (rho_water * f);
    let transport_v = -tau_x / (rho_water * f);
    let transport = (transport_u.powi(2) + transport_v.powi(2)).sqrt();

    let u_surface = 2.0f64.sqrt() * tau_x / (rho_water * f * de);
    let v_surface = 2.0f64.sqrt() * tau_y / (rho_water * f * de);
    let speed = (u_surface.powi(2) + v_surface.powi(2)).sqrt();
    let dir = (u_surface.atan2(v_surface).to_degrees() + 360.0) % 360.0;

    EkmanResult {
        surface_speed_ms: speed,
        direction_deg: dir,
        transport_m3_s_m: transport,
        u_transport: transport_u,
        v_transport: transport_v,
        ekman_depth_m: de,
    }
}

/// 地转流：海面高度梯度 → 地转流速。
/// u_g = -(g/f) · d(SSH)/dy
/// v_g =  (g/f) · d(SSH)/dx
pub fn geostrophic_current(ssh_gradient_x: f64, ssh_gradient_y: f64, lat: f64) -> (f64, f64) {
    let g = 9.81;
    let omega = 7.2921e-5;
    let f = 2.0 * omega * lat.to_radians().sin().abs().max(1e-10);
    let v_g = (g / f) * ssh_gradient_x;
    let u_g = -(g / f) * ssh_gradient_y;
    (u_g, v_g)
}

/// 总表层流 = 地转流 + Ekman 流。
pub fn total_surface_current(
    wind_speed_ms: f64,
    wind_dir_deg: f64,
    ssh_grad_x: f64,
    ssh_grad_y: f64,
    lat: f64,
) -> (f64, f64, f64) {
    let (u_e, v_e) = {
        let ek = ekman_transport(wind_speed_ms, wind_dir_deg, lat);
        (ek.u_transport, ek.v_transport)
    };
    let (u_g, v_g) = geostrophic_current(ssh_grad_x, ssh_grad_y, lat);
    let u = u_e + u_g;
    let v = v_e + v_g;
    let speed = (u.powi(2) + v.powi(2)).sqrt();
    (u, v, speed)
}

// ──────────────────────────────────────────────
// 3. ENSO 指数
// ──────────────────────────────────────────────

/// ENSO 相位。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EnsoPhase {
    #[serde(rename = "El Niño")]
    ElNino,
    #[serde(rename = "La Niña")]
    LaNina,
    Neutral,
}

/// ENSO 诊断结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnsoIndex {
    /// Niño 3.4 SST 异常 (°C)
    pub nino34_anomaly_c: f64,
    /// ENSO 相位
    pub phase: EnsoPhase,
    /// Oceanic Niño Index: 3 月滑动平均
    pub oni: f64,
    /// SOI 估计值
    pub soi_estimate: f64,
}

/// 分类 ENSO 相位。
/// ONI ≥ +0.5°C 持续 5 个月 → El Niño
/// ONI ≤ -0.5°C 持续 5 个月 → La Niña
pub fn enso_phase(nino34_monthly: &[f64]) -> Vec<EnsoPhase> {
    let oni = oni_index(nino34_monthly);
    let n = oni.len();
    let mut phases = Vec::with_capacity(n);
    let mut el_count = 0usize;
    let mut la_count = 0usize;

    for &oni_val in &oni {
        if oni_val >= 0.5 {
            el_count += 1;
            la_count = 0;
        } else if oni_val <= -0.5 {
            la_count += 1;
            el_count = 0;
        } else {
            el_count = 0;
            la_count = 0;
        }

        if el_count >= 5 {
            phases.push(EnsoPhase::ElNino);
        } else if la_count >= 5 {
            phases.push(EnsoPhase::LaNina);
        } else {
            phases.push(EnsoPhase::Neutral);
        }
    }
    phases
}

/// ONI: 3 月滑动平均。
pub fn oni_index(nino34_monthly: &[f64]) -> Vec<f64> {
    if nino34_monthly.is_empty() {
        return Vec::new();
    }
    let n = nino34_monthly.len();
    let mut result = Vec::with_capacity(n);
    // Padding: first 2 months use available data
    for i in 0..n {
        let start = i.saturating_sub(2);
        let count = i - start + 1;
        let sum: f64 = nino34_monthly[start..=i].iter().sum();
        result.push(sum / count as f64);
    }
    result
}

/// 从 Niño 3.4 估计 SOI。
/// SOI ≈ -2.0 × Niño3.4 异常（近似回归）
pub fn soi_estimate(nino34_anomaly: f64) -> f64 {
    -2.0 * nino34_anomaly
}

/// 完整 ENSO 诊断。
pub fn enso_diagnosis(nino34_monthly: &[f64]) -> Vec<EnsoIndex> {
    let oni = oni_index(nino34_monthly);
    let phases = enso_phase(nino34_monthly);
    nino34_monthly
        .iter()
        .zip(oni.iter().zip(phases.iter()))
        .map(|(&anom, (&o, p))| EnsoIndex {
            nino34_anomaly_c: anom,
            phase: p.clone(),
            oni: o,
            soi_estimate: soi_estimate(anom),
        })
        .collect()
}

// ──────────────────────────────────────────────
// 4. 简化 SWAN 波浪传播
// ──────────────────────────────────────────────

/// SWAN 波浪参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwanParams {
    /// 外海有效波高 (m)
    pub hs_offshore: f64,
    /// 外海谱峰周期 (s)
    pub tp_offshore: f64,
    /// 外海波向 (°)
    pub wave_direction_deg: f64,
    /// 底坡 (m/m)
    pub bottom_slope: f64,
}

/// SWAN 波浪传播结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwanResult {
    /// 近岸有效波高 (m)
    pub hs_shore: f64,
    /// 近岸谱峰周期 (s)
    pub tp_shore: f64,
    /// 波浪增水 (m)
    pub setup_m: f64,
    /// 破波波高 (m)
    pub break_height_m: f64,
    /// 破波水深 (m)
    pub break_depth_m: f64,
    /// 破波指数 H/d
    pub break_index: f64,
    /// 折射系数 Kr = √(cosθ₀/cosθ₁)
    pub refraction_coeff: f64,
    /// 浅化系数 Ks = √(Cg₀/Cg₁)
    pub shoaling_coeff: f64,
}

/// 简化 1D 波浪传播模型。
///
/// 1. 深水波: L0 = g·T²/(2π), C0 = L0/T, Cg0 = 0.5·C0
/// 2. 破波: Hb = 0.39·g^(0.2)·(H0²·C0)^(0.4) (Komar-Gaughan)
/// 3. Snell 定律折射: Kr = √(cosθ₀/cosθ₁)
/// 4. 浅化: Cg 从深水到浅水的变化
/// 5. 增水: η = 0.25·Hb
pub fn swan_wave_transformation(params: &SwanParams, depth_shore_m: f64) -> SwanResult {
    let g = 9.81;

    // 深水波参数
    let l0 = g * params.tp_offshore.powi(2) / (2.0 * std::f64::consts::PI);
    let c0 = l0 / params.tp_offshore;
    let cg0 = c0 * 0.5; // deep water group velocity

    // 破波 (Komar-Gaughan, 1972)
    // Hb = 0.56 * H0 * (H0 / L0)^(-1/5) = 0.56 * H0^0.8 * L0^0.2
    let hb = 0.56 * params.hs_offshore * (params.hs_offshore / l0).powf(-0.2);
    let hb = hb.min(params.hs_offshore * 1.5); // cap

    // 破波水深 (Hb = γ * db, γ≈0.78)
    let gamma = 0.78;
    let break_depth = hb / gamma;

    // 折射
    let _theta0 = params.wave_direction_deg.to_radians();

    // 浅水参数 (近似到破波点)
    let depth_effective = depth_shore_m.max(0.5);

    // 浅化: 波速近似
    let _kh = if depth_effective > l0 {
        // 深水
        3.0 // phase is effectively deep
    } else {
        0.5 // very shallow approximation
    };
    let c = if depth_effective >= l0 / 2.0 {
        c0
    } else {
        (g * depth_effective).sqrt()
    };
    let cg = if depth_effective >= l0 / 2.0 {
        cg0
    } else {
        let kh = 2.0 * std::f64::consts::PI * depth_effective / l0;
        let n = 0.5 * (1.0 + 2.0 * kh / (2.0 * kh).sinh());
        n * c
    };

    let kr = 1.0; // 1D shore-normal: Kr≈1
    let ks = (cg0 / cg.max(0.01)).sqrt().min(2.0);

    // 近岸波高
    let hs_shore = params.hs_offshore * kr * ks;
    // 保守校验: 不超过破波高
    let hs_shore = hs_shore.min(hb * 0.9);

    // 波浪增水
    let setup_m = 0.25 * hb;

    SwanResult {
        hs_shore,
        tp_shore: params.tp_offshore, // 周期守恒
        setup_m,
        break_height_m: hb,
        break_depth_m: break_depth,
        break_index: gamma,
        refraction_coeff: kr,
        shoaling_coeff: ks,
    }
}

/// 波能通量: P = ρg·H²·Cg/8 (W/m 波峰线)
pub fn wave_energy_flux(hs_m: f64, tp_s: f64, depth_m: f64) -> f64 {
    let g = 9.81;
    let rho = 1025.0;
    let l0 = g * tp_s.powi(2) / (2.0 * std::f64::consts::PI);
    let c0 = l0 / tp_s;

    let c = if depth_m >= l0 / 2.0 {
        c0
    } else {
        (g * depth_m).sqrt()
    };
    let kh = 2.0 * std::f64::consts::PI * depth_m / l0.max(0.1);
    let n = if kh > 0.01 {
        0.5 * (1.0 + 2.0 * kh / (2.0 * kh).sinh())
    } else {
        1.0
    };
    let cg = n * c;

    rho * g * hs_m.powi(2) * cg / 8.0
}

// ──────────────────────────────────────────────
// 测试
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_predict_tide_m2_only() {
        let c = vec![TidalConstituent {
            name: "M2".into(),
            speed_deg_hr: 28.984104,
            amplitude_m: 1.0,
            phase_deg: 0.0,
        }];
        let pred = predict_tide(&c, 1.0, 25, 0.0);
        assert_eq!(pred.predictions.len(), 25);
        // At t=0: cos(0)=1 → 1.0
        assert!((pred.predictions[0].1 - 1.0).abs() < 1e-6);
        // At t=6.2h (~M2 half period): cos(28.98*6.2°) ≈ cos(π) ≈ -1.0
        assert!(pred.predictions[6].1 < -0.8);
    }

    #[test]
    fn test_standard_constituents() {
        let c = standard_constituents(0.5, 45.0, 0.3, 60.0, 0.2, 30.0, 0.15, 20.0);
        assert_eq!(c.len(), 4);
        assert_eq!(c[0].name, "M2");
        assert!((c[0].amplitude_m - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_spring_neap() {
        let c = standard_constituents(1.0, 0.0, 0.5, 0.0, 0.2, 0.0, 0.1, 0.0);
        let (spring, neap) = spring_neap_heights(&c);
        assert!((spring - 1.5).abs() < 1e-6);
        assert!((neap - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_tidal_range_stats() {
        let p = vec![(0.0, -1.0), (3.0, 0.0), (6.0, 1.0)];
        let (min, max, range) = tidal_range_stats(&p);
        assert!((min - (-1.0)).abs() < 1e-6);
        assert!((max - 1.0).abs() < 1e-6);
        assert!((range - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_ekman_transport_basic() {
        let lat = 30.0;
        let ek = ekman_transport(10.0, 0.0, lat); // wind blowing north
        assert!(ek.transport_m3_s_m > 0.0);
        assert!(ek.ekman_depth_m > 0.0 && ek.ekman_depth_m < 100.0);
        // In NH, Ekman transport is 90° to the right of wind
        // Wind blowing north → transport to the east
        assert!(ek.u_transport > 0.0);
        assert!(ek.ekman_depth_m > 0.0);
    }

    #[test]
    fn test_ekman_equator_near() {
        let ek = ekman_transport(5.0, 90.0, 0.1); // near equator
        assert!(ek.transport_m3_s_m > 0.0); // should still produce positive transport
    }

    #[test]
    fn test_geostrophic_current() {
        let (u, v) = geostrophic_current(1e-5, 1e-5, 30.0);
        assert!(u.is_finite());
        assert!(v.is_finite());
        assert!(u.abs() > 0.0 || v.abs() > 0.0);
    }

    #[test]
    fn test_total_surface_current() {
        let (u, v, speed) = total_surface_current(10.0, 0.0, 1e-5, 1e-5, 30.0);
        assert!(speed > 0.0);
        assert!(u.is_finite() && v.is_finite());
    }

    #[test]
    fn test_enso_phase_neutral() {
        let data = vec![0.0, 0.0, 0.0, 0.0, 0.0];
        let phases = enso_phase(&data);
        for p in phases {
            assert_eq!(p, EnsoPhase::Neutral);
        }
    }

    #[test]
    fn test_enso_phase_el_nino() {
        let data = vec![0.8, 0.9, 0.7, 0.8, 1.0];
        let phases = enso_phase(&data);
        assert_eq!(phases[4], EnsoPhase::ElNino);
    }

    #[test]
    fn test_oni_index() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let oni = oni_index(&data);
        assert!((oni[2] - 2.0).abs() < 1e-6); // (1+2+3)/3 = 2.0
        assert!((oni[3] - 3.0).abs() < 1e-6); // (2+3+4)/3 = 3.0
    }

    #[test]
    fn test_soi_estimate() {
        let soi = soi_estimate(1.0);
        assert!((soi - (-2.0)).abs() < 1e-6);
    }

    #[test]
    fn test_enso_diagnosis() {
        let data = vec![0.5, 0.6, 0.7];
        let diag = enso_diagnosis(&data);
        assert_eq!(diag.len(), 3);
        assert_eq!(diag[0].phase, EnsoPhase::Neutral);
        assert!((diag[0].soi_estimate - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_swan_wave_transformation() {
        let params = SwanParams {
            hs_offshore: 2.0,
            tp_offshore: 10.0,
            wave_direction_deg: 0.0,
            bottom_slope: 0.02,
        };
        let result = swan_wave_transformation(&params, 5.0);
        assert!(result.hs_shore > 0.0 && result.hs_shore <= result.break_height_m * 0.9 + 0.01);
        assert!(result.setup_m > 0.0);
        assert!(result.break_height_m > 0.0);
        assert!(result.break_depth_m > 0.0);
    }

    #[test]
    fn test_wave_energy_flux() {
        let p = wave_energy_flux(2.0, 10.0, 50.0);
        assert!(p > 10000.0); // ~40 kW/m = 40000 W/m
        assert!(p < 500000.0); // reasonable bound
    }

    #[test]
    fn test_serde_roundtrip() {
        let ek = ekman_transport(10.0, 45.0, 30.0);
        let json = serde_json::to_string(&ek).unwrap();
        let ek2: EkmanResult = serde_json::from_str(&json).unwrap();
        assert!((ek.transport_m3_s_m - ek2.transport_m3_s_m).abs() < 1e-6);
    }
}
