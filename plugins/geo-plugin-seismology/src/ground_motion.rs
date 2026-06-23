//! 地震动参数预测 — GB 18306-2015 衰减关系、反应谱。
use serde::{Deserialize, Serialize};

/// 地震动评估结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundMotionResult {
    /// 峰值加速度 PGA (g)
    pub pga_g: f64,
    /// 峰值速度 PGV (cm/s)
    pub pgv_cm_s: f64,
    /// 地震烈度 (中国 XII 度制)
    pub intensity: u8,
    /// 场地类别
    pub site_class: String,
    /// 震级
    pub magnitude: f64,
    /// 距离 (km)
    pub distance_km: f64,
}

/// 反应谱点。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseSpectrumPoint {
    /// 周期 (s)
    pub period_s: f64,
    /// 谱加速度 Sa (g)
    pub sa_g: f64,
}

/// 反应谱。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseSpectrum {
    pub points: Vec<ResponseSpectrumPoint>,
    pub pga_g: f64,
    pub damping: f64,
}

/// 场地放大因子。
pub fn site_amplification(site_class: &str) -> f64 {
    match site_class {
        "I0" => 0.72,
        "I1" => 0.80,
        "II" => 1.00,
        "III" => 1.35,
        "IV" => 1.80,
        _ => 1.00,
    }
}

/// PGA 衰减关系 (GB 18306-2015 简化版)。
/// 公式: ln(PGA) = a + b*M - c*ln(R + d) + e*S
/// 简化: PGA(g) = a * exp(b*M) / (R + c)^2
pub fn pga_from_mag_distance(magnitude: f64, distance_km: f64, site_class: &str) -> f64 {
    if magnitude <= 0.0 || distance_km < 0.0 {
        return 0.0;
    }
    let amp = site_amplification(site_class);
    let r = distance_km.max(5.0);
    // 简化衰减: ln(PGA) = -2.0 + 0.8*(M-6) - 1.5*ln(R/30)
    let ln_pga = -2.0 + 0.8 * (magnitude - 6.0) - 1.5 * (r / 30.0).ln();
    let pga = ln_pga.exp() * amp;
    pga.min(2.0).max(0.001)
}

/// PGV 从 PGA 估算 (经验关系)。
pub fn pgv_from_pga(pga_g: f64, site_class: &str) -> f64 {
    let amp = site_amplification(site_class);
    // PGV(cm/s) ≈ 120 * PGA(g)^0.6 (纽马克衰减, 中国经验系数)
    120.0 * pga_g.powf(0.6) * amp
}

/// PGA → 中国地震烈度 (GB/T 17742-2020 简化版)。
pub fn pga_to_intensity(pga_g: f64) -> u8 {
    if pga_g >= 0.40 { 12 }
    else if pga_g >= 0.20 { 11 }
    else if pga_g >= 0.10 { 10 }
    else if pga_g >= 0.05 { 9 }
    else if pga_g >= 0.025 { 8 }
    else if pga_g >= 0.01 { 7 }
    else if pga_g >= 0.005 { 6 }
    else { 5 }
}

/// 完整地震动评估。
pub fn ground_motion_assessment(magnitude: f64, distance_km: f64, site_class: &str) -> GroundMotionResult {
    let pga = pga_from_mag_distance(magnitude, distance_km, site_class);
    let pgv = pgv_from_pga(pga, site_class);
    let intensity = pga_to_intensity(pga);
    GroundMotionResult { pga_g: pga, pgv_cm_s: pgv, intensity, site_class: site_class.to_string(), magnitude, distance_km }
}

/// 加速度反应谱 (Newmark-Hall 简化法, GB 50011-2010)。
pub fn response_spectrum(pga_g: f64, periods: &[f64], damping: f64) -> ResponseSpectrum {
    let damping_factor = ((0.05 / damping.max(0.001)).powf(0.4) * 0.55 + 0.45).min(1.5).max(0.5);
    let points: Vec<ResponseSpectrumPoint> = periods.iter().map(|&t| {
        let sa = if t < 0.1 {
            pga_g * (1.0 + (damping_factor * 2.5 - 1.0) * t / 0.1)
        } else if t <= 0.4 {
            pga_g * damping_factor * 2.5
        } else {
            // 下降段: Sa ∝ 1/T
            pga_g * damping_factor * 2.5 * (0.4 / t.max(0.4))
        };
        ResponseSpectrumPoint { period_s: t, sa_g: sa.min(5.0) }
    }).collect();
    ResponseSpectrum { points, pga_g, damping }
}

/// 默认周期数组 (0.02-6.0s, 建筑抗震设计常用周期)。
pub fn default_periods() -> Vec<f64> {
    vec![0.02, 0.05, 0.1, 0.15, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0, 6.0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pga_from_mag_distance() {
        let pga = pga_from_mag_distance(7.0, 30.0, "II");
        assert!(pga > 0.01 && pga < 2.0, "PGA={} should be reasonable for M7@30km II", pga);
    }

    #[test]
    fn test_pga_site_amplification() {
        let pga_ii = pga_from_mag_distance(6.5, 20.0, "II");
        let pga_iv = pga_from_mag_distance(6.5, 20.0, "IV");
        assert!(pga_iv > pga_ii, "IV site should amplify more than II");
    }

    #[test]
    fn test_pga_to_intensity() {
        assert_eq!(pga_to_intensity(0.1), 10);
        assert_eq!(pga_to_intensity(0.01), 7);
        assert_eq!(pga_to_intensity(0.5), 12);
    }

    #[test]
    fn test_pgv_from_pga() {
        let pgv = pgv_from_pga(0.2, "II");
        assert!(pgv > 10.0 && pgv < 100.0, "PGV={} should be reasonable", pgv);
    }

    #[test]
    fn test_response_spectrum() {
        let periods = vec![0.1, 0.4, 1.0, 3.0];
        let rs = response_spectrum(0.3, &periods, 0.05);
        assert_eq!(rs.points.len(), 4);
        assert!(rs.points[1].sa_g >= rs.points[0].sa_g); // plateau >= rise
        assert!(rs.points[3].sa_g <= rs.points[1].sa_g); // decay after plateau
    }

    #[test]
    fn test_ground_motion_assessment() {
        let result = ground_motion_assessment(7.5, 50.0, "III");
        assert!(result.pga_g > 0.02);
        assert!(result.pgv_cm_s > 5.0);
        assert!(result.intensity >= 6);
    }
}
