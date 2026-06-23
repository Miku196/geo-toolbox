//! 生态系统服务模块 — 碳固存、水源涵养、游憩潜力。
use serde::{Deserialize, Serialize};

/// 碳密度表条目 (tCO₂e/ha)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonDensity {
    pub landcover_class: String,
    pub density_tco2e_per_ha: f64,
}

/// 生态系统服务评估结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcoServiceResult {
    /// 碳固存量 (tCO₂e)
    pub carbon_sequestration: f64,
    /// 水源涵养量 (m³)
    pub water_yield_m3: f64,
    /// 游憩潜力 (0-1)
    pub recreation_index: f64,
    /// 总面积 (ha)
    pub area_ha: f64,
}

/// 根据土地覆盖变化计算碳固存/排放。
///
/// 参数:
/// - `class_areas_ha` — 每种土地覆盖类型的面积 (ha)
/// - `carbon_densities` — 每种类型的碳密度 (tCO₂e/ha)
/// - `changes` — 变化矩阵: changes[from][to] = area_change ha, 正值 = 向此类型转化
///
/// 返回: 总碳变化 (tCO₂e), 正 = 排放, 负 = 固存
pub fn carbon_sequestration_service(
    class_areas_ha: &[f64],
    carbon_densities: &[f64],
    changes: &[Vec<f64>],
) -> f64 {
    let n = class_areas_ha.len().min(carbon_densities.len());
    if n == 0 {
        return 0.0;
    }

    // Current carbon stock
    let current_stock: f64 = (0..n)
        .map(|i| class_areas_ha[i] * carbon_densities[i])
        .sum();

    // Future carbon stock after changes
    let mut future_areas = class_areas_ha.to_vec();
    for from in 0..changes.len().min(n) {
        for to in 0..changes[from].len().min(n) {
            let amount = changes[from][to];
            if amount <= 0.0 {
                continue;
            }
            // 转入 to
            if to < future_areas.len() {
                future_areas[to] += amount;
            }
        }
    }

    let future_stock: f64 = (0..n).map(|i| future_areas[i] * carbon_densities[i]).sum();

    future_stock - current_stock
}

/// 计算水源涵养量: Y = (P - ET) × A
///
/// 参数:
/// - `precip_mm` — 年均降水量 (mm)
/// - `et_mm` — 年均蒸散发量 (mm)
/// - `area_ha` — 面积 (ha)
///
/// 返回: 产水量 (m³)
pub fn water_yield_service(precip_mm: f64, et_mm: f64, area_ha: f64) -> f64 {
    let net_mm = (precip_mm - et_mm).max(0.0);
    // 1 mm * 1 ha = 10 m³
    net_mm * area_ha * 10.0
}

/// 计算游憩潜力 (0-1):
/// 基于土地覆盖多样性 (Shannon) × 可达性 × 归一化因子
///
/// 参数:
/// - `landcover_counts` — 每种土地覆盖的像素数
/// - `accessibility` — 可达性评分 (0-1, 如道路密度归一化)
pub fn recreation_potential(landcover_counts: &[usize], accessibility: f64) -> f64 {
    let total: usize = landcover_counts.iter().sum();
    if total == 0 {
        return 0.0;
    }

    // Shannon diversity index
    let shannon: f64 = landcover_counts
        .iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / total as f64;
            -p * p.ln()
        })
        .sum();

    // Normalize Shannon by ln(n_classes)
    let n_classes = landcover_counts.iter().filter(|&&c| c > 0).count();
    let diversity = if n_classes > 1 {
        (shannon / (n_classes as f64).ln()).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // Combine: diversity * accessibility, clamp to 0-1
    (diversity * accessibility).clamp(0.0, 1.0)
}

/// 完整生态系统服务评估管线。
pub fn assess_ecosystem_services(
    class_areas_ha: &[f64],
    carbon_densities: &[f64],
    changes: &[Vec<f64>],
    precip_mm: f64,
    et_mm: f64,
    landcover_counts: &[usize],
    accessibility: f64,
) -> EcoServiceResult {
    let carbon = carbon_sequestration_service(class_areas_ha, carbon_densities, changes);
    let water = water_yield_service(precip_mm, et_mm, class_areas_ha.iter().sum());
    let recreation = recreation_potential(landcover_counts, accessibility);
    let area: f64 = class_areas_ha.iter().sum();

    EcoServiceResult {
        carbon_sequestration: carbon,
        water_yield_m3: water,
        recreation_index: recreation,
        area_ha: area,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_carbon_sequestration_no_change() {
        let areas = vec![10.0, 20.0];
        let densities = vec![-5.0, 2.0];
        let changes = vec![vec![0.0, 0.0], vec![0.0, 0.0]];
        let val = carbon_sequestration_service(&areas, &densities, &changes);
        assert!((val).abs() < 1e-6);
    }

    #[test]
    fn test_carbon_sequestration_afforestation() {
        let areas = vec![100.0, 0.0]; // cropland -> forest
        let densities = vec![-5.0, -10.0]; // forest sinks more
                                           // 10 ha cropland becomes forest
        let changes = vec![vec![0.0, 10.0], vec![0.0, 0.0]];
        let val = carbon_sequestration_service(&areas, &densities, &changes);
        assert!(val < 0.0); // net sink
    }

    #[test]
    fn test_water_yield() {
        let val = water_yield_service(1000.0, 600.0, 10.0);
        assert!((val - 400.0 * 10.0 * 10.0).abs() < 0.01);
    }

    #[test]
    fn test_water_yield_zero() {
        let val = water_yield_service(100.0, 200.0, 10.0);
        assert_eq!(val, 0.0);
    }

    #[test]
    fn test_recreation_uniform() {
        let val = recreation_potential(&[100, 100], 1.0);
        assert!(val > 0.5);
    }

    #[test]
    fn test_recreation_single_class() {
        let val = recreation_potential(&[200], 0.5);
        assert!((val - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_recreation_low_accessibility() {
        let val = recreation_potential(&[100, 100], 0.0);
        assert_eq!(val, 0.0);
    }

    #[test]
    fn test_assess_full() {
        let result = assess_ecosystem_services(
            &[50.0, 50.0],
            &[-5.0, 2.0],
            &[vec![0.0, 10.0], vec![0.0, 0.0]],
            1200.0,
            500.0,
            &[50, 50],
            0.7,
        );
        assert!(result.area_ha > 0.0);
        assert!(result.water_yield_m3 > 0.0);
        assert!(result.recreation_index > 0.0);
    }
}
