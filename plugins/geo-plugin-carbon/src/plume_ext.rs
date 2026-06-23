/// 大气边界层与空气质量扩展函数 — 高斯烟羽的上下游参数。
///
/// 提供：
/// - 大气边界层高度估算 (ABL)
/// - 湍流热通量估算 (感热/潜热)
/// - AOD 到 PM2.5 浓度转化
use crate::plume::StabilityClass;

/// 根据风速、粗糙度和稳定度估算大气边界层高度 (m)。
///
/// 使用基于 Pasquill-Gifford 稳定度的经验关系:
/// - A/B (不稳定): h ≈ 1000-1500 m (对流边界层)
/// - C/D (中性): h ≈ 600-1000 m (机械混合层)
/// - E/F (稳定): h ≈ 100-400 m (稳定边界层)
///
/// 风速和粗糙度对中性/不稳定条件下有修正。
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

    // 机械混合修正: 在中性/不稳定条件下，风速增强混合层
    let ws_factor = match stability {
        StabilityClass::A | StabilityClass::B | StabilityClass::C | StabilityClass::D => {
            1.0 + 0.1 * (wind_speed_m_s - 3.0).max(0.0) / wind_speed_m_s.max(1.0)
        }
        _ => 1.0,
    };

    // 粗糙度修正: 粗糙地表增加机械湍流
    let z0_factor = 1.0 + 0.2 * (roughness_m / 0.1).min(1.0).max(0.0);

    let h = base * ws_factor * z0_factor;
    h.clamp(50.0, 2500.0)
}

/// 估算感热通量 (SHF, W/m²) 和潜热通量 (LHF, W/m²)。
///
/// 使用空气动力学方法:
/// SHF = ρ · Cp · CH · u · (T_surface - T_air)
/// LHF = ρ · Lv · CE · u · (q_surface - q_air)
///
/// 简化版: 不引入完整湿度剖面，用温度差和风速近似估算。
///
/// # 参数
/// - `temp_profile` — [T_surface, T_2m, T_10m, T_50m] (°C)
/// - `wind_profile` — [u_2m, u_10m, u_50m] (m/s)
///
/// # 返回
/// - `(shf_w_m2, lhf_w_m2)` — 感热通量, 潜热通量
pub fn turbulent_heat_fluxes(temp_profile: &[f64], wind_profile: &[f64]) -> (f64, f64) {
    if temp_profile.len() < 2 || wind_profile.is_empty() {
        return (0.0, 0.0);
    }

    let ts = temp_profile[0]; // 地表温度
    let t2 = temp_profile.get(1).copied().unwrap_or(ts); // 2m 气温
    let u10 = wind_profile.get(1).copied().unwrap_or(3.0); // 10m 风速
                                                           // 通常 u2 = u10 * 0.75 (对数廓线近似)
    let u2 = wind_profile.first().copied().unwrap_or(u10 * 0.75);

    // 空气密度 ρ (kg/m³), 近似 1.225 @ 15°C, 温度修正
    let rho = 1.225 * (288.15 / (t2 + 273.15));
    // 空气定压比热 Cp (J/kg·K)
    let cp = 1005.0;
    // 潜热汽化 Lv (J/kg)
    let lv = 2.5e6;

    // 中性条件下的传输系数
    // CH ≈ 0.0011 (感热), CE ≈ 0.0012 (潜热) — 经验值
    let ch = 0.0011;
    let ce = 0.0012;

    // 感热通量: SHF = ρ · Cp · CH · u · ΔT
    let dtemp = ts - t2;
    let shf = rho * cp * ch * u10 * dtemp;

    // 潜热通量 (简化): 使用温湿关系估算
    // 假设地面相对湿度 70%, 2m 相对湿度 50%
    let es_surf = 611.0 * ((17.27 * ts) / (ts + 237.3)).exp(); // 饱和水汽压 (Pa)
    let es_2m = 611.0 * ((17.27 * t2) / (t2 + 237.3)).exp();
    let q_surf = 0.622 * (0.70 * es_surf) / (101325.0 - 0.378 * 0.70 * es_surf);
    let q_2m = 0.622 * (0.50 * es_2m) / (101325.0 - 0.378 * 0.50 * es_2m);
    let dq = (q_surf - q_2m).max(0.0);

    let lhf = rho * lv * ce * u2 * dq;

    (shf, lhf)
}

/// 从 AOD (气溶胶光学厚度, 550nm) 估算地面 PM2.5 浓度 (μg/m³)。
///
/// PM2.5 = AOD_550 × η × f(RH)
/// 其中:
/// - η = AOD 到 PM2.5 的质量转换因子 (实测或模式比值)
/// - f(RH) = 吸湿增长因子
///
/// # 参数
/// - `aod_550` — 550nm 气溶胶光学厚度
/// - `aod_ratio` — AOD/PBLH 转换参数 (默认 0.025 μg/m³)
/// - `rh_correction` — 相对湿度校正因子 (1.0 = 无校正, >1.0 = 吸湿增长)
///
/// # 参考
/// - Van Donkelaar et al. (2016), Global Estimates of Fine Particulate
///   Matter using a Combined Geophysical-Statistical Method
pub fn aod_to_pm25(aod_550: f64, aod_ratio: f64, rh_correction: f64) -> f64 {
    if aod_550 < 0.0 {
        return 0.0;
    }
    let ratio = if aod_ratio > 0.0 { aod_ratio } else { 0.025 };
    let rh = if rh_correction > 0.0 {
        rh_correction
    } else {
        1.0
    };
    // PM2.5 = AOD × η × f(RH)
    let pm25 = aod_550 * ratio * rh;
    // 地表浓度 (μg/m³), 扣除非气溶胶背景 (2 μg/m³)
    (pm25 * 1000.0 - 2.0).max(0.0)
}

