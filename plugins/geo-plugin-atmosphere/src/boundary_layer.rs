use serde::{Deserialize, Serialize};

/// Pasquill-Gifford 大气稳定度分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StabilityClass {
    A,
    B,
    C,
    D,
    E,
    F,
}

impl StabilityClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::A => "A (very unstable)",
            Self::B => "B (unstable)",
            Self::C => "C (slightly unstable)",
            Self::D => "D (neutral)",
            Self::E => "E (slightly stable)",
            Self::F => "F (stable)",
        }
    }

    /// 从字符标记解析。
    pub fn from_char(c: char) -> Option<Self> {
        match c.to_ascii_uppercase() {
            'A' => Some(Self::A),
            'B' => Some(Self::B),
            'C' => Some(Self::C),
            'D' => Some(Self::D),
            'E' => Some(Self::E),
            'F' => Some(Self::F),
            _ => None,
        }
    }
}

/// 边界层输出。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryLayerResult {
    pub abl_height_m: f64,
    pub shf_w_m2: f64,
    pub lhf_w_m2: f64,
    pub monin_obukhov_m: f64,
    pub mixing_height_m: f64,
    pub stability: String,
    pub u_star_m_s: f64,
}

/// 估算大气边界层高度 (m)。
///
/// 基于 Pasquill-Gifford 稳定度 + 风速/粗糙度修正。
pub fn atmospheric_boundary_layer_height(
    wind_speed_m_s: f64,
    roughness_m: f64,
    stability: StabilityClass,
) -> f64 {
    let base = match stability {
        StabilityClass::A => 1400.0,
        StabilityClass::B => 1200.0,
        StabilityClass::C => 900.0,
        StabilityClass::D => 750.0,
        StabilityClass::E => 350.0,
        StabilityClass::F => 150.0,
    };

    // 风速修正
    let ws_factor = match stability {
        StabilityClass::A | StabilityClass::B | StabilityClass::C | StabilityClass::D => {
            1.0 + 0.1 * (wind_speed_m_s - 3.0).max(0.0) / wind_speed_m_s.max(1.0)
        }
        _ => 1.0,
    };

    // 粗糙度修正
    let z0_factor = 1.0 + 0.2 * (roughness_m / 0.1).min(1.0).max(0.0);

    (base * ws_factor * z0_factor).clamp(50.0, 2500.0)
}

/// 估算摩擦速度 u* (m/s)。
pub fn friction_velocity(wind_speed_m_s: f64, roughness_m: f64, stability: StabilityClass) -> f64 {
    let karman = 0.4;
    let z_ref = 10.0; // 参考高度 10m
    let psi_m = match stability {
        StabilityClass::A | StabilityClass::B => -2.0, // 不稳定修正
        StabilityClass::C | StabilityClass::D => 0.0,
        StabilityClass::E | StabilityClass::F => 2.0,
    };
    let ln_term = (z_ref / roughness_m.max(0.001)).ln();
    (karman * wind_speed_m_s) / (ln_term - psi_m).max(0.1)
}

/// 估算感热通量 SHF (W/m²) 和潜热通量 LHF (W/m²)。
///
/// temp_profile: [T_surface, T_2m, T_10m, T_50m] (°C)
/// wind_profile: [u_2m, u_10m, u_50m] (m/s)
pub fn turbulent_heat_fluxes(temp_profile: &[f64], wind_profile: &[f64]) -> (f64, f64) {
    if temp_profile.len() < 2 || wind_profile.is_empty() {
        return (0.0, 0.0);
    }

    let ts = temp_profile[0];
    let t2 = temp_profile.get(1).copied().unwrap_or(ts);
    let u10 = wind_profile.get(1).copied().unwrap_or(3.0);
    let u2 = wind_profile.first().copied().unwrap_or(u10 * 0.75);

    let rho = 1.225 * (288.15 / (t2 + 273.15));
    let cp = 1005.0;
    let lv = 2.5e6;
    let ch = 0.0011;
    let ce = 0.0012;

    let dtemp = ts - t2;
    let shf = rho * cp * ch * u10 * dtemp;

    // 简化湿度估算
    let es_surf = 611.0 * ((17.27 * ts) / (ts + 237.3)).exp();
    let es_2m = 611.0 * ((17.27 * t2) / (t2 + 237.3)).exp();
    let q_surf = 0.622 * (0.70 * es_surf) / (101325.0 - 0.378 * 0.70 * es_surf);
    let q_2m = 0.622 * (0.50 * es_2m) / (101325.0 - 0.378 * 0.50 * es_2m);
    let dq = (q_surf - q_2m).max(0.0);

    let lhf = rho * lv * ce * u2 * dq;

    (shf, lhf)
}

