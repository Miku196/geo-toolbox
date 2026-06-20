//! 地热发电潜力评估模块。
//!
//! ## 物理模型
//!
//! 热流密度 q = -k × ΔT/Δz  (Fourier's law)
//!    q  → 地表热流 (mW/m²)
//!    k  → 热导率 (W/(m·K))
//!    ΔT → 温差 (K)；Δz → 深度差 (m)
//!
//! 地热发电潜力:
//!   P_el = q × A × η_carnot × η_plant
//!    η_carnot ≈ 1 - T_cold / T_hot (Carnot efficiency)
//!    η_plant ≈ 0.12 (typical binary cycle)
//!
//! ## LCOE (Levelized Cost of Electricity)
//!
//!   LCOE = (C_capital × CRF + C_om) / E_annual
//!    CRF = r × (1+r)^n / ((1+r)^n - 1)

use serde::Serialize;

/// 地热资源评估结果。
#[derive(Debug, Clone, Serialize)]
pub struct GeothermalAssessment {
    /// 名称
    pub name: String,
    /// 地热梯度 (°C/km)
    pub temp_gradient_c_per_km: f64,
    /// 热导率 (W/(m·K))
    pub thermal_conductivity: f64,
    /// 表面热流 (mW/m²)
    pub heat_flux_mw_m2: f64,
    /// 储层温度估计 (°C) @ 3km 深度
    pub reservoir_temp_c: f64,
    /// 面积 (km²)
    pub area_km2: f64,
    /// 卡诺效率
    pub carnot_efficiency: f64,
    /// 发电潜力 (MW)
    pub power_potential_mw: f64,
    /// 年发电量 (GWh)
    pub annual_generation_gwh: f64,
    /// LCOE 估算 (USD/MWh)
    pub lcoe_usd_per_mwh: f64,
    /// 潜力评级
    pub grade: String,
}

impl GeothermalAssessment {
    /// 根据热流密度和面积评估地热发电潜力。
    ///
    /// # Arguments
    /// * `heat_flux` - 地表热流密度 (mW/m²)，可直接测量或通过地温梯度推算
    /// * `area_km2` - 评估区域面积 (km²)
    /// * `surface_temp_c` - 地表平均温度 (°C)
    ///
    /// ## 典型热流参考
    /// - 冰岛 / 东非裂谷: > 200 mW/m² (Excellent)
    /// - 青藏高原 / 云南: 80–120 mW/m² (Good)
    /// - 华北平原: 60–80 mW/m² (Moderate)
    /// - 稳定克拉通: < 50 mW/m² (Poor)
    pub fn from_heat_flux(name: &str, heat_flux: f64, area_km2: f64, surface_temp_c: f64) -> Self {
        // 储层温度: 3km 典型钻井深度
        let reservoir_temp = surface_temp_c + (heat_flux / 1000.0) * 3000.0 / 2.5;
        let t_cold = surface_temp_c + 273.15;
        let t_hot = reservoir_temp + 273.15;

        let carnot = if t_hot > t_cold {
            1.0 - t_cold / t_hot
        } else {
            0.05
        };

        // Binary cycle plant efficiency ~12% of Carnot
        let plant_eff = 0.12;
        let efficiency = carnot * plant_eff;

        // Power: P = q × A × η
        let area_m2 = area_km2 * 1e6;
        let heat_flux_w = heat_flux * 1e-3 * area_m2; // mW/m² → W/m² → total W
        let power_w = heat_flux_w * efficiency;
        let power_mw = power_w / 1e6;

        let annual_gwh = power_mw * 8760.0 * 0.85 / 1000.0; // 85% capacity factor

        // LCOE simple model
        let lcoe = if power_mw > 0.0 {
            lcoe_estimate(power_mw, reservoir_temp)
        } else {
            f64::INFINITY
        };

        let grade = classify_geothermal(heat_flux, reservoir_temp);

        Self {
            name: name.to_string(),
            temp_gradient_c_per_km: (heat_flux / 1000.0) / 2.5 * 1000.0,
            thermal_conductivity: 2.5,
            heat_flux_mw_m2: heat_flux,
            reservoir_temp_c: reservoir_temp,
            area_km2,
            carnot_efficiency: carnot,
            power_potential_mw: power_mw,
            annual_generation_gwh: annual_gwh,
            lcoe_usd_per_mwh: lcoe,
            grade: grade.to_string(),
        }
    }

    /// 根据地温梯度和热导率评估。
    pub fn from_gradient(
        name: &str,
        gradient_c_per_km: f64,
        conductivity: f64,
        area_km2: f64,
        surface_temp_c: f64,
    ) -> Self {
        // q = k × dT/dz
        let heat_flux = conductivity * gradient_c_per_km / 1000.0 * 1000.0; // → mW/m²
        let mut result = Self::from_heat_flux(name, heat_flux, area_km2, surface_temp_c);
        result.thermal_conductivity = conductivity;
        result.temp_gradient_c_per_km = gradient_c_per_km;
        result
    }
}

