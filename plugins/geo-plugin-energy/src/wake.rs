/// 风力机尾流效应模型。
///
/// Jensen (1983) 尾流模型: 下游风速衰减，适用于风电场布局优化。
///
/// Jensen 公式:
///   V(x) = V₀ × [1 - (1 - √(1-CT)) × (R / (R + k×x))²]
///
/// 其中:
///   V(x) = 下游距离 x 处的风速 (m/s)
///   V₀   = 自由流风速 (m/s)
///   CT   = 推力系数
///   R    = 风轮半径 (m)
///   k    = 尾流衰减常数 (海上 ~0.04, 陆上 ~0.075)
///   x    = 下游距离 (m)
///
/// # 参考文献
/// Jensen, N.O. (1983). A note on wind generator interaction.
///   Risø-M-2411, Risø National Laboratory.
/// Katic, I., Højstrup, J., & Jensen, N.O. (1986). A simple model for
///   cluster efficiency. EWEC '86, 407-410.
/// Stevens, R.J.A.M., & Meneveau, C. (2014). Flow structure and
///   turbulence in wind farms. Annual Review of Fluid Mechanics, 46, 409-429.

use serde::{Deserialize, Serialize};

/// 尾流衰减常数 — 陆上 (Jensen 推荐)
pub const K_ONSIGHT: f64 = 0.075;

/// 尾流衰减常数 — 海上
pub const K_OFFSHORE: f64 = 0.04;

// ─── 单机尾流 ───

/// 单台风力机尾流风速 (Jensen 模型)。
///
/// # 参数
/// * `v0` — 自由流风速 (m/s)
/// * `ct` — 推力系数 (≈ 0.7-0.9)
/// * `rotor_radius_m` — 风轮半径 (m)
/// * `distance_m` — 下游距离 (m)
/// * `k` — 尾流衰减常数
pub fn jensen_wake(
    v0: f64,
    ct: f64,
    rotor_radius_m: f64,
    distance_m: f64,
    k: f64,
) -> f64 {
    let deficit = 1.0 - (1.0 - ct).sqrt();
    let wake_radius = rotor_radius_m + k * distance_m;
    let expansion_ratio = (rotor_radius_m / wake_radius).powi(2);
    let velocity_loss = deficit * expansion_ratio;
    v0 * (1.0 - velocity_loss).max(0.0)
}

/// 尾流半径 (Jensen 线性膨胀)。
pub fn wake_radius(rotor_radius_m: f64, distance_m: f64, k: f64) -> f64 {
    rotor_radius_m + k * distance_m
}

/// 尾流横截面风速分布（归一化）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeProfile {
    /// 下游距离 (m)
    pub distance_m: f64,
    /// 当前尾流半径 (m)
    pub radius_m: f64,
    /// 尾流中心线风速 (m/s)
    pub center_wind_ms: f64,
    /// 中心线衰减比 V/V₀
    pub deficit_ratio: f64,
}

/// 单机尾流完整剖面。
pub fn wake_profile(
    v0: f64,
    ct: f64,
    rotor_radius_m: f64,
    distance_m: f64,
    k: f64,
) -> WakeProfile {
    let v_center = jensen_wake(v0, ct, rotor_radius_m, distance_m, k);
    WakeProfile {
        distance_m,
        radius_m: wake_radius(rotor_radius_m, distance_m, k),
        center_wind_ms: v_center,
        deficit_ratio: if v0 > 0.0 { v_center / v0 } else { 0.0 },
    }
}

// ─── 尾流叠加 ───

/// 尾流叠加方法。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WakeSummation {
    /// 线性叠加: V = V₀ - Σ(V₀ - Vᵢ) — 可能过估
    Linear,
    /// 能量守恒: V³ = V₀³ - Σ(V₀³ - Vᵢ³) — 推荐
    Energy,
    /// 平方和: V² = V₀² - Σ(V₀² - Vᵢ²)
    Square,
}

