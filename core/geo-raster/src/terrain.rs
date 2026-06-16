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
            if z.iter()
                .any(|&v| v.is_nan() || (!nd.is_nan() && (v - nd).abs() < 1e-10))
            {
                continue;
            }

            // Horn 方法: dz/dx = ((z7 + 2*z5 + z3) - (z1 + 2*z4 + z6)) / (8 * cell_size)
            // 但标准 arcgis 做法略有不同。我们用标准 Horn:
            let dz_dx =
                ((z[7] + 2.0 * z[4] + z[2]) - (z[0] + 2.0 * z[3] + z[5])) / (8.0 * cell_size_m);
            let dz_dy =
                ((z[5] + 2.0 * z[6] + z[7]) - (z[0] + 2.0 * z[1] + z[2])) / (8.0 * cell_size_m);

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
    let max_degrees = if valid.is_empty() {
        None
    } else {
        Some(max_degrees)
    };

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

            if z.iter()
                .any(|&v| v.is_nan() || (!nd.is_nan() && (v - nd).abs() < 1e-10))
            {
                continue;
            }

            let dz_dx =
                ((z[7] + 2.0 * z[4] + z[2]) - (z[0] + 2.0 * z[3] + z[5])) / (8.0 * cell_size_m);
            let dz_dy =
                ((z[5] + 2.0 * z[6] + z[7]) - (z[0] + 2.0 * z[1] + z[2])) / (8.0 * cell_size_m);

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

    let valid: Vec<f64> = aspect_deg
        .iter()
        .cloned()
        .filter(|&v| !v.is_nan() && v >= 0.0)
        .collect();
    let mean_aspect = if valid.is_empty() {
        None
    } else {
        // 使用向量平均计算圆形均值
        let (sum_sin, sum_cos) = valid.iter().fold((0.0, 0.0), |(s, c), &a| {
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

/// 地形位置指数 (Topographic Position Index)。
///
/// TPI = z_center - mean(neighbors)，正值=山脊/山顶，负值=山谷。
///
/// # 参数
/// - `dem`: DEM 高程值，行优先
/// - `rows`, `cols`: DEM 尺寸
/// - `radius`: 分析窗口半径（像元数），1 表示 3×3
/// - `nodata`: NoData 值
pub fn compute_tpi(
    dem: &[f64],
    rows: usize,
    cols: usize,
    radius: usize,
    nodata: Option<f64>,
) -> Vec<f64> {
    let nd = nodata.unwrap_or(f64::NAN);
    let n = rows * cols;
    let mut tpi = vec![f64::NAN; n];

    for r in radius..rows - radius {
        for c in radius..cols - radius {
            let idx = r * cols + c;
            let center = dem[idx];

            // skip nodata center
            if center.is_nan() || (!nd.is_nan() && (center - nd).abs() < 1e-10) {
                continue;
            }

            let mut sum = 0.0;
            let mut count = 0usize;
            for dr in -(radius as isize)..=radius as isize {
                for dc in -(radius as isize)..=radius as isize {
                    if dr == 0 && dc == 0 {
                        continue;
                    }
                    let nr = (r as isize + dr) as usize;
                    let nc = (c as isize + dc) as usize;
                    let v = dem[nr * cols + nc];
                    if v.is_nan() || (!nd.is_nan() && (v - nd).abs() < 1e-10) {
                        continue;
                    }
                    sum += v;
                    count += 1;
                }
            }
            if count > 0 {
                tpi[idx] = center - sum / count as f64;
            }
        }
    }
    tpi
}

/// 地形粗糙度指数 (Terrain Ruggedness Index)。
///
/// TRI = sqrt(Σ (z_center - z_neighbor)²)，Riley et al. (1999)。
///
/// # 参数
/// - `dem`: DEM 高程值，行优先
/// - `rows`, `cols`: DEM 尺寸
/// - `nodata`: NoData 值
pub fn compute_tri(dem: &[f64], rows: usize, cols: usize, nodata: Option<f64>) -> Vec<f64> {
    let nd = nodata.unwrap_or(f64::NAN);
    let n = rows * cols;
    let mut tri = vec![f64::NAN; n];

    for r in 1..rows - 1 {
        for c in 1..cols - 1 {
            let idx = r * cols + c;
            let center = dem[idx];

            if center.is_nan() || (!nd.is_nan() && (center - nd).abs() < 1e-10) {
                continue;
            }

            let mut ssq = 0.0;
            let mut count = 0usize;
            for dr in -1i32..=1 {
                for dc in -1i32..=1 {
                    if dr == 0 && dc == 0 {
                        continue;
                    }
                    let nr = (r as i32 + dr) as usize;
                    let nc = (c as i32 + dc) as usize;
                    let v = dem[nr * cols + nc];
                    if v.is_nan() || (!nd.is_nan() && (v - nd).abs() < 1e-10) {
                        continue;
                    }
                    ssq += (center - v).powi(2);
                    count += 1;
                }
            }
            if count > 0 {
                tri[idx] = ssq.sqrt();
            }
        }
    }
    tri
}

/// 山体阴影 (Hillshade)。
///
/// 使用 Horn 法计算坡度/坡向，结合太阳方位角/高度角生成 0~255 灰度阴影。
///
/// # 参数
/// - `dem`: DEM 高程值，行优先
/// - `rows`, `cols`: DEM 尺寸
/// - `cell_size_m`: 像元分辨率（米）
/// - `azimuth_deg`: 太阳方位角（度，0=正北，顺时针）
/// - `altitude_deg`: 太阳高度角（度，0=地平线，90=天顶）
/// - `nodata`: NoData 值
pub fn compute_hillshade(
    dem: &[f64],
    rows: usize,
    cols: usize,
    cell_size_m: f64,
    azimuth_deg: f64,
    altitude_deg: f64,
    nodata: Option<f64>,
) -> Vec<f64> {
    let nd = nodata.unwrap_or(f64::NAN);
    let n = rows * cols;
    let mut hillshade = vec![f64::NAN; n];

    let zenith_rad = (90.0 - altitude_deg).to_radians();
    let azimuth_rad = azimuth_deg.to_radians();
    let sin_zenith = zenith_rad.sin();
    let cos_zenith = zenith_rad.cos();

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

            if z.iter()
                .any(|&v| v.is_nan() || (!nd.is_nan() && (v - nd).abs() < 1e-10))
            {
                continue;
            }

            let dz_dx =
                ((z[7] + 2.0 * z[4] + z[2]) - (z[0] + 2.0 * z[3] + z[5])) / (8.0 * cell_size_m);
            let dz_dy =
                ((z[5] + 2.0 * z[6] + z[7]) - (z[0] + 2.0 * z[1] + z[2])) / (8.0 * cell_size_m);

            let slope_rad = (dz_dx * dz_dx + dz_dy * dz_dy).sqrt().atan();

            // 坡向：从正北顺时针
            let mut aspect_rad = (-dz_dy).atan2(dz_dx);
            // 转换为从正北顺时针的方位角
            aspect_rad = std::f64::consts::FRAC_PI_2 - aspect_rad;
            if aspect_rad < 0.0 {
                aspect_rad += 2.0 * std::f64::consts::PI;
            }

            let hs = 255.0
                * (cos_zenith * slope_rad.cos()
                    + sin_zenith * slope_rad.sin() * (azimuth_rad - aspect_rad).cos());
            hillshade[idx] = hs.max(0.0);
        }
    }
    hillshade
}

/// 双线性插值重采样。
///
/// 将源栅格重采样到目标尺寸。
///
/// # 参数
/// - `src`: 源数据（行优先）
/// - `src_rows`, `src_cols`: 源尺寸
/// - `dst_rows`, `dst_cols`: 目标尺寸
/// - `nodata`: 源 NoData 值
pub fn resample_bilinear(
    src: &[f64],
    src_rows: usize,
    src_cols: usize,
    dst_rows: usize,
    dst_cols: usize,
    nodata: Option<f64>,
) -> Vec<f64> {
    let nd = nodata.unwrap_or(f64::NAN);
    let mut dst = vec![f64::NAN; dst_rows * dst_cols];
    let scale_r = src_rows as f64 / dst_rows as f64;
    let scale_c = src_cols as f64 / dst_cols as f64;

    for dr in 0..dst_rows {
        for dc in 0..dst_cols {
            // 源坐标（浮点）
            let sr = (dr as f64 + 0.5) * scale_r - 0.5;
            let sc = (dc as f64 + 0.5) * scale_c - 0.5;

            let r0 = sr.floor() as isize;
            let c0 = sc.floor() as isize;
            let r1 = r0 + 1;
            let c1 = c0 + 1;

            if r0 < 0 || c0 < 0 || r1 >= src_rows as isize || c1 >= src_cols as isize {
                continue;
            }

            let fr = sr - r0 as f64;
            let fc = sc - c0 as f64;

            let (r0u, c0u, r1u, c1u) = (r0 as usize, c0 as usize, r1 as usize, c1 as usize);
            let v00 = src[r0u * src_cols + c0u];
            let v10 = src[r0u * src_cols + c1u];
            let v01 = src[r1u * src_cols + c0u];
            let v11 = src[r1u * src_cols + c1u];

            // 检查 NoData
            let values = [v00, v10, v01, v11];
            if values
                .iter()
                .any(|&v| v.is_nan() || (!nd.is_nan() && (v - nd).abs() < 1e-10))
            {
                continue;
            }

            let top = v00 + (v10 - v00) * fc;
            let bot = v01 + (v11 - v01) * fc;
            dst[dr * dst_cols + dc] = top + (bot - top) * fr;
        }
    }
    dst
}

/// Zonal Statistics 结果。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ZonalStats {
    /// 每个 zone 的统计值（顺序与输入 zone geometries 对应）。
    pub zones: Vec<ZoneStats>,
}

