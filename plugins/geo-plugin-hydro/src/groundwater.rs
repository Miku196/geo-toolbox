/// 地下水模块：达西定律、MODFLOW 适配器、地表水-地下水耦合。
///
/// 补全水文循环最后一环，与 SCS-CN 产流衔接。
/// 纯 Rust 实现，无外部依赖。
use serde::{Deserialize, Serialize};
use std::fmt::Write;

// ──────────────────────────────────────────────
// 1. 达西定律 + 渗流
// ──────────────────────────────────────────────

/// 达西渗流计算结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarcyFlow {
    /// 达西流速 (m/s)
    pub darcy_velocity_ms: f64,
    /// 渗流流量 (m³/s)
    pub flow_rate_m3_s: f64,
    /// 水力梯度 (m/m)
    pub hydraulic_gradient: f64,
    /// 雷诺数 (判别层流/湍流)
    pub reynolds_number: f64,
    /// 是否层流 (Re < 10)
    pub is_laminar: bool,
}

/// 达西定律: v = K · i
/// K = 渗透系数 (m/s), i = 水力梯度 (Δh/ΔL)
/// Q = v · A = K · i · A
pub fn darcy_law(
    hydraulic_conductivity_ms: f64,
    head_diff_m: f64,
    length_m: f64,
    area_m2: f64,
) -> DarcyFlow {
    let gradient = if length_m > 0.0 {
        head_diff_m / length_m
    } else {
        0.0
    };
    let velocity = hydraulic_conductivity_ms * gradient;
    let flow_rate = velocity * area_m2;

    // Re = ρ·v·d/μ, 使用 d≈0.01m（平均孔隙直径）
    let re = 1000.0 * velocity * 0.01 / 0.001; // ρ=1000, μ=0.001

    DarcyFlow {
        darcy_velocity_ms: velocity,
        flow_rate_m3_s: flow_rate,
        hydraulic_gradient: gradient,
        reynolds_number: re,
        is_laminar: re < 10.0,
    }
}

/// 典型渗透系数 K 值 (m/s)。
pub fn typical_hydraulic_conductivity(material: &str) -> f64 {
    match material.to_lowercase().as_str() {
        "gravel" => 1e-2,
        "coarse_sand" => 1e-3,
        "fine_sand" => 1e-4,
        "silt" => 1e-6,
        "clay" => 1e-9,
        "limestone" => 1e-6,
        "sandstone" => 1e-6,
        "granite" => 1e-11,
        "peat" => 1e-7,
        _ => 1e-5, // 默认: 细砂
    }
}

/// 达西通量: 单位面积通量 (m³/s/m²)
pub fn darcy_flux(k_ms: f64, gradient: f64) -> f64 {
    k_ms * gradient
}

// ──────────────────────────────────────────────
// 2. 地下水流 (1D Boussinesq 简化)
// ──────────────────────────────────────────────

/// 1D Boussinesq 方程计算结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundwaterFlow {
    /// 水位线 (m)，沿流动方向各点水位
    pub head_profile_m: Vec<f64>,
    /// 各点流量 (m³/s)
    pub flow_profile_m3_s: Vec<f64>,
    /// 总水头损失 (m)
    pub total_drawdown_m: f64,
    /// 平均流速 (m/s)
    pub avg_velocity_ms: f64,
}

/// 1D 稳态地下水流: 线性水位剖面。
/// 基于达西定律，恒定横截面积。
pub fn groundwater_flow_1d(
    k_ms: f64,
    head_upstream_m: f64,
    head_downstream_m: f64,
    length_m: f64,
    aquifer_width_m: f64,
    aquifer_thickness_m: f64,
    n_segments: usize,
) -> GroundwaterFlow {
    let area = aquifer_width_m * aquifer_thickness_m;
    let dh = head_upstream_m - head_downstream_m;
    let grad = if length_m > 0.0 { dh / length_m } else { 0.0 };
    let q = k_ms * grad * area;

    let mut head_profile = Vec::with_capacity(n_segments);
    let mut flow_profile = Vec::with_capacity(n_segments);
    for i in 0..n_segments {
        let x = i as f64 / (n_segments - 1).max(1) as f64;
        let head = head_upstream_m - dh * x;
        head_profile.push(head);
        flow_profile.push(q);
    }

    let avg_v = if area > 0.0 { q / area } else { 0.0 };

    GroundwaterFlow {
        head_profile_m: head_profile,
        flow_profile_m3_s: flow_profile,
        total_drawdown_m: dh,
        avg_velocity_ms: avg_v,
    }
}

// ──────────────────────────────────────────────
// 3. MODFLOW 适配器 — 委托到 geo-adapter-modflow
// ──────────────────────────────────────────────

pub use geo_adapter_modflow::{
    generate_bas6 as generate_modflow_bas6, generate_dis as generate_modflow_dis,
    generate_lpf as generate_modflow_lpf, generate_nam as generate_modflow_nam, ModflowGrid,
    ModflowStressPeriod,
};

// ──────────────────────────────────────────────
// 4. 地表水-地下水耦合
// ──────────────────────────────────────────────

/// 耦合计算结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoupledResult {
    /// 入渗补给量 (m³/s)
    pub recharge_m3_s: f64,
    /// 基流回归量 (m³/s)
    pub baseflow_m3_s: f64,
    /// 潜水蒸发 (m³/s)
    pub evaporation_m3_s: f64,
    /// 净补给 (m³/s)
    pub net_recharge_m3_s: f64,
    /// 水位变化 (m)
    pub water_table_change_m: f64,
}

