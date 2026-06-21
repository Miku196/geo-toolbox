/// 风力发电机功率曲线模型。
///
/// 风速 → 功率转换：Cp 系数、Betz 极限、额定/切入/切出风速。
///
/// 公式:
///   P = 0.5 × ρ × A × Cp × V³
///
/// 其中:
///   P  = 功率 (W)
///   ρ  = 空气密度 (kg/m³)，海平面 ≈ 1.225 kg/m³
///   A  = 风轮扫掠面积 = π × R² (m²)
///   Cp = 风能利用系数 (Betz 极限 16/27 ≈ 0.593)
///   V  = 风速 (m/s)
///
/// # 参考文献
/// Betz, A. (1920). Das Maximum der theoretisch möglichen Ausnutzung
///   des Windes durch Windmotoren. Zeitschrift für das gesamte
///   Turbinenwesen, 26, 307-309.
/// Burton, T., Jenkins, N., Sharpe, D., & Bossanyi, E. (2011).
///   Wind Energy Handbook (2nd ed.). Wiley.
/// IEC 61400-12-1: Power performance measurements of electricity
///   producing wind turbines.
use serde::{Deserialize, Serialize};

/// 标准空气密度 (海平面, 15°C, 1 atm) kg/m³
pub const RHO_SEA_LEVEL: f64 = 1.225;

/// Betz 极限 (最大理论 Cp)
pub const BETZ_LIMIT: f64 = 16.0 / 27.0;

// ─── 风力机参数 ───

/// 风力发电机参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurbineParams {
    /// 风轮半径 (m)
    pub radius_m: f64,
    /// 额定功率 (W)
    pub rated_power_w: f64,
    /// 切入风速 (m/s)
    pub cut_in_v: f64,
    /// 额定风速 (m/s)
    pub rated_v: f64,
    /// 切出风速 (m/s)
    pub cut_out_v: f64,
    /// 最大 Cp (Betz 极限 0.593)
    pub cp_max: f64,
    /// 推力系数 CT (Jensen 尾流用)
    pub thrust_coefficient: f64,
    /// 轮毂高度 (m)
    pub hub_height_m: f64,
}

/// 标准商用风力机参数。
impl TurbineParams {
    /// Vestas V80-2.0 MW — 典型陆上风机
    pub fn vestas_v80() -> Self {
        Self {
            radius_m: 40.0,
            rated_power_w: 2_000_000.0,
            cut_in_v: 4.0,
            rated_v: 15.0,
            cut_out_v: 25.0,
            cp_max: 0.47,
            thrust_coefficient: 0.8,
            hub_height_m: 80.0,
        }
    }

    /// Vestas V164-10.0 MW — 典型海上风机
    pub fn vestas_v164() -> Self {
        Self {
            radius_m: 82.0,
            rated_power_w: 10_000_000.0,
            cut_in_v: 3.0,
            rated_v: 13.0,
            cut_out_v: 25.0,
            cp_max: 0.50,
            thrust_coefficient: 0.75,
            hub_height_m: 110.0,
        }
    }

    /// Gamesa G114-2.5 MW — 中低风速型
    pub fn gamesa_g114() -> Self {
        Self {
            radius_m: 57.0,
            rated_power_w: 2_500_000.0,
            cut_in_v: 3.0,
            rated_v: 12.0,
            cut_out_v: 25.0,
            cp_max: 0.48,
            thrust_coefficient: 0.78,
            hub_height_m: 93.0,
        }
    }
}

// ─── 计算结果 ───

/// 涡轮单点功率计算结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurbinePower {
    /// 风速 (m/s)
    pub wind_speed_ms: f64,
    /// 理论功率 (W) — Betz 极限
    pub betz_power_w: f64,
    /// 实际功率 (W) — Cp 修正
    pub actual_power_w: f64,
    /// 空气密度 (kg/m³)
    pub rho: f64,
    /// 风轮扫掠面积 (m²)
    pub swept_area_m2: f64,
    /// 实际 Cp
    pub actual_cp: f64,
    /// 容量因子 (P_actual / P_rated)
    pub capacity_factor: f64,
}

// ─── 核心函数 ───

