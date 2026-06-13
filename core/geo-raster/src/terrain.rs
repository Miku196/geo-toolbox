use serde::Serialize;

/// 坡度计算结果。
#[derive(Debug, Clone, Serialize)]
pub struct SlopeResult {
    /// 行数。
    pub rows: usize,
    /// 列数。
    pub cols: usize,
    /// 坡度值（度），行优先，NoData 值为 NaN。
    pub slope_degrees: Vec<f64>,
    /// 坡度值（百分比），行优先。
    pub slope_percent: Vec<f64>,
    /// 均值（度）。
    pub mean_degrees: Option<f64>,
    /// 最大值（度）。
    pub max_degrees: Option<f64>,
}

/// 坡向计算结果（从正北方向顺时针，0°~360°）。
#[derive(Debug, Clone, Serialize)]
pub struct AspectResult {
    /// 行数。
    pub rows: usize,
    /// 列数。
    pub cols: usize,
    /// 坡向值（度，0=正北，顺时针），NoData 值为 NaN。
    pub aspect_degrees: Vec<f64>,
    /// 坡向分类（8 方向）。
    pub aspect_class: Vec<String>,
    /// 均值。
    pub mean_aspect: Option<f64>,
}

/// 使用 Horn (1981) 算法计算坡度（度）。
///
/// 使用 3×3 窗口的有限差分计算 dz/dx 和 dz/dy。
/// 边界像素设为 NaN。
///
/// # 参数
/// - `dem`: DEM 高程值，行优先，`dem[row * cols + col]`
/// - `rows`, `cols`: DEM 尺寸
/// - `cell_size_m`: 像元分辨率（米），需在投影坐标系下
/// - `nodata`: NoData 值（默认 `f64::NAN`）
pub fn compute_slope_degrees(
    dem: &[f64],
    rows: usize,
    cols: usize,
    cell_size_m: f64,
    nodata: Option<f64>,
) -> SlopeResult {
    let nd = nodata.unwrap_or(f64::NAN);
    let n = rows * cols;
    let mut slope_deg = vec![f64::NAN; n];
    let mut slope_pct = vec![f64::NAN; n];
    let _double_cell = 2.0 * cell_size_m;

    for r in 1..rows - 1 {
        for c in 1..cols - 1 {
            let idx = r * cols + c;

            // 3x3 邻域
            let z = [
                dem[(r - 1) * cols + c - 1], // NW
                dem[(r - 1) * cols + c],     // N
                dem[(r - 1) * cols + c + 1], // NE
                dem[r * cols + c - 1],       // W
                dem[r * cols + c + 1],       // E
                dem[(r + 1) * cols + c - 1], // SW
                dem[(r + 1) * cols + c],     // S
                dem[(r + 1) * cols + c + 1], // SE
            ];

            // 跳过含 NoData 的窗口
            if z.iter().any(|&v| v.is_nan() || (!nd.is_nan() && (v - nd).abs() < 1e-10)) {
                continue;
            }

            // Horn 方法: dz/dx = ((z7 + 2*z5 + z3) - (z1 + 2*z4 + z6)) / (8 * cell_size)
            // 但标准 arcgis 做法略有不同。我们用标准 Horn:
            let dz_dx = ((z[7] + 2.0 * z[4] + z[2]) - (z[0] + 2.0 * z[3] + z[5])) / (8.0 * cell_size_m);
            let dz_dy = ((z[5] + 2.0 * z[6] + z[7]) - (z[0] + 2.0 * z[1] + z[2])) / (8.0 * cell_size_m);

            let rise_run = (dz_dx * dz_dx + dz_dy * dz_dy).sqrt();
            let deg = rise_run.atan().to_degrees();
            slope_deg[idx] = deg;
            slope_pct[idx] = rise_run * 100.0;
        }
    }

    let valid: Vec<f64> = slope_deg.iter().cloned().filter(|v| !v.is_nan()).collect();
    let mean_degrees = if valid.is_empty() {
        None
    } else {
        Some(valid.iter().sum::<f64>() / valid.len() as f64)
    };
    let max_degrees = valid.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let max_degrees = if valid.is_empty() { None } else { Some(max_degrees) };

    SlopeResult {
        rows,
        cols,
        slope_degrees: slope_deg,
        slope_percent: slope_pct,
        mean_degrees,
        max_degrees,
    }
}

