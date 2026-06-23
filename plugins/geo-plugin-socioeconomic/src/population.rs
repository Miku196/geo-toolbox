/// 人口空间化模块 — dasymetric mapping, GDP 估算, 财富指数。
use serde::{Deserialize, Serialize};

/// 人口空间化结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopulationResult {
    /// 每栅格的人口估值
    pub population_grid: Vec<f64>,
    /// 总人口（应 ≈ admin_pop）
    pub total_population: f64,
    /// 最大人口密度 (persons/km²)
    pub max_density: f64,
    /// GDP 估值网格
    pub gdp_grid: Option<Vec<f64>>,
    /// 总 GDP
    pub total_gdp: Option<f64>,
}

/// 等面积 dasymetric mapping: 将行政区人口按土地覆盖权重分配到栅格。
///
/// `admin_pop` — 行政区总人口
/// `landcover_weights` — 每栅格的土地覆盖权重 (0-1, 如建设用地=1, 水域=0)
/// `cell_area_km2` — 单栅格面积 (km²)
/// 返回每栅格人口密度 (persons/km²) 及人口数
pub fn pop_density_from_landcover(
    admin_pop: f64,
    landcover_weights: &[f64],
    cell_area_km2: f64,
) -> (Vec<f64>, Vec<f64>) {
    let total_weight: f64 = landcover_weights.iter().sum();
    if total_weight <= 0.0 || cell_area_km2 <= 0.0 {
        return (vec![0.0; landcover_weights.len()], vec![0.0; landcover_weights.len()]);
    }
    let pop_per_weight = admin_pop / total_weight;
    let densities: Vec<f64> = landcover_weights.iter().map(|&w| {
        let pop = w * pop_per_weight;
        if cell_area_km2 > 0.0 { pop / cell_area_km2 } else { 0.0 }
    }).collect();
    let pops: Vec<f64> = landcover_weights.iter().map(|&w| w * pop_per_weight).collect();
    (densities, pops)
}

/// 夜间灯光 → GDP 估算（线性模型）。
/// GDP = ntl × calibration_factor
pub fn nightlight_to_gdp(ntl_values: &[f64], calibration: f64) -> Vec<f64> {
    ntl_values.iter().map(|&n| n * calibration).collect()
}

/// 综合财富指数（基于 NTL + 建筑密度 + 路网密度）。
/// index = w1×ntl_norm + w2×building_norm + w3×road_norm
pub fn wealth_index(
    ntl: &[f64],
    building_density: &[f64],
    road_density: &[f64],
) -> Vec<f64> {
    let n = ntl.len().min(building_density.len()).min(road_density.len());
    let normalize = |vals: &[f64]| -> Vec<f64> {
        let max = vals.iter().copied().fold(0.0_f64, f64::max);
        if max > 0.0 { vals.iter().map(|&v| v / max).collect() } else { vals.to_vec() }
    };
    let ntl_n = normalize(&ntl[..n]);
    let bld_n = normalize(&building_density[..n]);
    let road_n = normalize(&road_density[..n]);
    (0..n).map(|i| {
        (ntl_n[i] * 0.5 + bld_n[i] * 0.3 + road_n[i] * 0.2) * 100.0
    }).collect()
}

/// 完整人口空间化管线。
pub fn full_population_pipeline(
    admin_pop: f64,
    landcover_weights: &[f64],
    cell_area_km2: f64,
    ntl: Option<&[f64]>,
    calibration: f64,
    building_density: Option<&[f64]>,
    road_density: Option<&[f64]>,
) -> PopulationResult {
    let (densities, pops) = pop_density_from_landcover(admin_pop, landcover_weights, cell_area_km2);
    let max_density = densities.iter().copied().fold(0.0_f64, f64::max);
    let total_pop: f64 = pops.iter().sum();

    let (gdp_grid, total_gdp) = if let Some(ntl_vals) = ntl {
        let gdp = nightlight_to_gdp(ntl_vals, calibration);
        let total: f64 = gdp.iter().sum();
        (Some(gdp), Some(total))
    } else {
        (None, None)
    };

    PopulationResult {
        population_grid: pops,
        total_population: total_pop,
        max_density,
        gdp_grid,
        total_gdp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pop_density_uniform() {
        let weights = vec![1.0, 1.0, 1.0];
        let (densities, pops) = pop_density_from_landcover(3000.0, &weights, 0.01);
        assert!((densities[0] - 100000.0).abs() < 1e-6);
        assert!((pops[0] - 1000.0).abs() < 1e-6);
        assert!((pops.iter().sum::<f64>() - 3000.0).abs() < 1e-6);
    }

    #[test]
    fn test_pop_density_zero_weight() {
        let weights = vec![0.0, 0.0];
        let (densities, pops) = pop_density_from_landcover(1000.0, &weights, 0.01);
        assert!(densities.iter().all(|&d| d == 0.0));
        assert!(pops.iter().all(|&p| p == 0.0));
    }

    #[test]
    fn test_nightlight_to_gdp() {
        let ntl = vec![10.0, 20.0, 30.0];
        let gdp = nightlight_to_gdp(&ntl, 0.5);
        assert!((gdp[0] - 5.0).abs() < 1e-6);
        assert!((gdp[2] - 15.0).abs() < 1e-6);
    }

    #[test]
    fn test_wealth_index() {
        let ntl = vec![10.0, 20.0];
        let bld = vec![0.5, 0.1];
        let road = vec![0.8, 0.2];
        let index = wealth_index(&ntl, &bld, &road);
        assert!(index[0] > index[1]);
        assert!((index[0] - 75.0).abs() < 1e-6); // (0.5*0.5+1.0*0.3+1.0*0.2)*100=75
    }

    #[test]
    fn test_full_population_pipeline() {
        let result = full_population_pipeline(
            5000.0, &[1.0, 0.5, 0.0], 0.01,
            Some(&[15.0, 25.0, 5.0]), 0.5,
            Some(&[0.3, 0.1, 0.0]), Some(&[0.6, 0.2, 0.0]),
        );
        assert!((result.total_population - 5000.0).abs() < 1e-6);
        assert!(result.gdp_grid.is_some());
        assert_eq!(result.population_grid.len(), 3);
    }
}
