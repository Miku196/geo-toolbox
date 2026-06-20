//! RUSLE — 修正通用土壤流失方程。
//!
//! `A = R × K × LS × C × P`
//!
//! | 因子 | 含义 | 单位 |
//! |------|------|------|
//! | R    | 降雨侵蚀力 | MJ·mm/ha·h·yr |
//! | K    | 土壤可蚀性 | t·ha·h/ha·MJ·mm |
//! | LS   | 坡长-坡度 | 无量纲 |
//! | C    | 覆盖管理 | 无量纲 (0-1) |
//! | P    | 水土保持措施 | 无量纲 (0-1) |
//! | A    | 年均土壤流失量 | t/ha/yr |

use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────
// 侵蚀严重等级
// ──────────────────────────────────────────────

/// 土壤侵蚀严重等级（t/ha/yr）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErosionClass {
    /// 微度 < 5
    Slight,
    /// 轻度 5-10
    Moderate,
    /// 中度 10-20
    High,
    /// 强烈 20-50
    Severe,
    /// 极强烈 > 50
    VerySevere,
}

impl ErosionClass {
    pub fn from_rate(t_per_ha_yr: f64) -> Self {
        if t_per_ha_yr < 5.0 {
            Self::Slight
        } else if t_per_ha_yr < 10.0 {
            Self::Moderate
        } else if t_per_ha_yr < 20.0 {
            Self::High
        } else if t_per_ha_yr < 50.0 {
            Self::Severe
        } else {
            Self::VerySevere
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Slight => "微度",
            Self::Moderate => "轻度",
            Self::High => "中度",
            Self::Severe => "强烈",
            Self::VerySevere => "极强烈",
        }
    }
}

// ──────────────────────────────────────────────
// 水土保持措施类型
// ──────────────────────────────────────────────

/// 水土保持措施类型（用于 P 因子计算）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PracticeType {
    /// 无措施
    None,
    /// 等高耕作
    Contouring,
    /// 等高带状种植
    StripCropping,
    /// 梯田
    Terracing,
}

// ──────────────────────────────────────────────
// 土壤流失评估结果
// ──────────────────────────────────────────────

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

// ──────────────────────────────────────────────
// 核心函数
// ──────────────────────────────────────────────

/// 计算 R 因子（降雨侵蚀力）- Renard-Freimund 月尺度法。
///
/// 使用 Renard & Freimund (1994) 经验公式，基于 Modified Fournier Index (MFI)：
/// ```text
/// MFI = sum(P_i^2 / P_annual)     (i = 1..12月)
/// MFI < 55:  R = 0.7397 * MFI^1.847
/// MFI >= 55: R = 95.77 - 6.081*MFI + 0.477*MFI^2
/// ```
/// 其中 `P_i` 为月降雨量 (mm)，`P_annual` 为年降雨量 (mm)。
/// 多年数据返回多年平均 R 因子 [MJ·mm/ha·h·yr]。
///
/// **注意**：此公式适用于月降雨数据；如需更精确的 R 因子，应使用逐暴雨 EI30 法。
///
/// **来源**: Renard, K.G. & Freimund, J.R. (1994).
/// "Using monthly precipitation data to estimate the R-factor in the revised USLE".
/// Journal of Hydrology, 157(1-4): 287-306. doi:10.1016/0022-1694(94)90110-4
pub fn compute_r_factor(monthly_rainfall_mm: &[&[f64]]) -> f64 {
    let n_years = monthly_rainfall_mm.len();
    if n_years == 0 {
        return 0.0;
    }
    let mut r_sum = 0.0;
    for year_data in monthly_rainfall_mm {
        if year_data.len() < 12 {
            continue;
        }
        let annual: f64 = year_data.iter().sum();
        if annual <= 0.0 {
            continue;
        }
        // 计算 Modified Fournier Index (MFI)
        let mfi: f64 = year_data
            .iter()
            .filter(|&&p| p > 0.0)
            .map(|&p| p * p / annual)
            .sum();

        let year_r = if mfi < 55.0 {
            0.7397 * mfi.powf(1.847)
        } else {
            95.77 - 6.081 * mfi + 0.477 * mfi.powi(2)
        };
        r_sum += year_r.max(0.0);
    }
    r_sum / n_years as f64
}