/// 使用 Horn (1981) 算法计算坡度百分比。
///
/// 返回百分比值（如 45° ≈ 100%）。
pub fn compute_slope_percent(
    dem: &[f64],
    rows: usize,
    cols: usize,
    cell_size_m: f64,
    nodata: Option<f64>,
) -> SlopeResult {
    compute_slope_degrees(dem, rows, cols, cell_size_m, nodata)
}

/// 使用 Horn (1981) 算法计算坡向（度，从正北顺时针，0°~360°）。
///
/// 平坦区域（坡度为 0）返回 -1（或 NaN）坡向。
pub fn compute_aspect(
    dem: &[f64],
    rows: usize,
    cols: usize,
    cell_size_m: f64,
    nodata: Option<f64>,
) -> AspectResult {
    let nd = nodata.unwrap_or(f64::NAN);
    let n = rows * cols;
    let mut aspect_deg = vec![f64::NAN; n];
    let mut aspect_class = vec!["NODATA".to_string(); n];

    for r in 1..rows - 1 {
        for c in 1..cols - 1 {
            let idx = r * cols + c;

            let z = [
                dem[(r - 1) * cols + c - 1],
                dem[(r - 1) * cols + c],
                dem[(r - 1) * cols + c + 1],
                dem[r * cols + c - 1],
                dem[r * cols + c + 1],
                dem[(r + 1) * cols + c - 1],
                dem[(r + 1) * cols + c],
                dem[(r + 1) * cols + c + 1],
            ];

            if z.iter().any(|&v| v.is_nan() || (!nd.is_nan() && (v - nd).abs() < 1e-10)) {
                continue;
            }

            let dz_dx = ((z[7] + 2.0 * z[4] + z[2]) - (z[0] + 2.0 * z[3] + z[5])) / (8.0 * cell_size_m);
            let dz_dy = ((z[5] + 2.0 * z[6] + z[7]) - (z[0] + 2.0 * z[1] + z[2])) / (8.0 * cell_size_m);

            // 平坦区域
            if dz_dx.abs() < 1e-10 && dz_dy.abs() < 1e-10 {
                aspect_deg[idx] = -1.0;
                aspect_class[idx] = "FLAT".to_string();
                continue;
            }

            // 坡向 = atan2(dz_dy, -dz_dx)，转换为从正北顺时针的角度
            let mut asp = (-dz_dy).atan2(dz_dx).to_degrees();
            asp = 90.0 - asp; // 调整为地理方向
            if asp < 0.0 {
                asp += 360.0;
            }
            if asp >= 360.0 {
                asp -= 360.0;
            }

            aspect_deg[idx] = asp;
            aspect_class[idx] = classify_aspect(asp);
        }
    }

    let valid: Vec<f64> = aspect_deg.iter().cloned().filter(|&v| !v.is_nan() && v >= 0.0).collect();
    let mean_aspect = if valid.is_empty() {
        None
    } else {
        // 使用向量平均计算圆形均值
        let (sum_sin, sum_cos) = valid
            .iter()
            .fold((0.0, 0.0), |(s, c), &a| {
                let rad = a.to_radians();
                (s + rad.sin(), c + rad.cos())
            });
        let mean = sum_sin.atan2(sum_cos).to_degrees();
        Some(if mean < 0.0 { mean + 360.0 } else { mean })
    };

    AspectResult {
        rows,
        cols,
        aspect_degrees: aspect_deg,
        aspect_class,
        mean_aspect,
    }
}