/// 单 zone 统计。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ZoneStats {
    /// 像元数。
    pub count: usize,
    /// 最小值。
    pub min: Option<f64>,
    /// 最大值。
    pub max: Option<f64>,
    /// 均值。
    pub mean: Option<f64>,
    /// 标准差。
    pub stddev: Option<f64>,
    /// 总和。
    pub sum: Option<f64>,
}

/// 按 zone mask 做分区统计。
///
/// `zones` 长度为 `rows * cols`，每个像元 ∈ {0..num_zones}，
/// 值为 `zone_id` 表示该像元属于该 zone，0 通常表示背景/忽略。
pub fn zonal_stats(
    values: &[f64],
    zones: &[u32],
    num_zones: usize,
    _nodata: Option<f64>,
) -> ZonalStats {
    let mut zone_data: Vec<Vec<f64>> = vec![Vec::new(); num_zones];

    for i in 0..values.len().min(zones.len()) {
        let z = zones[i] as usize;
        if z > 0 && z <= num_zones && values[i].is_finite() {
            zone_data[z - 1].push(values[i]);
        }
    }

    let zones_stats = zone_data
        .into_iter()
        .map(|vals| {
            let count = vals.len();
            if count == 0 {
                return ZoneStats {
                    count: 0,
                    min: None,
                    max: None,
                    mean: None,
                    stddev: None,
                    sum: None,
                };
            }
            let sum: f64 = vals.iter().sum();
            let mean = sum / count as f64;
            let min = vals.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let variance = vals.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / count as f64;
            ZoneStats {
                count,
                min: if min.is_finite() { Some(min) } else { None },
                max: if max.is_finite() { Some(max) } else { None },
                mean: Some(mean),
                stddev: Some(variance.sqrt()),
                sum: Some(sum),
            }
        })
        .collect();

    ZonalStats { zones: zones_stats }
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
            10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 15.0, 20.0, 15.0, 10.0, 10.0, 20.0, 30.0, 20.0,
            10.0, 10.0, 15.0, 20.0, 15.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0,
        ];
        (dem, rows, cols)
    }

    #[test]
    fn test_slope_degrees() {
        let (dem, rows, cols) = test_dem();
        let result = compute_slope_degrees(&dem, rows, cols, 10.0, None);
        // 检查山坡像素 (1,1) — 非对称位置应有坡度
        let slope_at = result.slope_degrees[1 * cols + 1];
        assert!(
            !slope_at.is_nan(),
            "Slope pixel should have a value: {:?}",
            result
        );
        assert!(
            slope_at > 0.0,
            "Slope at (1,1) should be > 0, got {}",
            slope_at
        );
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
        assert!(
            result.slope_percent[1 * cols + 1] > 1.0,
            "Percent slope should be > 1%"
        );
    }

    #[test]
    fn test_aspect() {
        let (dem, rows, cols) = test_dem();
        let result = compute_aspect(&dem, rows, cols, 10.0, None);
        let center = result.aspect_degrees[2 * cols + 2];
        // 中心对称山体，坡向应为平坦（-1）
        assert!(
            (center + 1.0).abs() < 1e-6,
            "Symmetric peak should be flat: got {}",
            center
        );
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
            -999.0, -999.0, -999.0, -999.0, 100.0, -999.0, -999.0, -999.0, -999.0,
        ];
        // 全部 NoData，所有结果应为 NaN
        let result = compute_slope_degrees(&dem, rows, cols, 10.0, Some(-999.0));
        assert!(result.mean_degrees.is_none());
        assert!(result.slope_degrees.iter().all(|v| v.is_nan()));
    }

    #[test]
    fn test_tpi() {
        let (dem, rows, cols) = test_dem();
        let tpi = compute_tpi(&dem, rows, cols, 1, None);
        // 中心 (2,2) = 30，邻居均值 ≈ 15，TPI ≈ +15 (山脊)
        let center = tpi[2 * cols + 2];
        assert!(
            center > 10.0,
            "Center TPI should be positive (ridge): got {center}"
        );
        // 边缘应为 NaN
        assert!(tpi[0].is_nan());
    }

    #[test]
    fn test_tri() {
        let (dem, rows, cols) = test_dem();
        let tri = compute_tri(&dem, rows, cols, None);
        // 山坡处应有粗糙度
        let slope_tri = tri[1 * cols + 1];
        assert!(slope_tri > 0.0, "Slope TRI should be > 0: got {slope_tri}");
        // 边缘 NaN
        assert!(tri[0].is_nan());
    }

    #[test]
    fn test_hillshade() {
        let (dem, rows, cols) = test_dem();
        let hs = compute_hillshade(&dem, rows, cols, 10.0, 315.0, 45.0, None);
        let center = hs[2 * cols + 2];
        // 平坦峰顶（坡度为0），hillshade = 255 * cos(zenith)
        assert!(!center.is_nan());
        assert!(
            center >= 0.0 && center <= 255.0,
            "Hillshade range: {center}"
        );
        // 山坡应有阴影变化
        let slope_hs = hs[1 * cols + 1];
        assert!(!slope_hs.is_nan());
    }

    #[test]
    fn test_resample_bilinear() {
        let src = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        ];
        // 4x4 → 2x2 下采样
        let dst = resample_bilinear(&src, 4, 4, 2, 2, None);
        assert_eq!(dst.len(), 4);
        // (0,0) 应在 1..6 之间
        assert!(dst[0] > 1.0 && dst[0] < 7.0);
        // (1,1) 应在 11..16 之间
        assert!(dst[3] > 10.0 && dst[3] < 17.0);
    }

    #[test]
    fn test_zonal_stats() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        // zone 1: first row (1,2,3), zone 2: rest, 0 = background
        let zones = vec![1u32, 1, 1, 2, 2, 2, 2, 2, 2];
        let result = zonal_stats(&values, &zones, 2, None);
        assert_eq!(result.zones.len(), 2);
        // Zone 1: [1,2,3] → mean=2, min=1, max=3, sum=6
        assert_eq!(result.zones[0].count, 3);
        assert_eq!(result.zones[0].mean, Some(2.0));
        assert_eq!(result.zones[0].min, Some(1.0));
        assert_eq!(result.zones[0].max, Some(3.0));
        assert_eq!(result.zones[0].sum, Some(6.0));
        // Zone 2: [4,5,6,7,8,9] → mean=6.5
        assert_eq!(result.zones[1].count, 6);
        assert_eq!(result.zones[1].mean, Some(6.5));
        assert_eq!(result.zones[1].min, Some(4.0));
        assert_eq!(result.zones[1].max, Some(9.0));
    }
}