/// Monin-Obukhov 长度 (m)。
pub fn monin_obukhov_length(u_star: f64, shf: f64, t_mean_k: f64) -> f64 {
    let karman = 0.4;
    let g = 9.81;
    let rho_cp = 1.225 * 1005.0;

    if shf.abs() < 1e-6 {
        return 1e6; // 近中性 → 大值
    }

    -(u_star.powi(3) * rho_cp * t_mean_k) / (karman * g * shf)
}

/// 机械混合层高度 (m)。
pub fn mixing_height(u_star: f64, coriolis_param: f64) -> f64 {
    if coriolis_param <= 0.0 {
        return 300.0;
    }
    (0.3 * u_star) / coriolis_param.max(1e-6)
}

/// 从整体理查森数 Ri_b 推断稳定度。
pub fn stability_from_richardson(ri_bulk: f64) -> StabilityClass {
    if ri_bulk < -0.4 {
        StabilityClass::A
    } else if ri_bulk < -0.2 {
        StabilityClass::B
    } else if ri_bulk < -0.05 {
        StabilityClass::C
    } else if ri_bulk < 0.05 {
        StabilityClass::D
    } else if ri_bulk < 0.2 {
        StabilityClass::E
    } else {
        StabilityClass::F
    }
}

/// 计算整体理查森数。
pub fn bulk_richardson(
    temp_c: &[f64], // [T_surface, T_z]
    height_m: f64,  // z
    wind_speed_m_s: f64,
) -> f64 {
    if temp_c.len() < 2 || height_m <= 0.0 || wind_speed_m_s <= 0.0 {
        return 0.0;
    }
    let g = 9.81;
    let t_mean = (temp_c[0] + temp_c[1]) / 2.0 + 273.15;
    let dtheta = temp_c[1] - temp_c[0];
    (g / t_mean) * dtheta * height_m / (wind_speed_m_s.powi(2) + 0.01)
}

/// 完整边界层评估。
pub fn boundary_layer_assessment(
    temp_profile: &[f64],
    wind_profile: &[f64],
    roughness_m: f64,
    coriolis_param: f64,
) -> BoundaryLayerResult {
    let u10 = wind_profile.get(1).copied().unwrap_or(3.0);
    let t2 = temp_profile.get(1).copied().unwrap_or(15.0);
    let t_mean_k = t2 + 273.15;

    let ri = bulk_richardson(temp_profile, 10.0, u10);
    let stability = stability_from_richardson(ri);

    let abl = atmospheric_boundary_layer_height(u10, roughness_m, stability);
    let (shf, lhf) = turbulent_heat_fluxes(temp_profile, wind_profile);
    let u_star = friction_velocity(u10, roughness_m, stability);
    let mol = monin_obukhov_length(u_star, shf, t_mean_k);
    let mix_h = mixing_height(u_star, coriolis_param);

    BoundaryLayerResult {
        abl_height_m: abl,
        shf_w_m2: shf,
        lhf_w_m2: lhf,
        monin_obukhov_m: mol,
        mixing_height_m: mix_h,
        stability: stability.as_str().to_string(),
        u_star_m_s: u_star,
    }
}