/// 将坡向角度映射为 8 方向字符串。
fn classify_aspect(degrees: f64) -> String {
    match degrees {
        d if d < 22.5 || d >= 337.5 => "N",
        d if (22.5..67.5).contains(&d) => "NE",
        d if (67.5..112.5).contains(&d) => "E",
        d if (112.5..157.5).contains(&d) => "SE",
        d if (157.5..202.5).contains(&d) => "S",
        d if (202.5..247.5).contains(&d) => "SW",
        d if (247.5..292.5).contains(&d) => "W",
        _ => "NW",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建 5×5 测试 DEM：中心隆起（简化山体）。
    fn test_dem() -> (Vec<f64>, usize, usize) {
        let rows = 5;
        let cols = 5;
        let dem = vec![
            10.0, 10.0, 10.0, 10.0, 10.0,
            10.0, 15.0, 20.0, 15.0, 10.0,
            10.0, 20.0, 30.0, 20.0, 10.0,
            10.0, 15.0, 20.0, 15.0, 10.0,
            10.0, 10.0, 10.0, 10.0, 10.0,
        ];
        (dem, rows, cols)
    }

    #[test]
    fn test_slope_degrees() {
        let (dem, rows, cols) = test_dem();
        let result = compute_slope_degrees(&dem, rows, cols, 10.0, None);
        // 检查山坡像素 (1,1) — 非对称位置应有坡度
        let slope_at = result.slope_degrees[1 * cols + 1];
        assert!(!slope_at.is_nan(), "Slope pixel should have a value: {:?}", result);
        assert!(slope_at > 0.0, "Slope at (1,1) should be > 0, got {}", slope_at);
        // 边界像素应为 NaN
        assert!(result.slope_degrees[0].is_nan(), "Edge should be NaN");
        assert!(result.slope_degrees[4].is_nan(), "Edge should be NaN");
        assert!(result.mean_degrees.is_some());
        assert!(result.max_degrees.is_some());
    }

    #[test]
    fn test_slope_percent() {
        let (dem, rows, cols) = test_dem();
        let result = compute_slope_percent(&dem, rows, cols, 10.0, None);
        // 同样检查 (1,1) 像素的百分比坡度
        assert!(result.slope_percent[1 * cols + 1] > 1.0, "Percent slope should be > 1%");
    }

    #[test]
    fn test_aspect() {
        let (dem, rows, cols) = test_dem();
        let result = compute_aspect(&dem, rows, cols, 10.0, None);
        let center = result.aspect_degrees[2 * cols + 2];
        // 中心对称山体，坡向应为平坦（-1）
        assert!((center + 1.0).abs() < 1e-6, "Symmetric peak should be flat: got {}", center);
        // 山坡像素 (1,1) 应有非平坦坡向
        let slope_aspect = result.aspect_degrees[1 * cols + 1];
        assert!(!slope_aspect.is_nan());
        assert!(slope_aspect > 0.0);
        // 边界应为 NaN
        assert!(result.aspect_degrees[0].is_nan());
        assert!(result.mean_aspect.is_some());
    }

    #[test]
    fn test_aspect_classification() {
        assert_eq!(classify_aspect(0.0), "N");
        assert_eq!(classify_aspect(90.0), "E");
        assert_eq!(classify_aspect(180.0), "S");
        assert_eq!(classify_aspect(270.0), "W");
        assert_eq!(classify_aspect(45.0), "NE");
        assert_eq!(classify_aspect(135.0), "SE");
        assert_eq!(classify_aspect(225.0), "SW");
        assert_eq!(classify_aspect(315.0), "NW");
    }

    #[test]
    fn test_nodata_handling() {
        let rows = 3;
        let cols = 3;
        let dem = vec![
            -999.0, -999.0, -999.0,
            -999.0, 100.0,  -999.0,
            -999.0, -999.0, -999.0,
        ];
        // 全部 NoData，所有结果应为 NaN
        let result = compute_slope_degrees(&dem, rows, cols, 10.0, Some(-999.0));
        assert!(result.mean_degrees.is_none());
        assert!(result.slope_degrees.iter().all(|v| v.is_nan()));
    }
}