/// 简化的 R 因子估算：仅用年降雨量。
///
/// `R = 0.0483 × P^1.61` (P 为年均降雨量 mm，适用于中国湿润/半湿润区)
/// 来源: 周伏建等 (1989). "福建省降雨侵蚀力指标R值". 福建水土保持, (1): 32-37.
pub fn compute_r_factor_simple(annual_rainfall_mm: f64) -> f64 {
    if annual_rainfall_mm <= 0.0 {
        return 0.0;
    }
    0.0483 * annual_rainfall_mm.powf(1.61)
}

/// 计算 K 因子（土壤可蚀性）。
///
/// 使用 Wischmeier & Smith 诺模公式：
/// `K = 2.1 × M^1.14 × 10⁻⁴ × (12 - OM) + 0.0325 × (b - 2) + 0.025 × (c - 3)`
///
/// 其中：
/// - M = (粉粒% + 极细砂%) × (100 - 粘粒%)
/// - OM = 有机质 (%)
/// - b = 结构代码 (1-4)
/// - c = 渗透性等级 (1-6)
///
/// **参数说明：**
/// - `sand_pct` — 砂粒含量 (%)，2.0-0.05 mm
/// - `silt_pct` — 粉粒含量 (%)，0.05-0.002 mm
/// - `clay_pct` — 粘粒含量 (%)，<0.002 mm
/// - `very_fine_sand_pct` — 极细砂 (%)，0.1-0.05 mm（若无数据则取砂粒的 1/3 估算）
/// - `om_pct` — 有机质含量 (%)
/// - `structure_code` — 土壤结构代码 (1=块粒, 2=细团粒, 3=中粗团粒, 4=块状/板状)
/// - `permeability_code` — 渗透性等级 (1=快, 6=极慢)
pub fn compute_k_factor(
    _sand_pct: f64,
    silt_pct: f64,
    clay_pct: f64,
    very_fine_sand_pct: f64,
    om_pct: f64,
    structure_code: u32,
    permeability_code: u32,
) -> f64 {
    let m = (silt_pct + very_fine_sand_pct) * (100.0 - clay_pct);
    let om_factor = (12.0 - om_pct).max(0.0);
    let s_code = structure_code.clamp(1, 4);
    let p_code = permeability_code.clamp(1, 6);

    let k = 0.1317
        * (2.1e-4 * m.powf(1.14) * om_factor / 100.0
            + 0.0325 * (s_code as f64 - 2.0)
            + 0.025 * (p_code as f64 - 3.0));

    // K 因子取值范围通常为 0-0.7，截断异常值
    k.clamp(0.0, 0.7)
}

/// 简化的 K 因子估算（仅用土壤质地）。
///
/// 使用修正的 EPIC 公式：
/// `K = 0.1317 × (0.2 + 0.3 × exp(-0.0256 × SAN × (1 - SIL/100)))`
///   × `(SIL/(CLA+SIL))^0.3`
///   × `(1 - 0.25 × C / (C + exp(3.72 - 2.95 × C)))`
///   × `(1 - 0.7 × SN₁ / (SN₁ + exp(-5.51 + 22.9 × SN₁)))`
///
/// 其中 SAN = 砂粒%, SIL = 粉粒%, CLA = 粘粒%, C = 有机碳%, SN₁ = 1 - SAN/100
pub fn compute_k_factor_simple(sand_pct: f64, silt_pct: f64, clay_pct: f64, om_pct: f64) -> f64 {
    let san = sand_pct;
    let sil = silt_pct;
    let cla = clay_pct;
    let c = om_pct * 0.58; // 有机碳 = 有机质 × 0.58 (Van Bemmelen 系数)
    let sn1 = 1.0 - san / 100.0;

    let f1 = 0.2 + 0.3 * (-0.0256 * san * (1.0 - sil / 100.0)).exp();
    let f2 = (sil / (cla + sil)).powf(0.3);
    let f3 = 1.0 - 0.25 * c / (c + (3.72 - 2.95 * c).exp());
    let f4 = 1.0 - 0.7 * sn1 / (sn1 + (-5.51 + 22.9 * sn1).exp());

    let k = 0.1317 * f1 * f2 * f3 * f4;
    k.clamp(0.0, 0.7)
}