/// 空气密度随海拔修正。
/// 标准大气近似: ρ(z) = ρ₀ × exp(-z / H₀)
/// H₀ ≈ 8400m (scale height)
pub fn air_density(altitude_m: f64) -> f64 {
    RHO_SEA_LEVEL * (-altitude_m / 8400.0).exp()
}

/// 风轮扫掠面积。
pub fn swept_area(radius_m: f64) -> f64 {
    std::f64::consts::PI * radius_m.powi(2)
}

/// 理想 Betz 功率 (Cp = 16/27)。
pub fn betz_power(rho: f64, swept_area_m2: f64, wind_speed_ms: f64) -> f64 {
    0.5 * rho * swept_area_m2 * BETZ_LIMIT * wind_speed_ms.powi(3)
}

/// 实际风力机功率 (Cp 修正)。
pub fn turbine_power(rho: f64, swept_area_m2: f64, wind_speed_ms: f64, cp: f64) -> f64 {
    0.5 * rho * swept_area_m2 * cp * wind_speed_ms.powi(3)
}

/// 完整功率曲线：输入风速 → 实际功率 (含切入/额定/切出逻辑)。
pub fn power_curve(params: &TurbineParams, wind_speed_ms: f64, rho: f64) -> TurbinePower {
    let a = swept_area(params.radius_m);

    let actual_power_w = if wind_speed_ms < params.cut_in_v || wind_speed_ms >= params.cut_out_v {
        0.0
    } else if wind_speed_ms >= params.rated_v {
        params.rated_power_w
    } else {
        // 额定风速以下：Cp 修正的立方规律
        let p = turbine_power(rho, a, wind_speed_ms, params.cp_max);
        p.min(params.rated_power_w)
    };

    let betz_w = betz_power(rho, a, wind_speed_ms);

    TurbinePower {
        wind_speed_ms,
        betz_power_w: betz_w,
        actual_power_w,
        rho,
        swept_area_m2: a,
        actual_cp: if betz_w > 0.0 {
            actual_power_w / (0.5 * rho * a * wind_speed_ms.powi(3))
        } else {
            0.0
        },
        capacity_factor: if params.rated_power_w > 0.0 {
            actual_power_w / params.rated_power_w
        } else {
            0.0
        },
    }
}

/// 年发电量 (AEP) — 基于 Weibull 风速分布。
///
/// # 参数
/// * `params` — 风机参数
/// * `rho` — 空气密度
/// * `weibull_k` — Weibull 形状参数
/// * `weibull_c` — Weibull 尺度参数 (m/s)
/// * `hours_per_year` — 年小时数 (8760)
pub fn annual_energy_production(
    params: &TurbineParams,
    rho: f64,
    weibull_k: f64,
    weibull_c: f64,
    hours_per_year: f64,
) -> f64 {
    // 数值积分: 风速 0 到 cut_out+5, 步长 0.5 m/s
    let mut total_kwh = 0.0;
    let mut v = 0.0;
    while v <= params.cut_out_v + 5.0 {
        let p = power_curve(params, v, rho);
        // Weibull PDF
        let pdf = if weibull_c > 0.0 && weibull_k > 0.0 {
            let ratio = v / weibull_c;
            (weibull_k / weibull_c) * ratio.powf(weibull_k - 1.0) * (-ratio.powf(weibull_k)).exp()
        } else {
            0.0
        };
        total_kwh += p.actual_power_w * pdf * 0.5; // 0.5 = step
        v += 0.5;
    }
    total_kwh * hours_per_year / 1000.0 // Wh → kWh
}

// ─── 机舱传递函数 ───

/// 轮毂高度风速修正 (对数风廓线)。
///
/// V(z) = Vref × ln(z/z0) / ln(zref/z0)
///
/// # 参数
/// * `v_ref` — 参考高度风速 (m/s)
/// * `z_ref` — 参考高度 (m)
/// * `z_hub` — 轮毂高度 (m)
/// * `z0` — 地表粗糙度长度 (m)，开阔田野 0.03, 森林 0.5-1.0, 海面 0.0002
pub fn wind_shear_log(v_ref: f64, z_ref: f64, z_hub: f64, z0: f64) -> f64 {
    if z_ref <= 0.0 || z_hub <= 0.0 || z0 <= 0.0 {
        return v_ref;
    }
    v_ref * (z_hub / z0).ln() / (z_ref / z0).ln()
}

