use super::practice::PracticeType;

// ──────────────────────────────────────────────
// 核心因子计算函数
// ──────────────────────────────────────────────

/// 计算 R 因子（降雨侵蚀力）- Renard-Freimund 月尺度法。
///
/// 使用 Renard & Freimund (1994) 经验公式，基于 Modified Fournier Index (MFI)：
/// \`\`\`text
/// MFI = sum(P_i^2 / P_annual)     (i = 1..12月)
/// MFI < 55:  R = 0.7397 * MFI^1.847
/// MFI >= 55: R = 95.77 - 6.081*MFI + 0.477*MFI^2
/// \`\`\`
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
/// \`K = 0.1317 × (0.2 + 0.3 × exp(-0.0256 × SAN × (1 - SIL/100)))\`
///   × \`(SIL/(CLA+SIL))^0.3\`
///   × \`(1 - 0.25 × C / (C + exp(3.72 - 2.95 × C)))\`
///   × \`(1 - 0.7 × SN₁ / (SN₁ + exp(-5.51 + 22.9 × SN₁)))\`
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
                    0.30
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