/// 计算 LS 因子（坡长-坡度因子）。
///
/// 使用 Wischmeier & Smith 公式的 McCool 改进版。
///
/// - `slope_deg`: 坡度 (°)
/// - `slope_length_m`: 坡长 (m)
/// - `rows`: 行数
/// - `cols`: 列数
pub fn compute_ls_factor(
    slope_deg: &[f64],
    slope_length_m: f64,
    rows: usize,
    cols: usize,
) -> Vec<f64> {
    let n = rows * cols;
    let actual_len = slope_deg.len().min(n);
    let mut ls = vec![0.0; n];

    for i in 0..actual_len {
        let angle_rad = slope_deg[i].to_radians();
        let sin_theta = angle_rad.sin();
        let slope_pct = (angle_rad.tan()) * 100.0;

        if slope_pct <= 0.0 {
            ls[i] = 0.0;
            continue;
        }

        // m 指数
        let m = if slope_pct < 1.0 {
            0.2
        } else if slope_pct < 3.0 {
            0.3
        } else if slope_pct < 5.0 {
            0.4
        } else {
            0.5
        };

        // L 因子
        let l_factor = (slope_length_m / 22.13).powf(m);

        // S 因子 (McCool, 1987)
        let s_factor = if slope_pct < 9.0 {
            10.8 * sin_theta + 0.03
        } else {
            16.8 * sin_theta - 0.50
        };

        ls[i] = l_factor * s_factor;
    }

    ls
}

/// 从 DEM 和坡长计算 LS 因子栅格。
///
/// - `dem`: DEM 高程数组
/// - `cellsize_m`: 像元大小 (m)
/// - `rows`: 行数
/// - `cols`: 列数
/// - `slope_length_m`: 标准坡长 (默认 22.13 m 为标准坡长)
pub fn compute_ls_from_dem(dem: &[f64], cellsize_m: f64, rows: usize, cols: usize) -> Vec<f64> {
    let n = rows * cols;
    if dem.len() < n {
        return vec![0.0; n];
    }

    // 用最大下坡差分计算坡度
    let slope_deg = compute_slope_from_dem(dem, cellsize_m, rows, cols);
    let slope_length_m = cellsize_m; // 坡长近似为像元大小 × 汇流面积系数
    compute_ls_factor(&slope_deg, slope_length_m, rows, cols)
}