/// 结合 AOD 和边界层高度的改进 PM2.5 估算。
///
/// PM2.5 ≈ AOD × (PBLH / 1000)^(-1) × η × f(RH)
/// 考虑了气溶胶在边界层内的垂直分布。
pub fn aod_to_pm25_with_pblh(aod_550: f64, pblh_m: f64, aod_ratio: f64, rh_correction: f64) -> f64 {
    if aod_550 < 0.0 || pblh_m <= 0.0 {
        return 0.0;
    }
    let ratio = if aod_ratio > 0.0 { aod_ratio } else { 0.025 };
    let rh = if rh_correction > 0.0 {
        rh_correction
    } else {
        1.0
    };
    // PBLH 越浅，地面浓度越高 (气溶胶压缩在更小体积内)
    let pblh_factor = (1000.0 / pblh_m).clamp(0.5, 3.0);
    let pm25 = aod_550 * ratio * rh * pblh_factor;
    (pm25 * 1000.0 - 2.0).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abl_height_unstable() {
        let h = atmospheric_boundary_layer_height(5.0, 0.03, StabilityClass::A);
        assert!(h > 1000.0);
        assert!(h > 1200.0);
    }

    #[test]
    fn test_abl_height_stable() {
        let h = atmospheric_boundary_layer_height(2.0, 0.01, StabilityClass::F);
        assert!(h < 400.0);
        assert!(h >= 50.0);
    }

    #[test]
    fn test_abl_height_neutral() {
        let h = atmospheric_boundary_layer_height(8.0, 0.1, StabilityClass::D);
        assert!(h > 500.0);
        assert!(h < 2000.0);
    }

    #[test]
    fn test_abl_wind_speed_effect() {
        let calm = atmospheric_boundary_layer_height(1.0, 0.03, StabilityClass::D);
        let windy = atmospheric_boundary_layer_height(15.0, 0.03, StabilityClass::D);
        assert!(windy > calm);
    }

    #[test]
    fn test_turbulent_heat_fluxes() {
        // 白天: 地表 30°C, 2m 25°C, 有风
        let (shf, lhf) = turbulent_heat_fluxes(&[30.0, 25.0, 23.0, 21.0], &[3.0, 5.0, 7.0]);
        assert!(shf > 0.0);
        assert!(lhf > 0.0);
    }

    #[test]
    fn test_turbulent_heat_fluxes_stable() {
        // 夜间逆温: 地表 5°C, 2m 8°C → 负感热通量
        let (shf, lhf) = turbulent_heat_fluxes(&[5.0, 8.0, 9.0, 8.0], &[1.0, 2.0, 3.0]);
        assert!(shf < 0.0);
    }

    #[test]
    fn test_turbulent_heat_fluxes_short_profile() {
        let (shf, lhf) = turbulent_heat_fluxes(&[20.0], &[2.0]);
        assert!((shf - 0.0).abs() < 1e-10); // only 1 temp point
    }

    #[test]
    fn test_aod_to_pm25_basic() {
        let pm = aod_to_pm25(0.3, 0.025, 1.0);
        // 0.3 * 0.025 * 1.0 * 1000 - 2 = 5.5 μg/m³
        assert!((pm - 5.5).abs() < 0.1);
    }

    #[test]
    fn test_aod_to_pm25_high_aod() {
        let pm = aod_to_pm25(1.5, 0.025, 1.2);
        // 1.5 * 0.025 * 1.2 * 1000 - 2 = 43 μg/m³
        assert!(pm > 40.0);
    }

    #[test]
    fn test_aod_to_pm25_negative_aod() {
        let pm = aod_to_pm25(-1.0, 0.025, 1.0);
        assert_eq!(pm, 0.0);
    }

    #[test]
    fn test_aod_to_pm25_default_ratio() {
        let pm = aod_to_pm25(0.5, 0.0, 1.0);
        // uses default ratio 0.025
        assert!((pm - 10.5).abs() < 0.1);
    }

    #[test]
    fn test_aod_to_pm25_with_pblh() {
        // Shallow boundary layer → higher ground concentration
        let shallow = aod_to_pm25_with_pblh(0.5, 300.0, 0.025, 1.0);
        let deep = aod_to_pm25_with_pblh(0.5, 1500.0, 0.025, 1.0);
        assert!(shallow > deep);
    }

    #[test]
    fn test_aod_to_pm25_with_pblh_zero_pblh() {
        let pm = aod_to_pm25_with_pblh(0.5, 0.0, 0.025, 1.0);
        assert_eq!(pm, 0.0);
    }
}