// ═══════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abl_height_neutral() {
        let h = atmospheric_boundary_layer_height(5.0, 0.1, StabilityClass::D);
        assert!((h - 750.0).abs() < 100.0, "abl={}", h);
    }

    #[test]
    fn test_abl_height_stable() {
        let h = atmospheric_boundary_layer_height(2.0, 0.01, StabilityClass::F);
        assert!(h <= 200.0, "stable abl should be low, got {}", h);
    }

    #[test]
    fn test_abl_height_unstable_with_wind() {
        let h = atmospheric_boundary_layer_height(8.0, 0.5, StabilityClass::B);
        assert!(h > 1000.0, "unstable+wind abl should be high, got {}", h);
    }

    #[test]
    fn test_heat_fluxes_no_profile() {
        let (shf, lhf) = turbulent_heat_fluxes(&[], &[]);
        assert_eq!(shf, 0.0);
        assert_eq!(lhf, 0.0);
    }

    #[test]
    fn test_heat_fluxes_daytime() {
        // 夏季白天：地表 35°C → 2m 30°C
        let (shf, lhf) = turbulent_heat_fluxes(&[35.0, 30.0, 28.0], &[2.0, 4.0, 5.0]);
        assert!(shf > 0.0, "shf should be positive (unstable), got {}", shf);
        assert!(lhf >= 0.0, "lhf should be >= 0, got {}", lhf);
    }

    #[test]
    fn test_heat_fluxes_nighttime() {
        // 夜间逆温：地表 5°C → 2m 10°C
        let (shf, lhf) = turbulent_heat_fluxes(&[5.0, 10.0, 8.0], &[1.0, 2.0, 3.0]);
        assert!(shf < 0.0, "shf should be negative (stable), got {}", shf);
    }

    #[test]
    fn test_friction_velocity() {
        let u_star = friction_velocity(5.0, 0.1, StabilityClass::D);
        assert!(u_star > 0.1 && u_star < 1.5, "u_star={}", u_star);
    }

    #[test]
    fn test_monin_obukhov_unstable() {
        let mol = monin_obukhov_length(0.3, 100.0, 300.0);
        assert!(mol < 0.0, "unstable -> mol<0, got {}", mol);
    }

    #[test]
    fn test_monin_obukhov_stable() {
        let mol = monin_obukhov_length(0.3, -50.0, 300.0);
        assert!(mol > 0.0, "stable -> mol>0, got {}", mol);
    }

    #[test]
    fn test_monin_obukhov_neutral() {
        let mol = monin_obukhov_length(0.3, 0.0, 300.0);
        assert!(mol.abs() > 1e5, "neutral -> large mol, got {}", mol);
    }

    #[test]
    fn test_mixing_height() {
        let h = mixing_height(0.5, 1.0e-4);
        assert!((h - 1500.0).abs() < 10.0, "mix_h={}", h);
    }

    #[test]
    fn test_stability_from_ri() {
        assert_eq!(stability_from_richardson(-0.5), StabilityClass::A);
        assert_eq!(stability_from_richardson(-0.1), StabilityClass::C);
        assert_eq!(stability_from_richardson(0.0), StabilityClass::D);
        assert_eq!(stability_from_richardson(0.3), StabilityClass::F);
    }

    #[test]
    fn test_bulk_richardson() {
        let ri = bulk_richardson(&[25.0, 30.0], 10.0, 3.0);
        assert!(ri > 0.0, "stable profile -> ri>0, got {}", ri);
    }

    #[test]
    fn test_full_assessment() {
        let result = boundary_layer_assessment(&[35.0, 30.0, 27.0], &[2.5, 4.0, 5.5], 0.1, 1.0e-4);
        assert!(result.abl_height_m > 500.0);
        assert!(result.shf_w_m2 > 0.0);
        assert!(result.u_star_m_s > 0.1);
        assert!(result.mixing_height_m > 100.0);
        assert!(result.abl_height_m <= 2500.0);
    }

    #[test]
    fn test_stability_class_from_char() {
        assert_eq!(StabilityClass::from_char('A'), Some(StabilityClass::A));
        assert_eq!(StabilityClass::from_char('a'), Some(StabilityClass::A));
        assert_eq!(StabilityClass::from_char('D'), Some(StabilityClass::D));
        assert_eq!(StabilityClass::from_char('F'), Some(StabilityClass::F));
        assert_eq!(StabilityClass::from_char('X'), None);
    }
}