/// 从 DEM 计算坡度 (°)。
pub fn compute_slope_from_dem(dem: &[f64], cellsize_m: f64, rows: usize, cols: usize) -> Vec<f64> {
    let n = rows * cols;
    let mut slope = vec![0.0; n];
    if dem.len() < n {
        return slope;
    }

    let idx = |r: usize, c: usize| -> usize { r * cols + c };

    for r in 0..rows {
        for c in 0..cols {
            let mut dz_dx = 0.0;
            let mut dz_dy = 0.0;

            // 3x3 窗口计算坡度和坡向 (Horn, 1981)
            let has_left = c > 0;
            let has_right = c + 1 < cols;
            let has_up = r > 0;
            let has_down = r + 1 < rows;

            if has_left && has_right {
                if has_up {
                    let w = dem[idx(r - 1, c - 1)];
                    let e = dem[idx(r - 1, c + 1)];
                    dz_dx += (e - w) * 1.0; // weight 1
                    dz_dy += (dem[idx(r - 1, c)] - dem[idx(r, c)]) * 0.5;
                }
                if has_down {
                    let w = dem[idx(r + 1, c - 1)];
                    let e = dem[idx(r + 1, c + 1)];
                    dz_dx += (e - w) * 1.0;
                }
                let w = dem[idx(r, c - 1)];
                let e = dem[idx(r, c + 1)];
                dz_dx += (e - w) * 2.0;

                dz_dx /= 8.0 * cellsize_m;
            }

            if has_up && has_down {
                if has_left {
                    dz_dy += (dem[idx(r - 1, c - 1)] - dem[idx(r + 1, c - 1)]) * 1.0;
                }
                if has_right {
                    dz_dy += (dem[idx(r - 1, c + 1)] - dem[idx(r + 1, c + 1)]) * 1.0;
                }
                let n = dem[idx(r - 1, c)];
                let s = dem[idx(r + 1, c)];
                dz_dy += (n - s) * 2.0;

                dz_dy /= 8.0 * cellsize_m;
            }

            slope[idx(r, c)] = (dz_dx * dz_dx + dz_dy * dz_dy).sqrt().atan().to_degrees();
        }
    }

    slope
}

/// 计算 C 因子（覆盖管理因子）。
///
/// 使用 NDVI 经验公式：
/// `C = exp(-2 × NDVI / (1 - NDVI))` (Van der Knijff, 1999)
///
/// 或基于土地利用分类的查表法。
pub fn compute_c_factor_from_ndvi(ndvi: &[f64]) -> Vec<f64> {
    ndvi.iter()
        .map(|&ndvi_val| {
            // NDVI 接近或小于 0 → C ≈ 1.0（裸土）
            if ndvi_val <= 0.0 {
                return 1.0;
            }
            // NDVI 接近 1 → C ≈ 0（完全覆盖）
            if ndvi_val >= 1.0 {
                return 0.001; // 避免除零，极小值
            }
            let ratio = -2.0 * ndvi_val / (1.0 - ndvi_val);
            ratio.exp()
        })
        .collect()
}

/// 基于土地利用类型的 C 因子查表。
pub fn c_factor_for_landuse(code: &str) -> f64 {
    match code {
        "forest" | "林地" => 0.005,
        "shrub" | "灌木" => 0.02,
        "grass" | "草地" => 0.05,
        "cropland" | "耕地" | "农田" => 0.25,
        "rice" | "水田" => 0.15,
        "orchard" | "果园" => 0.20,
        "bare" | "裸地" | "bareland" => 1.0,
        "urban" | "建设用地" | "built-up" => 0.01,
        "water" | "水域" => 0.0,
        "wetland" | "湿地" => 0.0,
        "mining" | "采矿用地" => 0.8,
        _ => 0.15, // 默认耕地
    }
}

/// 计算 P 因子（水土保持措施因子）。
///
/// 基于坡度和措施类型查表（Wischmeier & Smith, 1978）。
pub fn compute_p_factor(slope_pct: &[f64], practice: PracticeType) -> Vec<f64> {
    slope_pct
        .iter()
        .map(|&s| match practice {
            PracticeType::None => 1.0,
            PracticeType::Contouring => {
                if s < 1.0 {
                    0.60
                } else if s < 2.0 {
                    0.50
                } else if s < 5.0 {
                    0.45
                } else if s < 8.0 {
                    0.50
                } else if s < 12.0 {
                    0.60
                } else if s < 16.0 {
                    0.70
                } else if s < 20.0 {
                    0.80
                } else {
                    0.90
                }
            }
            PracticeType::StripCropping => {
                if s < 1.0 {
                    0.45
                } else if s < 2.0 {
                    0.40
                } else if s < 5.0 {
                    0.35
                } else if s < 8.0 {
                    0.40
                } else if s < 12.0 {
                    0.45
                } else if s < 16.0 {
                    0.55
                } else {
                    0.65
                }
            }
            PracticeType::Terracing => {
                if s < 1.0 {
                    0.20
                } else if s < 2.0 {
                    0.15
                } else if s < 5.0 {
                    0.12
                } else if s < 8.0 {
                    0.15
                } else if s < 12.0 {
                    0.20
                } else if s < 16.0 {
                    0.25
                } else {
                    0.35
                }
            }
        })
        .collect()
}

