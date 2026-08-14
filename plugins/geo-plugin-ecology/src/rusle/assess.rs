use super::erosion_class::ErosionClass;
use super::factors::{
    compute_c_factor_from_ndvi, compute_ls_from_dem, compute_p_factor, compute_slope_from_dem,
    compute_soil_loss,
};
use super::practice::PracticeType;
use serde::{Deserialize, Serialize};

/// RUSLE 土壤流失评估结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RusleAssessment {
    /// R 因子值
    pub r_factor: f64,
    /// K 因子值（均值）
    pub k_factor_mean: f64,
    /// LS 因子值（均值）
    pub ls_factor_mean: f64,
    /// C 因子值（均值）
    pub c_factor_mean: f64,
    /// P 因子值（均值）
    pub p_factor_mean: f64,
    /// 年均土壤流失量 (t/ha/yr)
    pub soil_loss_mean: f64,
    /// 土壤流失总量 (t/yr)
    pub soil_loss_total: f64,
    /// 评估面积 (ha)
    pub area_ha: f64,
    /// 侵蚀等级分布
    pub class_distribution: Vec<(ErosionClass, f64)>,
    /// 各像素的土壤流失量 (t/ha/yr)
    pub soil_loss_grid: Vec<f64>,
}

/// 完整的 RUSLE 土壤流失评估。
///
/// # 参数
///
/// * `dem` — DEM 高程数组
/// * `slope_deg` — 坡度 (°)（可选；如为 None 则从 DEM 计算）
/// * `cellsize_m` — 像元大小 (m)
/// * `rows`, `cols` — 栅格尺寸
/// * `r_factor` — R 因子标量值
/// * `k_factor_grid` — K 因子栅格（可选标量扩展）
/// * `ndvi` — NDVI 栅格（用于 C 因子）
/// * `practice` — 水土保持措施类型
#[allow(clippy::too_many_arguments)]
pub fn assess_soil_loss(
    dem: &[f64],
    slope_deg: Option<&[f64]>,
    cellsize_m: f64,
    rows: usize,
    cols: usize,
    r_factor: f64,
    k_factor_grid: Option<&[f64]>,
    ndvi: &[f64],
    practice: PracticeType,
) -> RusleAssessment {
    let n = rows * cols;
    let area_cell_ha = cellsize_m * cellsize_m / 10000.0;
    let area_ha = n as f64 * area_cell_ha;

    // 坡度
    let slope = match slope_deg {
        Some(s) => s.to_vec(),
        None => compute_slope_from_dem(dem, cellsize_m, rows, cols),
    };

    // LS 因子
    let ls = compute_ls_from_dem(dem, cellsize_m, rows, cols);

    // K 因子
    let k = resolve_k_factor(k_factor_grid, n);

    // C 因子
    let c = compute_c_factor_from_ndvi(ndvi);

    // P 因子
    let slope_pct: Vec<f64> = slope
        .iter()
        .map(|&deg| deg.to_radians().tan() * 100.0)
        .collect();
    let p = compute_p_factor(&slope_pct, practice);

    // R 因子数组（标量扩展）
    let r_arr = vec![r_factor; n];

    // A = R × K × LS × C × P
    let soil_loss = compute_soil_loss(&r_arr, &k, &ls, &c, &p, n);

    compute_erosion_statistics(r_factor, area_cell_ha, area_ha, &k, &ls, &c, &p, soil_loss)
}

/// Resolve K factor grid with fallback to default silt-loam (0.032).
fn resolve_k_factor(k_factor_grid: Option<&[f64]>, n: usize) -> Vec<f64> {
    match k_factor_grid {
        Some(g) => {
            if g.len() >= n {
                g[..n].to_vec()
            } else {
                let fill = g.first().copied().unwrap_or(0.032);
                vec![fill; n]
            }
        }
        None => vec![0.032; n], // 默认粉砂壤土 K 值
    }
}

/// Compute erosion statistics and build the final RusleAssessment.
#[allow(clippy::too_many_arguments)]
fn compute_erosion_statistics(
    r_factor: f64,
    area_cell_ha: f64,
    area_ha: f64,
    k: &[f64],
    ls: &[f64],
    c: &[f64],
    p: &[f64],
    soil_loss: Vec<f64>,
) -> RusleAssessment {
    let n = soil_loss.len();
    let mean_loss = if n > 0 {
        soil_loss.iter().sum::<f64>() / n as f64
    } else {
        0.0
    };
    let total_loss = if n > 0 {
        soil_loss.iter().sum::<f64>() * area_cell_ha
    } else {
        0.0
    };

    // 侵蚀等级分布
    let classes = [
        ErosionClass::Slight,
        ErosionClass::Moderate,
        ErosionClass::High,
        ErosionClass::Severe,
        ErosionClass::VerySevere,
    ];
    let mut class_dist = Vec::with_capacity(classes.len());
    for &cls in &classes {
        let count = soil_loss
            .iter()
            .filter(|&&v| ErosionClass::from_rate(v) == cls)
            .count();
        let pct = if n > 0 {
            count as f64 / n as f64 * 100.0
        } else {
            0.0
        };
        class_dist.push((cls, pct));
    }

    fn factor_mean(factors: &[f64]) -> f64 {
        if factors.is_empty() {
            0.0
        } else {
            factors.iter().sum::<f64>() / factors.len() as f64
        }
    }

    RusleAssessment {
        r_factor,
        k_factor_mean: factor_mean(k),
        ls_factor_mean: factor_mean(ls),
        c_factor_mean: factor_mean(c),
        p_factor_mean: factor_mean(p),
        soil_loss_mean: mean_loss,
        soil_loss_total: total_loss,
        area_ha,
        class_distribution: class_dist,
        soil_loss_grid: soil_loss,
    }
}
