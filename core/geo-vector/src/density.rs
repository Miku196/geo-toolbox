use geo_core::{GeoError, GeoResult};

/// 核密度估计 (Kernel Density Estimation)。
///
/// 对点集做高斯核密度估计，返回规则网格的密度值。
///
/// # 参数
/// - `points`: 点坐标 `(x, y)` 列表
/// - `grid_cols`, `grid_rows`: 输出网格尺寸
/// - `bbox`: 分析范围 `(min_x, min_y, max_x, max_y)`
/// - `bandwidth`: 带宽（核半径），0 则用 Silverman 法则自动计算
pub fn kernel_density(
    points: &[(f64, f64)],
    grid_cols: usize,
    grid_rows: usize,
    bbox: (f64, f64, f64, f64),
    bandwidth: f64,
) -> Vec<f64> {
    let n = points.len();
    if n == 0 || grid_cols == 0 || grid_rows == 0 {
        return vec![0.0; grid_cols * grid_rows];
    }

    let bw = if bandwidth > 0.0 {
        bandwidth
    } else {
        // Silverman's rule of thumb
        let mean_x = points.iter().map(|p| p.0).sum::<f64>() / n as f64;
        let mean_y = points.iter().map(|p| p.1).sum::<f64>() / n as f64;
        let std_x = (points.iter().map(|p| (p.0 - mean_x).powi(2)).sum::<f64>() / n as f64).sqrt();
        let std_y = (points.iter().map(|p| (p.1 - mean_y).powi(2)).sum::<f64>() / n as f64).sqrt();
        let sigma = std_x.min(std_y).max(0.001);
        (4.0 * sigma.powi(5) / (3.0 * n as f64)).powf(1.0 / 5.0)
    };

    let (min_x, min_y, max_x, max_y) = bbox;
    let cell_w = (max_x - min_x) / grid_cols as f64;
    let cell_h = (max_y - min_y) / grid_rows as f64;
    let inv_bw_sq = 1.0 / (bw * bw);
    let norm = 1.0 / (2.0 * std::f64::consts::PI * bw * bw);

    let mut result = vec![0.0; grid_cols * grid_rows];
    for (px, py) in points {
        let ci = ((px - min_x) / cell_w).floor() as isize;
        let ri = ((py - min_y) / cell_h).floor() as isize;
        let search_radius = (bw * 3.0 / cell_w).ceil() as isize;

        for dr in -search_radius..=search_radius {
            for dc in -search_radius..=search_radius {
                let r = ri + dr;
                let c = ci + dc;
                if r < 0 || c < 0 || r >= grid_rows as isize || c >= grid_cols as isize {
                    continue;
                }
                let cx = min_x + (c as f64 + 0.5) * cell_w;
                let cy = min_y + (r as f64 + 0.5) * cell_h;
                let dist_sq = (cx - px).powi(2) + (cy - py).powi(2);
                if dist_sq > (bw * 3.0).powi(2) {
                    continue;
                }
                result[(r as usize) * grid_cols + c as usize] +=
                    norm * (-0.5 * dist_sq * inv_bw_sq).exp();
            }
        }
    }

    result
}

/// 线密度分析。
///
/// 计算每个网格像元内线段的总长度密度（单位：长度/面积）。
///
/// # 参数
/// - `lines`: 线段 `(x1, y1, x2, y2)` 列表
/// - `grid_cols`, `grid_rows`: 输出网格尺寸
/// - `bbox`: 分析范围
///
/// # 错误
/// - `grid_cols` / `grid_rows` 任一为 0 时返回 [`GeoError::InvalidInput`]，
///   避免除零 / 越界，而不是 panic。
pub fn line_density(
    lines: &[(f64, f64, f64, f64)],
    grid_cols: usize,
    grid_rows: usize,
    bbox: (f64, f64, f64, f64),
) -> GeoResult<Vec<f64>> {
    if grid_cols == 0 || grid_rows == 0 {
        return Err(GeoError::InvalidInput {
            field: "grid".into(),
            reason: "line_density requires non-zero grid_cols and grid_rows".into(),
        });
    }
    let (min_x, min_y, max_x, max_y) = bbox;
    let cell_w = (max_x - min_x) / grid_cols as f64;
    let cell_h = (max_y - min_y) / grid_rows as f64;
    let cell_area = cell_w * cell_h;

    let mut result = vec![0.0; grid_cols * grid_rows];

    for &(x1, y1, x2, y2) in lines {
        // 确定线段跨越的网格范围
        let c1 = ((x1 - min_x) / cell_w).floor() as isize;
        let r1 = ((y1 - min_y) / cell_h).floor() as isize;
        let c2 = ((x2 - min_x) / cell_w).floor() as isize;
        let r2 = ((y2 - min_y) / cell_h).floor() as isize;

        let c_min = c1.min(c2).max(0) as usize;
        let c_max = c1.max(c2).min(grid_cols as isize - 1) as usize;
        let r_min = r1.min(r2).max(0) as usize;
        let r_max = r1.max(r2).min(grid_rows as isize - 1) as usize;

        let dx = x2 - x1;
        let dy = y2 - y1;
        let line_len = (dx * dx + dy * dy).sqrt();
        if line_len < 1e-10 {
            continue;
        }

        for r in r_min..=r_max {
            for c in c_min..=c_max {
                let cx_min = min_x + c as f64 * cell_w;
                let cy_min = min_y + r as f64 * cell_h;
                let cx_max = cx_min + cell_w;
                let cy_max = cy_min + cell_h;

                // 线段与网格相交的裁剪长度（简化：按网格比例估算）
                let clipped_len = clip_line_length(x1, y1, x2, y2, cx_min, cy_min, cx_max, cy_max);
                result[r * grid_cols + c] += clipped_len / cell_area;
            }
        }
    }
    Ok(result)
}

/// 计算线段与矩形裁剪后的长度（Cohen-Sutherland 裁剪简化版）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn clip_line_length(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    rx_min: f64,
    ry_min: f64,
    rx_max: f64,
    ry_max: f64,
) -> f64 {
    // Liang-Barsky 线段裁剪
    let dx = x2 - x1;
    let dy = y2 - y1;
    let p = [-dx, dx, -dy, dy];
    let q = [x1 - rx_min, rx_max - x1, y1 - ry_min, ry_max - y1];

    let mut u1: f64 = 0.0;
    let mut u2: f64 = 1.0;

    for i in 0..4 {
        if p[i].abs() < 1e-10 {
            if q[i] < 0.0 {
                return 0.0; // 线段完全在外面
            }
        } else {
            let u = q[i] / p[i];
            if p[i] < 0.0 {
                u1 = u1.max(u);
            } else {
                u2 = u2.min(u);
            }
        }
    }

    if u1 > u2 {
        return 0.0; // 无交集
    }

    // 裁剪后长度
    (dx * dx + dy * dy).sqrt() * (u2 - u1)
}