/// 计算最终土壤流失量。
///
/// 接受等长的一维数组，输出 `A = R × K × LS × C × P`。
/// 对于标量因子（R），扩展到数组长度。`cells` 指定输出数组长度。
pub fn compute_soil_loss(
    r_factor: &[f64],
    k_factor: &[f64],
    ls_factor: &[f64],
    c_factor: &[f64],
    p_factor: &[f64],
    cells: usize,
) -> Vec<f64> {
    let mut loss = vec![0.0; cells];
    for (i, l) in loss.iter_mut().enumerate() {
        let r = r_factor.get(i).copied().unwrap_or(0.0);
        let k = k_factor.get(i).copied().unwrap_or(0.0);
        let ls = ls_factor.get(i).copied().unwrap_or(0.0);
        let c = c_factor.get(i).copied().unwrap_or(0.0);
        let p = p_factor.get(i).copied().unwrap_or(0.0);
        *l = r * k * ls * c * p;
    }
    loss
}

/// 计算 MUSLE（Modified Universal Soil Loss Equation）— 单场暴雨产沙量。
///
/// MUSLE 用径流因子替代 RUSLE 的降雨侵蚀力 R 因子，
/// 适用于单场暴雨的泥沙产量估算。
/// `Y = 11.8 × (Q × q_p)^0.56 × K × LS × C × P`
///
/// 其中 `Q` 为径流深 (mm，来自 SCS-CN)，`q_p` 为洪峰流量 (mm/h)，
/// K/LS/C/P 与 RUSLE 相同。返回 t/ha（吨/公顷）。
///
/// **来源**: Williams, J.R. (1975).
/// "Sediment-yield prediction with Universal Equation using runoff energy factor".
/// USDA-ARS, ARS-S-40, pp. 244-252.
///
/// **注意**: MUSLE 是事件模型，适用于单次暴雨。
/// 常数 11.8 将单位转换为 (t·ha⁻¹)。
pub fn compute_musle_sediment(
    runoff_depth_mm: &[f64],
    peak_runoff_rate_mm_h: &[f64],
    k_factor: &[f64],
    ls_factor: &[f64],
    c_factor: &[f64],
    p_factor: &[f64],
    cells: usize,
) -> Vec<f64> {
    let mut sediment = vec![0.0; cells];
    for (i, s) in sediment.iter_mut().enumerate() {
        let q = runoff_depth_mm.get(i).copied().unwrap_or(0.0);
        let qp = peak_runoff_rate_mm_h.get(i).copied().unwrap_or(0.0);
        let k = k_factor.get(i).copied().unwrap_or(0.0);
        let ls = ls_factor.get(i).copied().unwrap_or(0.0);
        let c = c_factor.get(i).copied().unwrap_or(0.0);
        let p = p_factor.get(i).copied().unwrap_or(0.0);

        if q <= 0.0 || qp <= 0.0 {
            continue;
        }
        let energy = (q * qp).powf(0.56);
        *s = 11.8 * energy * k * ls * c * p;
    }
    sediment
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
    let k: Vec<f64> = match k_factor_grid {
        Some(g) => {
            if g.len() >= n {
                g[..n].to_vec()
            } else {
                let fill = g.first().copied().unwrap_or(0.032);
                vec![fill; n]
            }
        }
        None => vec![0.032; n], // 默认粉砂壤土 K 值
    };

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

    // 统计
    let mean_loss = if n > 0 {
        soil_loss.iter().sum::<f64>() / n as f64
    } else {
        0.0
    };
    let total_loss = soil_loss.iter().sum::<f64>() * area_cell_ha;

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

    RusleAssessment {
        r_factor,
        k_factor_mean: if n > 0 {
            k.iter().sum::<f64>() / n as f64
        } else {
            0.0
        },
        ls_factor_mean: if n > 0 {
            ls.iter().sum::<f64>() / n as f64
        } else {
            0.0
        },
        c_factor_mean: if n > 0 {
            c.iter().sum::<f64>() / n as f64
        } else {
            0.0
        },
        p_factor_mean: if n > 0 {
            p.iter().sum::<f64>() / n as f64
        } else {
            0.0
        },
        soil_loss_mean: mean_loss,
        soil_loss_total: total_loss,
        area_ha,
        class_distribution: class_dist,
        soil_loss_grid: soil_loss,
    }
}

