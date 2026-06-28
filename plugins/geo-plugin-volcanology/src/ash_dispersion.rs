use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// 火山灰扩散结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AshDispersionResult {
    /// 下风方向浓度剖面 [μg/m³]
    pub centerline_concentration_ugm3: Vec<f64>,
    /// 下风距离 (km)
    pub downwind_distances_km: Vec<f64>,
    /// 沉降通量 [g/m²]
    pub deposition_flux_gm2: Vec<f64>,
    /// 羽流高度 (m)
    pub plume_height_m: f64,
    /// 总排放量 (kg)
    pub total_emission_kg: f64,
    /// 最大地面浓度 (μg/m³)
    pub max_ground_concentration_ugm3: f64,
    /// 最大浓度距离 (km)
    pub max_concentration_distance_km: f64,
}

/// 颗粒沉降速度 (Stokes 定律)。
#[allow(non_snake_case)]
pub fn settling_velocity(
    particle_diameter_m: f64,
    particle_density_kgm3: f64,
    air_density_kgm3: f64,
    air_viscosity_Pa_s: f64,
) -> f64 {
    let g = 9.81;
    if air_viscosity_Pa_s <= 0.0 {
        return 0.0;
    }
    (particle_diameter_m.powi(2) * (particle_density_kgm3 - air_density_kgm3) * g)
        / (18.0 * air_viscosity_Pa_s)
}

/// 下风方向指定点的火山灰浓度 (高斯烟羽变体 + 沉降)。
/// 类似 atmospheric dispersion 但沿高度有源分布。
pub fn plume_concentration(
    x_m: f64,
    y_m: f64,
    z_m: f64,
    emission_rate_kg_s: f64,
    wind_speed_m_s: f64,
    plume_height_m: f64,
    settling_vel_m_s: f64,
    stability_class: &str,
) -> f64 {
    if wind_speed_m_s <= 0.0 || emission_rate_kg_s <= 0.0 {
        return 0.0;
    }

    let sigma_y = briggs_sigma_y(x_m, stability_class);
    let sigma_z = briggs_sigma_z(x_m, stability_class);

    if sigma_y <= 0.0 || sigma_z <= 0.0 {
        return 0.0;
    }

    // 有效排放高度受沉降影响降低
    let he = (plume_height_m - settling_vel_m_s * x_m / wind_speed_m_s).max(1.0);

    let exp_y = (-y_m.powi(2) / (2.0 * sigma_y.powi(2))).exp();
    let term1 = (-(z_m - he).powi(2) / (2.0 * sigma_z.powi(2))).exp();
    let term2 = (-(z_m + he).powi(2) / (2.0 * sigma_z.powi(2))).exp();

    let c = emission_rate_kg_s * 1.0e9 // kg/s → μg/s
        / (2.0 * PI * wind_speed_m_s * sigma_y * sigma_z)
        * exp_y
        * (term1 + term2);

    c.max(0.0)
}

/// Briggs 横向扩散参数 σy (m)。
fn briggs_sigma_y(x_m: f64, stability: &str) -> f64 {
    let x_km = x_m / 1000.0;
    if x_km <= 0.0 {
        return 0.1;
    }
    match stability {
        "A" => 0.22 * x_km / (1.0 + 0.0001 * x_km).sqrt(),
        "B" => 0.16 * x_km / (1.0 + 0.0001 * x_km).sqrt(),
        "C" => 0.11 * x_km / (1.0 + 0.0001 * x_km).sqrt(),
        "D" => 0.08 * x_km / (1.0 + 0.0001 * x_km).sqrt(),
        "E" => 0.06 * x_km / (1.0 + 0.0001 * x_km).sqrt(),
        "F" => 0.04 * x_km / (1.0 + 0.0001 * x_km).sqrt(),
        _ => 0.08 * x_km / (1.0 + 0.0001 * x_km).sqrt(),
    }
}