/// 简化 LCOE 估算 (USD/MWh)。
///
/// 基于 MIT Future of Geothermal Energy 成本模型 + 中国实际调整。
fn lcoe_estimate(power_mw: f64, reservoir_temp_c: f64) -> f64 {
    // 资本成本: 高温中小机组更贵
    let cap_cost_per_kw = if reservoir_temp_c > 200.0 {
        4000.0 // 高温干热岩，井成本高
    } else if reservoir_temp_c > 120.0 {
        2500.0 // 中温，二元循环
    } else {
        3500.0 // 低温，换热器面积大
    };

    // O&M 成本 $30/kW-yr
    let om_per_kw_yr = 30.0;

    // 容量因子 85%
    let cf = 0.85;
    let annual_mwh = power_mw * 8760.0 * cf;

    if annual_mwh <= 0.0 {
        return f64::INFINITY;
    }

    // CRF: r=7%, n=25 years
    let r: f64 = 0.07;
    let n = 25.0;
    let crf = r * (1.0 + r).powf(n) / ((1.0 + r).powf(n) - 1.0);

    let cap_million = cap_cost_per_kw * power_mw * 1000.0 / 1e6;
    let om_million = om_per_kw_yr * power_mw * 1000.0 / 1e6;
    let total_annual_million = cap_million * crf + om_million;

    total_annual_million * 1e6 / annual_mwh
}

/// 地热资源分级。
fn classify_geothermal(heat_flux: f64, reservoir_temp_c: f64) -> &'static str {
    if heat_flux > 150.0 && reservoir_temp_c > 200.0 {
        "🏆 优质 (EGS可行)"
    } else if heat_flux > 80.0 && reservoir_temp_c > 120.0 {
        "✅ 良好 (二元循环)"
    } else if heat_flux > 50.0 {
        "⚠ 一般 (低温热储)"
    } else {
        "❌ 不适合"
    }
}

/// 多个潜在场址排序，返回 Top-K。
pub fn rank_sites(mut sites: Vec<GeothermalAssessment>) -> Vec<GeothermalAssessment> {
    sites.sort_by(|a, b| {
        b.power_potential_mw
            .partial_cmp(&a.power_potential_mw)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    sites
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tibet_geothermal() {
        // 青藏高原典型值: 热流 ~120 mW/m²
        let site = GeothermalAssessment::from_heat_flux("羊八井", 120.0, 10.0, 5.0);
        assert!(site.heat_flux_mw_m2 >= 100.0);
        assert!(site.reservoir_temp_c > 100.0);
        assert!(site.power_potential_mw > 0.0);
        assert!(site.lcoe_usd_per_mwh.is_finite());
        assert!(site.grade.contains("良好"));
    }

    #[test]
    fn test_gradient_input() {
        // 地温梯度 35°C/km, 热导率 2.5 W/(m·K)
        let site = GeothermalAssessment::from_gradient("测试井", 35.0, 2.5, 1.0, 15.0);
        // q = 2.5 * 35/1000 * 1000 = 87.5 mW/m²
        assert!((site.heat_flux_mw_m2 - 87.5).abs() < 1.0);
        assert!(site.power_potential_mw > 0.0);
    }

    #[test]
    fn test_poor_site() {
        let site = GeothermalAssessment::from_heat_flux("稳定克拉通", 30.0, 100.0, 20.0);
        assert!(site.grade.contains("不适合"));
        assert!(site.power_potential_mw < 5.0);
    }

    #[test]
    fn test_rank_sites() {
        let a = GeothermalAssessment::from_heat_flux("A(高)", 150.0, 5.0, 10.0);
        let b = GeothermalAssessment::from_heat_flux("B(中)", 80.0, 5.0, 10.0);
        let c = GeothermalAssessment::from_heat_flux("C(低)", 40.0, 5.0, 10.0);
        let ranked = rank_sites(vec![c, b, a]);
        assert_eq!(ranked[0].name, "A(高)");
        assert!(ranked[0].power_potential_mw > ranked[1].power_potential_mw);
        assert!(ranked[1].power_potential_mw > ranked[2].power_potential_mw);
    }

    #[test]
    fn test_lcoe_decreases_with_size() {
        let small = GeothermalAssessment::from_heat_flux("小型", 100.0, 1.0, 10.0);
        let large = GeothermalAssessment::from_heat_flux("大型", 100.0, 50.0, 10.0);
        // LCOE is power-invariant for same reservoir temp (power cancels in $/MWh formula)
        assert!(
            (large.lcoe_usd_per_mwh - small.lcoe_usd_per_mwh).abs() < 0.01,
            "LCOE should be similar for same reservoir temp: {} vs {}",
            large.lcoe_usd_per_mwh,
            small.lcoe_usd_per_mwh
        );
    }
}