// ──────────────────────────────────────────────
// 测试
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn test_erosion_class() {
        assert_eq!(ErosionClass::from_rate(0.0), ErosionClass::Slight);
        assert_eq!(ErosionClass::from_rate(4.9), ErosionClass::Slight);
        assert_eq!(ErosionClass::from_rate(5.0), ErosionClass::Moderate);
        assert_eq!(ErosionClass::from_rate(7.5), ErosionClass::Moderate);
        assert_eq!(ErosionClass::from_rate(10.0), ErosionClass::High);
        assert_eq!(ErosionClass::from_rate(20.0), ErosionClass::Severe);
        assert_eq!(ErosionClass::from_rate(50.0), ErosionClass::VerySevere);
        assert_eq!(ErosionClass::from_rate(100.0), ErosionClass::VerySevere);
        assert_eq!(ErosionClass::Slight.label(), "微度");
        assert_eq!(ErosionClass::VerySevere.label(), "极强烈");
    }

    #[test]
    fn test_r_factor_simple() {
        let r = compute_r_factor_simple(1000.0);
        assert!(r > 3000.0 && r < 4000.0);
    }

    #[test]
    fn test_r_factor_monthly() {
        // 模拟典型南方红壤区月降雨 (mm)
        let monthly: Vec<Vec<f64>> = vec![
            vec![
                50.0, 60.0, 100.0, 150.0, 200.0, 250.0, 200.0, 180.0, 120.0, 80.0, 60.0, 40.0,
            ],
            vec![
                45.0, 55.0, 95.0, 140.0, 190.0, 240.0, 210.0, 170.0, 110.0, 75.0, 55.0, 38.0,
            ],
            vec![
                55.0, 65.0, 105.0, 160.0, 210.0, 260.0, 190.0, 190.0, 130.0, 85.0, 65.0, 42.0,
            ],
        ];
        let refs: Vec<&[f64]> = monthly.iter().map(|v| v.as_slice()).collect();
        let r = compute_r_factor(&refs);
        assert!(r > 0.0);
        assert!(r < 50000.0);
    }

    #[test]
    fn test_r_factor_empty() {
        assert_eq!(compute_r_factor(&[]), 0.0);
        assert_eq!(compute_r_factor_simple(0.0), 0.0);
    }

    #[test]
    fn test_k_factor() {
        // 粉砂壤土: 砂20% 粉65% 粘15%
        let k = compute_k_factor(20.0, 65.0, 15.0, 7.0, 2.0, 2, 3);
        assert!(k > 0.001 && k < 0.7);
    }

    #[test]
    fn test_k_factor_simple() {
        // 粉砂壤土简化计算
        let k = compute_k_factor_simple(20.0, 65.0, 15.0, 2.0);
        assert!(k > 0.01 && k < 0.5);
    }

    #[test]
    fn test_c_factor_from_ndvi() {
        let ndvi = vec![0.0, 0.3, 0.5, 0.7, 1.0, -0.1];
        let c = compute_c_factor_from_ndvi(&ndvi);
        // NDVI=0 → C≈1.0
        assert!(approx_eq(c[0], 1.0, 1e-6));
        // NDVI=1 → C≈0.001
        assert!(c[4] < 0.01);
        // NDVI<0 → C=1.0
        assert!(approx_eq(c[5], 1.0, 1e-6));
        // NDVI 越高，C 越低
        assert!(c[1] > c[2]);
        assert!(c[2] > c[3]);
    }

    #[test]
    fn test_c_factor_landuse() {
        assert!(approx_eq(c_factor_for_landuse("forest"), 0.005, 1e-6));
        assert!(approx_eq(c_factor_for_landuse("bare"), 1.0, 1e-6));
        assert!(approx_eq(c_factor_for_landuse("裸地"), 1.0, 1e-6));
        assert!(approx_eq(c_factor_for_landuse("water"), 0.0, 1e-6));
        assert!(c_factor_for_landuse("unknown") > 0.0);
    }

    #[test]
    fn test_p_factor() {
        let slopes = vec![0.5, 3.0, 10.0, 25.0];
        let p = compute_p_factor(&slopes, PracticeType::Contouring);
        assert!(p.iter().all(|&v| v > 0.0 && v <= 1.0));
        // 梯田 P 因子小于等高耕作
        let p_t = compute_p_factor(&slopes, PracticeType::Terracing);
        for i in 0..slopes.len() {
            assert!(p_t[i] <= p[i]);
        }
    }

    #[test]
    fn test_slope_from_dem() {
        // 平坦 DEM → 坡度 ≈ 0
        let dem = vec![100.0; 9];
        let slope = compute_slope_from_dem(&dem, 10.0, 3, 3);
        assert!(slope.iter().all(|&s| s < 0.001));

        // 斜坡 DEM
        let dem2 = vec![100.0, 100.0, 100.0, 100.0, 95.0, 90.0, 100.0, 95.0, 90.0];
        let slope2 = compute_slope_from_dem(&dem2, 10.0, 3, 3);
        // 中心像元应有正坡度
        assert!(slope2[4] > 0.0);
    }

    #[test]
    fn test_ls_factor() {
        // 平坦 → LS = 0
        let flat = vec![0.0; 4];
        let ls = compute_ls_factor(&flat, 22.13, 2, 2);
        assert!(ls.iter().all(|&v| v == 0.0));

        // 坡度 10° → LS > 0
        let sloped = vec![10.0; 4];
        let ls2 = compute_ls_factor(&sloped, 22.13, 2, 2);
        assert!(ls2.iter().all(|&v| v > 0.0));
    }

    #[test]
    fn test_compute_soil_loss() {
        let n = 6;
        let r = vec![5000.0; n];
        let k = vec![0.04; n];
        let ls = vec![1.0; n];
        let c = vec![0.2; n];
        let p = vec![1.0; n];
        let loss = compute_soil_loss(&r, &k, &ls, &c, &p, n);
        // 5000 × 0.04 × 1.0 × 0.2 × 1.0 = 40
        assert!(approx_eq(loss[0], 40.0, 1e-6));
        // 长度不足的元素填 0
        let r2 = vec![5000.0; 3];
        let loss2 = compute_soil_loss(&r2, &k, &ls, &c, &p, n);
        assert!(approx_eq(loss2[0], 40.0, 1e-6));
        assert_eq!(loss2[5], 0.0);
    }

    #[test]
    fn test_musle_sediment() {
        let q = vec![30.0, 0.0, 50.0];
        let qp = vec![10.0, 20.0, 0.0];
        let k = vec![0.04, 0.04, 0.04];
        let ls = vec![1.0, 1.0, 1.0];
        let c = vec![0.2, 0.2, 0.2];
        let p = vec![1.0, 1.0, 1.0];
        let sed = compute_musle_sediment(&q, &qp, &k, &ls, &c, &p, 3);
        // cell 0: 11.8 × (30×10)^0.56 × 0.04 × 1.0 × 0.2 × 1.0 > 0
        assert!(sed[0] > 0.0, "cell0 should have sediment, got {}", sed[0]);
        // cell 1: Q=0 → sediment=0
        assert_eq!(sed[1], 0.0);
        // cell 2: qp=0 → sediment=0
        assert_eq!(sed[2], 0.0);
        // Running 11.8 × (30×10)^0.56 × 0.04 × 0.2
        // = 11.8 × (300)^0.56 × 0.008
        let expected = 11.8 * (300.0_f64).powf(0.56) * 0.04 * 0.2;
        assert!(
            (sed[0] - expected).abs() < 1e-6,
            "sed[0]={} expected={}",
            sed[0],
            expected
        );
    }

    #[test]
    fn test_assess_soil_loss_flat() {
        let dem = vec![100.0; 9];
        let ndvi = vec![0.5; 9];
        let result = assess_soil_loss(
            &dem,
            None,
            30.0,
            3,
            3,
            4000.0,
            None,
            &ndvi,
            PracticeType::None,
        );

        // 平坦 → LS ≈ 0 → loss ≈ 0
        assert!(result.soil_loss_mean < 1.0);
        assert!(approx_eq(result.area_ha, 0.81, 1e-4)); // 0.81 ha
                                                        // 100% 微度
        let slight_pct = result
            .class_distribution
            .iter()
            .find(|(c, _)| *c == ErosionClass::Slight)
            .map(|(_, p)| *p)
            .unwrap_or(0.0);
        assert!(approx_eq(slight_pct, 100.0, 1e-3));
    }

    #[test]
    fn test_assess_soil_loss_steep() {
        // 陡坡 DEM（八字形：中间低，四周高）
        let dem = vec![
            120.0, 110.0, 120.0, 110.0, 100.0, 110.0, 120.0, 110.0, 120.0,
        ];
        let ndvi = vec![0.3; 9];
        let result = assess_soil_loss(
            &dem,
            None,
            10.0,
            3,
            3,
            5000.0,
            None,
            &ndvi,
            PracticeType::None,
        );

        // 有坡度 → 应有土壤流失
        assert!(result.soil_loss_mean > 0.0);
        assert!(result.ls_factor_mean > 0.0);
        assert!(result.soil_loss_total > 0.0);
    }

    #[test]
    fn test_assess_with_terracing() {
        let dem = vec![
            120.0, 110.0, 120.0, 110.0, 100.0, 110.0, 120.0, 110.0, 120.0,
        ];
        let ndvi = vec![0.3; 9];

        let result_none = assess_soil_loss(
            &dem,
            None,
            10.0,
            3,
            3,
            5000.0,
            None,
            &ndvi,
            PracticeType::None,
        );
        let result_terrace = assess_soil_loss(
            &dem,
            None,
            10.0,
            3,
            3,
            5000.0,
            None,
            &ndvi,
            PracticeType::Terracing,
        );

        // 梯田 P 因子低 → 土壤流失应更少
        assert!(result_terrace.soil_loss_mean < result_none.soil_loss_mean);
        assert!(result_terrace.p_factor_mean < result_none.p_factor_mean);
    }

    #[test]
    fn test_k_factor_simple_typical() {
        // 典型中国南方红壤
        let k = compute_k_factor_simple(35.0, 40.0, 25.0, 1.5);
        assert!(k > 0.01 && k < 0.4);

        // 砂土 — K 值较低
        let k_sand = compute_k_factor_simple(80.0, 10.0, 10.0, 0.5);
        assert!(k_sand < 0.2);

        // 粘土 — K 值中等
        let k_clay = compute_k_factor_simple(20.0, 20.0, 60.0, 3.0);
        assert!(k_clay > 0.01 && k_clay < 0.5);
    }
}