/// Briggs 垂直扩散参数 σz (m)。
fn briggs_sigma_z(x_m: f64, stability: &str) -> f64 {
    let x_km = x_m / 1000.0;
    if x_km <= 0.0 {
        return 0.1;
    }
    match stability {
        "A" | "B" => 0.20 * x_km,
        "C" => 0.14 * x_km / (1.0 + 0.0003 * x_km).sqrt(),
        "D" => 0.08 * x_km / (1.0 + 0.0015 * x_km).sqrt(),
        "E" => 0.06 * x_km / (1.0 + 0.0015 * x_km).sqrt(),
        "F" => 0.03 * x_km / (1.0 + 0.0003 * x_km).sqrt(),
        _ => 0.08 * x_km / (1.0 + 0.0015 * x_km).sqrt(),
    }
}

/// 完整火山灰扩散评估。
pub fn ash_dispersion_assessment(
    emission_rate_kg_s: f64,
    wind_speed_m_s: f64,
    plume_height_m: f64,
    particle_diameter_m: f64,
    particle_density_kgm3: f64,
    stability: &str,
    n_points: usize,
) -> AshDispersionResult {
    let air_density = 1.225;
    let air_viscosity = 1.8e-5;
    let vs = settling_velocity(
        particle_diameter_m,
        particle_density_kgm3,
        air_density,
        air_viscosity,
    );
    let total_emission = emission_rate_kg_s * 3600.0; // 1 hour

    let max_dist = 50_000.0; // 50 km
    let distances: Vec<f64> = (0..n_points)
        .map(|i| (i as f64 + 1.0) * max_dist / n_points as f64)
        .collect();

    let mut concentrations = Vec::new();
    let mut fluxes = Vec::new();
    let mut max_c = 0.0_f64;
    let mut max_d = 0.0_f64;

    for &x in &distances {
        let c = plume_concentration(
            x,
            0.0,
            0.0,
            emission_rate_kg_s,
            wind_speed_m_s,
            plume_height_m,
            vs,
            stability,
        );
        let flux = c * vs / 1.0e9 * 1000.0; // μg/m³ * m/s → g/m²
        concentrations.push(c);
        fluxes.push(flux);
        if c > max_c {
            max_c = c;
            max_d = x / 1000.0;
        }
    }

    AshDispersionResult {
        centerline_concentration_ugm3: concentrations,
        downwind_distances_km: distances.iter().map(|&d| d / 1000.0).collect(),
        deposition_flux_gm2: fluxes,
        plume_height_m,
        total_emission_kg: total_emission,
        max_ground_concentration_ugm3: (max_c * 100.0).round() / 100.0,
        max_concentration_distance_km: (max_d * 100.0).round() / 100.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settling_velocity() {
        // 1 mm 颗粒, 石英密度 ~2650
        let v = settling_velocity(0.001, 2650.0, 1.225, 1.8e-5);
        assert!(v > 0.0);
        assert!(v < 100.0); // 合理范围
    }

    #[test]
    fn test_plume_concentration() {
        let c = plume_concentration(5000.0, 0.0, 0.0, 1000.0, 10.0, 5000.0, 0.5, "D");
        assert!(c >= 0.0);
    }

    #[test]
    fn test_plume_zero_wind() {
        let c = plume_concentration(1000.0, 0.0, 0.0, 1000.0, 0.0, 5000.0, 0.5, "D");
        assert!((c - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_ash_dispersion() {
        let r = ash_dispersion_assessment(1000.0, 10.0, 5000.0, 0.001, 2500.0, "D", 20);
        assert_eq!(r.centerline_concentration_ugm3.len(), 20);
        assert!(r.max_ground_concentration_ugm3 > 0.0);
    }

    #[test]
    fn test_briggs_sigma() {
        let sy = briggs_sigma_y(1000.0, "D");
        let sz = briggs_sigma_z(1000.0, "D");
        assert!(sy > 0.0);
        assert!(sz > 0.0);
    }

    #[test]
    fn test_small_diameter_settling() {
        // 10 μm — 几乎不沉降
        let v = settling_velocity(1e-5, 2500.0, 1.225, 1.8e-5);
        assert!(v < 0.01);
    }
}