/// 多台风力机尾流叠加：计算阵列中指定风机的实际来流风速。
///
/// # 参数
/// * `wind_speed` — 自由流风速 (m/s)
/// * `wind_deg` — 风向 (°, 0=北, 90=东)
/// * `turbines` — 所有风机坐标 (x, y, ct, radius_m)
/// * `target_idx` — 目标风机在数组中的索引
/// * `k` — 尾流衰减常数
/// * `method` — 叠加方法
/// * `spacing_m` — 标准间距，仅计算此距离内的尾流影响
pub fn cumulative_wake(
    wind_speed: f64,
    _wind_deg: f64,
    turbines: &[(f64, f64, f64, f64)], // (x, y, ct, radius)
    target_idx: usize,
    k: f64,
    method: &WakeSummation,
    spacing_m: f64,
) -> f64 {
    if target_idx >= turbines.len() {
        return wind_speed;
    }
    let (tx, ty, _tct, _tr) = turbines[target_idx];

    let mut deficit = Vec::new();

    for (i, &(sx, sy, sct, sr)) in turbines.iter().enumerate() {
        if i == target_idx {
            continue;
        }
        let dx = tx - sx;
        let dy = ty - sy;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist <= 0.0 || dist > spacing_m {
            continue;
        }

        // 下游判断（简化）：上游风机在风向逆方向
        // 如果上游风机到目标的方向与风向夹角 < 60° → 尾流影响
        let angle_rad = dy.atan2(dx);
        let wind_rad = _wind_deg.to_radians();
        let diff = (angle_rad - wind_rad - std::f64::consts::PI).abs();
        let diff = diff.min(2.0 * std::f64::consts::PI - diff);

        if diff < std::f64::consts::FRAC_PI_3 {
            // 60° 尾流扇区
            let v_wake = jensen_wake(wind_speed, sct, sr, dist, k);
            let local_deficit = wind_speed - v_wake;
            deficit.push(local_deficit);
        }
    }

    if deficit.is_empty() {
        return wind_speed;
    }

    match method {
        WakeSummation::Linear => {
            let total_deficit: f64 = deficit.iter().sum();
            (wind_speed - total_deficit).max(0.0)
        }
        WakeSummation::Energy => {
            let v3 = wind_speed.powi(3);
            let sum_deficit3: f64 = deficit.iter().map(|d| v3 - (wind_speed - d).powi(3)).sum();
            (v3 - sum_deficit3).max(0.0).powf(1.0 / 3.0)
        }
        WakeSummation::Square => {
            let v2 = wind_speed.powi(2);
            let sum_deficit2: f64 = deficit.iter().map(|d| v2 - (wind_speed - d).powi(2)).sum();
            (v2 - sum_deficit2).max(0.0).sqrt()
        }
    }
}

// ─── 风电场效率 ───

/// 风电场尾流效率：考虑尾流效应后的整场发电效率。
///
/// # 参数
/// * `turbines` — 风机参数 (x, y, ct, radius_m)
/// * `wind_speed` — 自由流风速 (m/s)
/// * `wind_deg` — 风向 (°)
/// * `k` — 尾流衰减常数
/// * `method` — 叠加方法
/// * `rho` — 空气密度 (kg/m³)
/// * `cp` — 功率系数
/// * `spacing_m` — 计算范围
pub fn farm_wake_efficiency(
    turbines: &[(f64, f64, f64, f64)],
    wind_speed: f64,
    wind_deg: f64,
    k: f64,
    method: &WakeSummation,
    rho: f64,
    cp: f64,
    spacing_m: f64,
) -> (f64, Vec<f64>) {
    let mut turbine_powers = Vec::with_capacity(turbines.len());

    for i in 0..turbines.len() {
        let (_x, _y, _ct, r) = turbines[i];
        let v_local = cumulative_wake(
            wind_speed, wind_deg, turbines, i, k, method, spacing_m,
        );
        let a = std::f64::consts::PI * r * r;
        let p = 0.5 * rho * a * cp * v_local.powi(3);
        turbine_powers.push(p);
    }

    let total_power: f64 = turbine_powers.iter().sum();
    let no_wake_power = turbines.len() as f64 * (0.5 * rho * std::f64::consts::PI * {
        // Use average radius
        let avg_r: f64 = turbines.iter().map(|t| t.3).sum::<f64>() / turbines.len() as f64;
        avg_r * avg_r
    } * cp * wind_speed.powi(3));

    let efficiency = if no_wake_power > 0.0 {
        total_power / no_wake_power
    } else {
        1.0
    };

    (efficiency, turbine_powers)
}