/// 简单 SCS-CN 入渗补给。
/// 产流 = 降雨 - 初损 - 入渗
/// 补给 = 入渗量 × 补给系数
pub fn scs_recharge(rainfall_m: f64, cn: f64, recharge_factor: f64, area_m2: f64) -> f64 {
    let s = 254.0 * (100.0 / cn - 1.0) / 1000.0; // 潜在滞蓄量 (m)
    let ia = 0.2 * s; // 初损
    let runoff = if rainfall_m > ia {
        (rainfall_m - ia).powi(2) / (rainfall_m - ia + s)
    } else {
        0.0
    };
    let infiltration = (rainfall_m - runoff - ia).max(0.0);
    infiltration * recharge_factor * area_m2 / 86400.0 // m³/s
}

/// 基流回归 (线性水库)。
pub fn baseflow_exchange(
    aquifer_head_m: f64,
    stream_stage_m: f64,
    streambed_k_ms: f64,
    streambed_thickness_m: f64,
    stream_length_m: f64,
    stream_width_m: f64,
) -> f64 {
    let dh = aquifer_head_m - stream_stage_m;
    let area = stream_length_m * stream_width_m;
    let k_over_b = streambed_k_ms / streambed_thickness_m.max(0.01);
    k_over_b * dh * area // m³/s
}

/// 完整耦合计算。
pub fn coupled_water_balance(
    rainfall_m: f64,
    cn: f64,
    recharge_factor: f64,
    area_m2: f64,
    aquifer_head_m: f64,
    stream_stage_m: f64,
    streambed_k_ms: f64,
    streambed_thickness_m: f64,
    stream_length_m: f64,
    stream_width_m: f64,
    specific_yield: f64,
    aquifer_thickness_m: f64,
) -> CoupledResult {
    let recharge = scs_recharge(rainfall_m, cn, recharge_factor, area_m2);
    let baseflow = baseflow_exchange(
        aquifer_head_m,
        stream_stage_m,
        streambed_k_ms,
        streambed_thickness_m,
        stream_length_m,
        stream_width_m,
    );
    let storage_coeff = specific_yield * area_m2;
    let net = recharge - baseflow;
    let delta_h = if storage_coeff > 0.0 {
        net * 86400.0 / storage_coeff // daily water table change
    } else {
        0.0
    };

    CoupledResult {
        recharge_m3_s: recharge,
        baseflow_m3_s: baseflow.max(0.0),
        evaporation_m3_s: 0.0,
        net_recharge_m3_s: net,
        water_table_change_m: delta_h,
    }
}

// ──────────────────────────────────────────────
// 测试
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_darcy_law_basic() {
        let flow = darcy_law(1e-4, 5.0, 100.0, 10.0);
        assert!((flow.hydraulic_gradient - 0.05).abs() < 1e-6);
        assert!((flow.darcy_velocity_ms - 5e-6).abs() < 1e-10);
        assert!((flow.flow_rate_m3_s - 5e-5).abs() < 1e-10);
    }

    #[test]
    fn test_darcy_law_impermeable() {
        let flow = darcy_law(1e-11, 10.0, 100.0, 100.0); // granite
        assert!(flow.darcy_velocity_ms < 1e-10);
        assert!(flow.reynolds_number < 1.0);
    }

    #[test]
    fn test_typical_k() {
        assert!(typical_hydraulic_conductivity("gravel") > 1e-3);
        assert!(typical_hydraulic_conductivity("clay") < 1e-7);
        assert!(typical_hydraulic_conductivity("granite") < 1e-9);
    }

    #[test]
    fn test_groundwater_flow_1d() {
        let result = groundwater_flow_1d(1e-4, 50.0, 45.0, 100.0, 50.0, 20.0, 5);
        assert_eq!(result.head_profile_m.len(), 5);
        assert!((result.avg_velocity_ms - 5e-6).abs() < 1e-10);
    }

    #[test]
    fn test_modflow_dis() {
        let dis = generate_modflow_dis(1, 10, 10, 100.0, 100.0, 50.0, 0.0, 1);
        assert!(dis.contains("1 10 10"));
        assert!(dis.contains("100"));
        assert!(dis.contains("50"));
    }

    #[test]
    fn test_modflow_bas6() {
        let bas = generate_modflow_bas6(1, 50.0, 3, 4);
        assert!(bas.contains("FREE"));
        assert!(bas.contains("1 1 1 1"));
        assert!(bas.contains("50"));
    }

    #[test]
    fn test_modflow_lpf() {
        let lpf = generate_modflow_lpf(1e-4, 1e-5, 1e-6, 0.2, 2, 3);
        assert!(lpf.contains("HK"));
        assert!(lpf.contains("0.0001"));
    }

    #[test]
    fn test_scs_recharge() {
        let q = scs_recharge(0.05, 75.0, 0.3, 10000.0);
        assert!(q >= 0.0);
    }

    #[test]
    fn test_baseflow_exchange() {
        let q = baseflow_exchange(50.0, 48.0, 1e-6, 1.0, 100.0, 10.0);
        assert!(q > 0.0); // aquifer higher → baseflow to stream
    }

    #[test]
    fn test_coupled_water_balance() {
        let r = coupled_water_balance(
            0.05, 75.0, 0.3, 10000.0, 50.0, 48.0, 1e-6, 1.0, 100.0, 10.0, 0.2, 10.0,
        );
        assert!(r.recharge_m3_s >= 0.0);
        assert!(r.baseflow_m3_s >= 0.0);
    }
}