/// 轮毂高度风速修正 (幂律风廓线)。
///
/// V(z) = Vref × (z/zref)^α
/// α = 风切变指数 (典型: 海域 0.1, 平原 0.15, 丘陵 0.2-0.3, 森林 0.3-0.4)
pub fn wind_shear_power(v_ref: f64, z_ref: f64, z_hub: f64, alpha: f64) -> f64 {
    if z_ref <= 0.0 {
        return v_ref;
    }
    v_ref * (z_hub / z_ref).powf(alpha)
}

/// 标准粗糙度长度表。
pub fn roughness_length(terrain: &str) -> f64 {
    match terrain.to_lowercase().as_str() {
        "sea" | "ocean" | "海洋" | "海面" => 0.0002,
        "open_flat" | "open" | "开阔" | "平原" => 0.03,
        "grassland" | "草原" => 0.05,
        "scrub" | "灌木" => 0.1,
        "suburban" | "suburb" | "郊区" => 0.5,
        "forest" | "森林" => 1.0,
        "urban" | "城市" => 2.0,
        _ => 0.03,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_betz_power() {
        let p = betz_power(1.225, 80.0, 12.0);
        // 0.5 * 1.225 * 80 * 0.593 * 12^3
        let expected = 0.5 * 1.225 * 80.0 * BETZ_LIMIT * 1728.0;
        assert_relative_eq!(p, expected, epsilon = 0.01);
    }

    #[test]
    fn test_power_curve_below_cut_in() {
        let t = TurbineParams::vestas_v80();
        let result = power_curve(&t, 2.0, RHO_SEA_LEVEL);
        assert_relative_eq!(result.actual_power_w, 0.0);
    }

    #[test]
    fn test_power_curve_above_cut_out() {
        let t = TurbineParams::vestas_v80();
        let result = power_curve(&t, 30.0, RHO_SEA_LEVEL);
        assert_relative_eq!(result.actual_power_w, 0.0);
    }

    #[test]
    fn test_power_curve_rated() {
        let t = TurbineParams::vestas_v80();
        let result = power_curve(&t, 15.0, RHO_SEA_LEVEL);
        assert_relative_eq!(result.actual_power_w, t.rated_power_w, epsilon = 1.0);
    }

    #[test]
    fn test_power_curve_partial() {
        let t = TurbineParams::vestas_v80();
        let result = power_curve(&t, 8.0, RHO_SEA_LEVEL);
        assert!(result.actual_power_w > 0.0);
        assert!(result.actual_power_w < t.rated_power_w);
    }

    #[test]
    fn test_wind_shear_log() {
        // 10m → 80m, 开阔平原 z0=0.03
        let v_hub = wind_shear_log(5.0, 10.0, 80.0, 0.03);
        assert!(v_hub > 5.0); // 高度越高风速越大
    }

    #[test]
    fn test_wind_shear_power() {
        let v_hub = wind_shear_power(5.0, 10.0, 80.0, 0.15);
        assert!(v_hub > 5.0);
        assert_relative_eq!(v_hub, 5.0 * (8.0f64).powf(0.15), epsilon = 0.01);
    }

    #[test]
    fn test_air_density() {
        let rho_2000 = air_density(2000.0);
        assert!(rho_2000 < RHO_SEA_LEVEL);
        assert!(rho_2000 > 0.9);
    }

    #[test]
    fn test_swept_area() {
        let a = swept_area(40.0);
        assert_relative_eq!(a, std::f64::consts::PI * 1600.0, epsilon = 0.01);
    }

    #[test]
    fn test_roughness() {
        assert_relative_eq!(roughness_length("sea"), 0.0002);
        assert_relative_eq!(roughness_length("森林"), 1.0);
    }

    #[test]
    fn test_aep_nonzero() {
        let t = TurbineParams::vestas_v80();
        let aep = annual_energy_production(&t, RHO_SEA_LEVEL, 2.0, 6.0, 8760.0);
        assert!(aep > 1_000_000.0); // > 1 GWh/yr
        assert!(aep < 10_000_000.0); // < 10 GWh/yr (2 MW × 8760h = 17.5 GWh max)
    }
}