// ─── 多风速玫瑰年发电量 ───

/// 风速玫瑰表: (风速, 概率, 风向)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindRoseEntry {
    pub wind_speed_ms: f64,
    pub probability: f64,
    pub wind_direction_deg: f64,
}

/// 考虑尾流效应的年发电量估算。
pub fn farm_aep_with_wake(
    wind_rose: &[WindRoseEntry],
    turbines: &[(f64, f64, f64, f64)],
    k: f64,
    method: &WakeSummation,
    rho: f64,
    cp: f64,
    spacing_m: f64,
) -> f64 {
    let mut annual_kwh = 0.0;
    for entry in wind_rose {
        let (_efficiency, powers) = farm_wake_efficiency(
            turbines,
            entry.wind_speed_ms,
            entry.wind_direction_deg,
            k,
            method,
            rho,
            cp,
            spacing_m,
        );
        let total_w: f64 = powers.iter().sum();
        let hours = entry.probability * 8760.0;
        annual_kwh += total_w * hours / 1000.0;
    }
    annual_kwh
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_jensen_wake_short_distance() {
        // 近距离尾流：风速衰减明显
        let v = jensen_wake(10.0, 0.8, 40.0, 100.0, K_ONSIGHT);
        assert!(v < 10.0);
        assert!(v > 5.0);
    }

    #[test]
    fn test_jensen_wake_far_distance() {
        // 远距离尾流：风速恢复接近自由流
        let v = jensen_wake(10.0, 0.8, 40.0, 3000.0, K_ONSIGHT);
        assert!(v > 8.0);
    }

    #[test]
    fn test_wake_radius() {
        let r = wake_radius(40.0, 500.0, K_ONSIGHT);
        assert_relative_eq!(r, 40.0 + 0.075 * 500.0, epsilon = 0.01);
    }

    #[test]
    fn test_wake_profile() {
        let p = wake_profile(10.0, 0.8, 40.0, 200.0, K_ONSIGHT);
        assert!(p.center_wind_ms < 10.0);
        assert!(p.deficit_ratio < 1.0);
        assert!(p.radius_m > 40.0);
    }

    #[test]
    fn test_cumulative_wake_single() {
        let turbines = vec![
            (0.0, 0.0, 0.8, 40.0),  // upstream
            (300.0, 0.0, 0.8, 40.0), // downstream target
        ];
        let v = cumulative_wake(10.0, 90.0, &turbines, 1, K_ONSIGHT, &WakeSummation::Energy, 2000.0);
        assert!(v < 10.0, "downstream turbine should see reduced wind, got {}", v);
        assert!(v > 6.0);
    }

    #[test]
    fn test_farm_efficiency() {
        let turbines = vec![
            (0.0, 0.0, 0.8, 40.0),
            (400.0, 0.0, 0.8, 40.0),
            (800.0, 0.0, 0.8, 40.0),
        ];
        let (eff, _powers) = farm_wake_efficiency(
            &turbines, 10.0, 90.0, K_OFFSHORE, &WakeSummation::Energy, 1.225, 0.45, 2000.0,
        );
        // 3机直线阵：效率应在 0.8-0.95 之间
        assert!(eff > 0.7, "efficiency too low: {}", eff);
        assert!(eff <= 1.0, "efficiency > 1.0: {}", eff);
    }

    #[test]
    fn test_no_wake_at_long_distance() {
        let turbines = vec![
            (0.0, 0.0, 0.8, 40.0),
            (5000.0, 0.0, 0.8, 40.0), // far away → no wake
        ];
        let v = cumulative_wake(10.0, 90.0, &turbines, 1, K_ONSIGHT, &WakeSummation::Energy, 2000.0);
        assert_relative_eq!(v, 10.0, epsilon = 0.1);
    }

    #[test]
    fn test_jensen_offshore() {
        let v = jensen_wake(10.0, 0.75, 82.0, 1000.0, K_OFFSHORE);
        // Offshore: slower wake recovery
        assert!(v < 9.5);
        assert!(v > 7.0);
    }
}
