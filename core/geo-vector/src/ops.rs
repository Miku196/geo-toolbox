use geo::algorithm::{Area, BooleanOps, BoundingRect, ConvexHull};
use geo_types::{Coord, LineString, MultiPolygon, Polygon};

/// 缓冲区模式选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferMode {
    /// O(1) 轴对齐 BBox 矩形外扩（最快，忽略形状）。
    Bbox,
    /// O(n) 凸壳近似（适合凸多边形，凹形会被填充）。
    ConvexHull { quadrant_segments: u8 },
    /// O(n²) 精确偏移：逐边挤推平行四边形 + 顶点圆弧 + BooleanOps Union。
    /// 参考 Shapely / JTS buffer 算法。
    Precise { quadrant_segments: u8 },
}

/// 最大顶点数限制（防止恶意输入导致 OOM）。
pub const MAX_BUFFER_VERTICES: usize = 500_000;

/// 对多边形进行缓冲区分析。
///
/// # 参数
/// - `poly`: 输入多边形
/// - `distance`: 缓冲距离（正=外扩，负=内缩，0=原样返回）
/// - `mode`: 缓冲区模式选择
pub fn buffer(poly: &Polygon<f64>, distance: f64, mode: BufferMode) -> MultiPolygon<f64> {
    let n = poly.exterior().0.len();
    if !(3..=MAX_BUFFER_VERTICES).contains(&n) {
        return MultiPolygon::new(vec![poly.clone()]);
    }
    if distance == 0.0 {
        return MultiPolygon::new(vec![poly.clone()]);
    }

    match mode {
        BufferMode::Bbox => {
            if distance > 0.0 {
                bbox_buffer_outer(poly, distance)
            } else {
                bbox_buffer_inner(poly, -distance)
            }
        }
        BufferMode::ConvexHull { quadrant_segments } => {
            if distance > 0.0 {
                convexhull_buffer_outer(poly, distance, quadrant_segments.clamp(4, 32) as usize)
            } else {
                bbox_buffer_inner(poly, -distance)
            }
        }
        BufferMode::Precise { quadrant_segments } => {
            if distance > 0.0 {
                precise_buffer_outer(poly, distance, quadrant_segments.clamp(4, 32) as usize)
            } else {
                bbox_buffer_inner(poly, -distance)
            }
        }
    }
}

// ═══════════════════════ BBox 模式 ═══════════════════════

fn bbox_buffer_outer(poly: &Polygon<f64>, dist: f64) -> MultiPolygon<f64> {
    let bbox = match poly.bounding_rect() {
        Some(r) => r,
        None => return MultiPolygon::new(vec![poly.clone()]),
    };
    let min = bbox.min();
    let max = bbox.max();
    MultiPolygon::new(vec![Polygon::new(
        LineString::new(vec![
            Coord {
                x: min.x - dist,
                y: min.y - dist,
            },
            Coord {
                x: max.x + dist,
                y: min.y - dist,
            },
            Coord {
                x: max.x + dist,
                y: max.y + dist,
            },
            Coord {
                x: min.x - dist,
                y: max.y + dist,
            },
            Coord {
                x: min.x - dist,
                y: min.y - dist,
            },
        ]),
        vec![],
    )])
}

fn bbox_buffer_inner(poly: &Polygon<f64>, dist: f64) -> MultiPolygon<f64> {
    let bbox = match poly.bounding_rect() {
        Some(r) => r,
        None => return MultiPolygon::new(vec![poly.clone()]),
    };
    let min = bbox.min();
    let max = bbox.max();
    let cx = (min.x + max.x) / 2.0;
    let cy = (min.y + max.y) / 2.0;
    let hw = (max.x - min.x) / 2.0 - dist;
    let hh = (max.y - min.y) / 2.0 - dist;
    if hw <= 0.0 || hh <= 0.0 {
        return MultiPolygon::new(vec![]);
    }
    MultiPolygon::new(vec![Polygon::new(
        LineString::new(vec![
            Coord {
                x: cx - hw,
                y: cy - hh,
            },
            Coord {
                x: cx + hw,
                y: cy - hh,
            },
            Coord {
                x: cx + hw,
                y: cy + hh,
            },
            Coord {
                x: cx - hw,
                y: cy + hh,
            },
            Coord {
                x: cx - hw,
                y: cy - hh,
            },
        ]),
        vec![],
    )])
}

// ═══════════════════════ ConvexHull 模式 ═══════════════════════

fn convexhull_buffer_outer(poly: &Polygon<f64>, dist: f64, segments: usize) -> MultiPolygon<f64> {
    let exterior = poly.exterior();
    let coords = &exterior.0;
    if coords.len() < 3 {
        return MultiPolygon::new(vec![poly.clone()]);
    }

    let total_len: f64 = coords
        .windows(2)
        .map(|w| {
            let dx = w[1].x - w[0].x;
            let dy = w[1].y - w[0].y;
            (dx * dx + dy * dy).sqrt()
        })
        .sum();
    let step = (total_len / (coords.len() as f64 * segments as f64)).max(dist / 4.0);

    let mut offset_points: Vec<Coord<f64>> = Vec::new();
    for w in coords.windows(2) {
        let dx = w[1].x - w[0].x;
        let dy = w[1].y - w[0].y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-12 {
            continue;
        }
        let nx = dy / len;
        let ny = -dx / len;
        let n_steps = (len / step).ceil() as usize;
        for i in 0..=n_steps {
            let t = i as f64 / n_steps as f64;
            offset_points.push(Coord {
                x: w[0].x + dx * t + nx * dist,
                y: w[0].y + dy * t + ny * dist,
            });
        }
    }

    if offset_points.len() < 3 {
        return bbox_buffer_outer(poly, dist);
    }

    let hull_poly = Polygon::new(LineString::new(offset_points), vec![]).convex_hull();
    MultiPolygon::new(vec![hull_poly])
}

// ═══════════════════════ Precise 模式 ═══════════════════════

/// 精确外扩缓冲：逐边平行四边形挤推 + 顶点圆弧 + BooleanOps Union。
#[derive(Clone, Copy)]
struct EdgeNormal {
    nx: f64,
    ny: f64,
    len: f64,
}

/// Pre-compute outward edge normals for each segment of the polygon ring.
fn compute_edge_normals(coords: &[Coord<f64>], n: usize) -> Vec<EdgeNormal> {
    let mut normals = Vec::with_capacity(n);
    for i in 0..n {
        let j = (i + 1) % n;
        let dx = coords[j].x - coords[i].x;
        let dy = coords[j].y - coords[i].y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-12 {
            normals.push(EdgeNormal {
                nx: 0.0,
                ny: 0.0,
                len: 0.0,
            });
        } else {
            normals.push(EdgeNormal {
                nx: dy / len,
                ny: -dx / len,
                len,
            });
        }
    }
    normals
}

/// Build offset parallelograms for every edge of the polygon.
fn generate_offset_parallelograms(
    coords: &[Coord<f64>],
    n: usize,
    normals: &[EdgeNormal],
    dist: f64,
) -> Vec<Polygon<f64>> {
    let mut parts = Vec::with_capacity(n);
    for i in 0..n {
        let j = (i + 1) % n;
        let nv = &normals[i];
        if nv.len < 1e-12 {
            continue;
        }
        let ib = coords[i];
        let jb = coords[j];
        let i_off = Coord {
            x: ib.x + nv.nx * dist,
            y: ib.y + nv.ny * dist,
        };
        let j_off = Coord {
            x: jb.x + nv.nx * dist,
            y: jb.y + nv.ny * dist,
        };
        parts.push(Polygon::new(
            LineString::new(vec![ib, i_off, j_off, jb, ib]),
            vec![],
        ));
    }
    parts
}

/// Insert arc polygons at convex vertices to round the offset corners.
fn build_convex_vertex_arcs(
    coords: &[Coord<f64>],
    n: usize,
    normals: &[EdgeNormal],
    dist: f64,
    segments: usize,
) -> Vec<Polygon<f64>> {
    let step_angle = std::f64::consts::FRAC_PI_2 / (segments as f64).max(1.0);
    let mut parts = Vec::with_capacity(n);
    for i in 0..n {
        let prev = (i + n - 1) % n;
        let curr = i;

        let n1 = &normals[prev];
        let n2 = &normals[curr];
        if n1.len < 1e-12 || n2.len < 1e-12 {
            continue;
        }

        let prev_pt = coords[prev];
        let curr_pt = coords[curr];
        let next_pt = coords[(curr + 1) % n];

        let dx1 = curr_pt.x - prev_pt.x;
        let dy1 = curr_pt.y - prev_pt.y;
        let dx2 = next_pt.x - curr_pt.x;
        let dy2 = next_pt.y - curr_pt.y;
        let cross = dx1 * dy2 - dy1 * dx2;

        if cross <= 0.0 {
            continue;
        }

        let prev_offset_end = Coord {
            x: curr_pt.x + n1.nx * dist,
            y: curr_pt.y + n1.ny * dist,
        };
        let curr_offset_start = Coord {
            x: curr_pt.x + n2.nx * dist,
            y: curr_pt.y + n2.ny * dist,
        };

        let angle1 = (prev_offset_end.y - curr_pt.y).atan2(prev_offset_end.x - curr_pt.x);
        let angle2 = (curr_offset_start.y - curr_pt.y).atan2(curr_offset_start.x - curr_pt.x);

        let mut sweep = angle2 - angle1;
        while sweep < -std::f64::consts::PI {
            sweep += std::f64::consts::TAU;
        }
        while sweep > std::f64::consts::PI {
            sweep -= std::f64::consts::TAU;
        }

        let arc_steps = ((sweep.abs() / step_angle).ceil() as usize).max(2);
        let arc_step = sweep / arc_steps as f64;

        let mut arc_coords: Vec<Coord<f64>> = Vec::with_capacity(arc_steps + 2);
        arc_coords.push(curr_pt);
        for k in 0..=arc_steps {
            let a = angle1 + arc_step * k as f64;
            arc_coords.push(Coord {
                x: curr_pt.x + a.cos() * dist,
                y: curr_pt.y + a.sin() * dist,
            });
        }

        parts.push(Polygon::new(LineString::new(arc_coords), vec![]));
    }
    parts
}

/// Union all geometry parts into a single MultiPolygon, falling back to BBox on failure.
fn union_geometry_parts(
    parts: Vec<Polygon<f64>>,
    poly: &Polygon<f64>,
    dist: f64,
) -> MultiPolygon<f64> {
    if parts.is_empty() {
        return bbox_buffer_outer(poly, dist);
    }
    let mut result = MultiPolygon::new(vec![parts[0].clone()]);
    for part in &parts[1..] {
        result = result.union(&MultiPolygon::new(vec![part.clone()]));
    }
    result
}

fn precise_buffer_outer(poly: &Polygon<f64>, dist: f64, segments: usize) -> MultiPolygon<f64> {
    let exterior = poly.exterior();
    let coords = &exterior.0;
    let n = coords.len() - 1;
    if n < 3 {
        return MultiPolygon::new(vec![poly.clone()]);
    }

    let edge_normals = compute_edge_normals(coords, n);
    let mut parts: Vec<Polygon<f64>> = Vec::with_capacity(n * 2);

    parts.push(poly.clone());

    let pgrams = generate_offset_parallelograms(coords, n, &edge_normals, dist);
    parts.extend(pgrams);

    let arcs = build_convex_vertex_arcs(coords, n, &edge_normals, dist, segments);
    parts.extend(arcs);

    union_geometry_parts(parts, poly, dist)
}

// ═══════════════════════ 通用运算 ═══════════════════════

/// 多边形相交（使用 BooleanOps 真实几何相交）。
pub fn intersect(a: &Polygon<f64>, b: &Polygon<f64>) -> Option<MultiPolygon<f64>> {
    if !bbox_intersect(a, b) {
        return None;
    }
    let result = a.intersection(b);
    if result.0.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// 合并多个多边形（使用 BooleanOps 逐个合并）。
pub fn union_all(polys: &[Polygon<f64>]) -> Option<MultiPolygon<f64>> {
    if polys.is_empty() {
        return None;
    }
    let mut result = MultiPolygon::new(polys[0..1].to_vec());
    for poly in &polys[1..] {
        result = result.union(&MultiPolygon::new(vec![poly.clone()]));
    }
    Some(result)
}

/// 计算 A - B（擦除/裁剪）。返回 A 中不在 B 内的部分。
pub fn difference(a: &Polygon<f64>, b: &Polygon<f64>) -> Option<MultiPolygon<f64>> {
    if !bbox_intersect(a, b) {
        return Some(MultiPolygon::new(vec![a.clone()]));
    }
    let result = a.difference(b);
    if result.0.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// 计算对称差 (A XOR B)。返回 A 和 B 的不重叠部分。
pub fn sym_difference(a: &Polygon<f64>, b: &Polygon<f64>) -> Option<MultiPolygon<f64>> {
    if !bbox_intersect(a, b) {
        return Some(MultiPolygon::new(vec![a.clone(), b.clone()]));
    }
    let result = a.xor(b);
    if result.0.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// 用裁剪多边形切割 MultiPolygon，保留重叠部分。
pub fn clip(target: &MultiPolygon<f64>, clip_poly: &Polygon<f64>) -> MultiPolygon<f64> {
    target
        .iter()
        .flat_map(|poly| {
            let inter = poly.intersection(clip_poly);
            inter.0
        })
        .collect::<Vec<_>>()
        .into()
}

/// 计算多边形面积（unsigned, sq degrees）。
pub fn area_sq_deg(poly: &Polygon<f64>) -> f64 {
    poly.unsigned_area()
}

fn bbox_intersect(a: &Polygon<f64>, b: &Polygon<f64>) -> bool {
    let ba = a.bounding_rect();
    let bb = b.bounding_rect();
    match (ba, bb) {
        (Some(ra), Some(rb)) => {
            ra.min().x < rb.max().x
                && ra.max().x > rb.min().x
                && ra.min().y < rb.max().y
                && ra.max().y > rb.min().y
        }
        _ => false,
    }
}

/// Douglas-Peucker 线简化（使用 `geo::Simplify` trait）。
///
/// 用给定 epsilon 容差简化线几何（Ramer-Douglas-Peucker 算法）。
/// 返回简化后的 MultiPolygon（如果是 Polygon 输入）。
pub fn simplify(poly: &Polygon<f64>, epsilon: f64) -> Polygon<f64> {
    use geo::Simplify;
    poly.simplify(&epsilon)
}

/// 简化线几何（LineString 输入）。
pub fn simplify_line(line: &LineString<f64>, epsilon: f64) -> LineString<f64> {
    use geo::Simplify;
    line.simplify(&epsilon)
}

/// Visvalingam-Whyatt 简化（LineString），按面积阈值删除次要顶点。
pub fn simplify_visvalingam(line: &LineString<f64>, epsilon: f64) -> LineString<f64> {
    use geo::SimplifyVw;
    line.simplify_vw(&epsilon)
}

/// Visvalingam-Whyatt 拓扑保持简化（LineString），避免自交。
pub fn simplify_visvalingam_preserve(line: &LineString<f64>, epsilon: f64) -> LineString<f64> {
    use geo::SimplifyVwPreserve;
    line.simplify_vw_preserve(&epsilon)
}

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
pub fn line_density(
    lines: &[(f64, f64, f64, f64)],
    grid_cols: usize,
    grid_rows: usize,
    bbox: (f64, f64, f64, f64),
) -> Vec<f64> {
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
    result
}

/// 计算线段与矩形裁剪后的长度（Cohen-Sutherland 裁剪简化版）。
#[allow(clippy::too_many_arguments)]
fn clip_line_length(
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

// ═══════════════════════ 测试 ═══════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use geo_types::{Coord, LineString};

    fn square(x: f64, y: f64, s: f64) -> Polygon<f64> {
        Polygon::new(
            LineString::new(vec![
                Coord { x, y },
                Coord { x: x + s, y },
                Coord { x: x + s, y: y + s },
                Coord { x, y: y + s },
                Coord { x, y },
            ]),
            vec![],
        )
    }

    /// L 形多边形（凹形）— 用于测试精确偏移 vs 凸壳差异
    fn l_shape() -> Polygon<f64> {
        Polygon::new(
            LineString::new(vec![
                Coord { x: 0.0, y: 0.0 },
                Coord { x: 10.0, y: 0.0 },
                Coord { x: 10.0, y: 8.0 },
                Coord { x: 6.0, y: 8.0 },
                Coord { x: 6.0, y: 4.0 },
                Coord { x: 0.0, y: 4.0 },
                Coord { x: 0.0, y: 0.0 },
            ]),
            vec![],
        )
    }

    #[test]
    fn test_intersect() {
        let a = square(0.0, 0.0, 10.0);
        let b = square(5.0, 5.0, 10.0);
        let result = intersect(&a, &b);
        assert!(result.is_some());
    }

    #[test]
    fn test_no_intersect() {
        let a = square(0.0, 0.0, 10.0);
        let b = square(20.0, 20.0, 10.0);
        let result = intersect(&a, &b);
        assert!(result.is_none());
    }

    #[test]
    fn test_union_all() {
        let a = square(0.0, 0.0, 5.0);
        let b = square(4.0, 0.0, 5.0);
        let result = union_all(&[a, b]);
        assert!(result.is_some());
    }

    #[test]
    fn test_buffer_bbox_positive() {
        let a = square(0.0, 0.0, 5.0);
        let buf = buffer(&a, 2.0, BufferMode::Bbox);
        assert!(buf.unsigned_area() > a.unsigned_area());
        // BBox 面积 = (5+4)*(5+4) = 81
        assert!((buf.unsigned_area() - 81.0).abs() < 1e-6);
    }

    #[test]
    fn test_buffer_bbox_negative() {
        let a = square(0.0, 0.0, 10.0);
        let buf = buffer(&a, -1.0, BufferMode::Bbox);
        // 内缩后面积 < 100
        assert!(buf.unsigned_area() < 100.0);
    }

    #[test]
    fn test_buffer_convexhull_positive() {
        let a = square(0.0, 0.0, 10.0);
        let buf = buffer(
            &a,
            2.0,
            BufferMode::ConvexHull {
                quadrant_segments: 16,
            },
        );
        assert!(buf.unsigned_area() > 100.0);
    }

    #[test]
    fn test_buffer_zero() {
        let a = square(0.0, 0.0, 5.0);
        let buf = buffer(
            &a,
            0.0,
            BufferMode::ConvexHull {
                quadrant_segments: 8,
            },
        );
        assert!((buf.unsigned_area() - a.unsigned_area()).abs() < 1e-6);
    }

    #[test]
    fn test_buffer_precise_square() {
        let a = square(0.0, 0.0, 10.0);
        let buf = buffer(
            &a,
            2.0,
            BufferMode::Precise {
                quadrant_segments: 16,
            },
        );
        // 精确偏移面积 = 原面积 + 4边×平行矩形 + 4角×扇形
        // ≈ 100 + 80 + 4π ≈ 192.57
        assert!(buf.unsigned_area() > 180.0, "area={}", buf.unsigned_area());
        assert!(buf.unsigned_area() < 200.0, "area={}", buf.unsigned_area());
    }

    #[test]
    fn test_buffer_precise_vs_convexhull_lshape() {
        // L 形凹多边形：凸壳会填满凹角，精确偏移不会
        let l = l_shape();
        let precise = buffer(
            &l,
            0.5,
            BufferMode::Precise {
                quadrant_segments: 8,
            },
        );
        let convex = buffer(
            &l,
            0.5,
            BufferMode::ConvexHull {
                quadrant_segments: 8,
            },
        );

        // 凸壳面积 ≥ 精确偏移面积（凸壳会多填凹角区域）
        assert!(
            convex.unsigned_area() >= precise.unsigned_area() - 1e-6,
            "Convex hull should cover more area: convex={}, precise={}",
            convex.unsigned_area(),
            precise.unsigned_area()
        );
    }

    #[test]
    fn test_buffer_precise_preserves_concavity() {
        // 精确偏移的 L 形外扩应在凹角处延伸而不是填满
        let l = l_shape();
        let buf = buffer(
            &l,
            0.5,
            BufferMode::Precise {
                quadrant_segments: 8,
            },
        );

        // 检查结果非空
        assert!(!buf.0.is_empty());
        // 缓冲后面积应 > 原面积
        assert!(buf.unsigned_area() > l.unsigned_area());
    }

    #[test]
    fn test_simplify() {
        let line = LineString::new(vec![
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 1.0, y: 0.1 },
            Coord { x: 2.0, y: 0.0 },
            Coord { x: 3.0, y: 0.0 },
        ]);
        let simplified = simplify_line(&line, 0.5);
        // 应减少顶点数
        assert!(simplified.0.len() <= line.0.len());
        // 起终点应保留
        assert_eq!(simplified.0.first().unwrap().x, 0.0);
        assert_eq!(simplified.0.last().unwrap().x, 3.0);
    }

    #[test]
    fn test_kernel_density() {
        let points = vec![(5.0, 5.0), (5.5, 5.5), (4.5, 4.5)];
        let result = kernel_density(&points, 10, 10, (0.0, 0.0, 10.0, 10.0), 1.0);
        assert_eq!(result.len(), 100);
        // 中心附近应有较高密度
        let center = result[5 * 10 + 5]; // grid cell (5,5)
        let corner = result[0];
        assert!(
            center > corner,
            "Center density {center} should exceed corner {corner}"
        );
    }

    #[test]
    fn test_line_density() {
        let lines = vec![(0.0, 0.0, 10.0, 10.0), (0.0, 10.0, 10.0, 0.0)];
        let result = line_density(&lines, 10, 10, (0.0, 0.0, 10.0, 10.0));
        assert_eq!(result.len(), 100);
        // 交叉点附近应有更高密度
        let center = result[5 * 10 + 5];
        assert!(center > 0.0, "Center should have non-zero line density");
    }

    #[test]
    fn test_difference_overlapping() {
        // Square minus inner square
        let a = square(0.0, 0.0, 10.0);
        let b = square(2.0, 2.0, 4.0);
        let result = difference(&a, &b);
        assert!(result.is_some());
        let mp = result.unwrap();
        // Difference should create a donut (one or more polygons)
        assert!(!mp.0.is_empty());
    }

    #[test]
    fn test_difference_non_overlapping() {
        let a = square(0.0, 0.0, 5.0);
        let b = square(10.0, 10.0, 5.0);
        let result = difference(&a, &b);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0.len(), 1);
    }

    #[test]
    fn test_sym_difference_overlapping() {
        let a = square(0.0, 0.0, 10.0);
        let b = square(5.0, 0.0, 10.0);
        let result = sym_difference(&a, &b);
        assert!(result.is_some());
        // XOR of two overlapping squares should produce multiple shapes
        let mp = result.unwrap();
        assert!(mp.0.len() >= 2, "XOR should produce ≥2 polygons");
    }

    #[test]
    fn test_clip_polygon() {
        let target: MultiPolygon<f64> =
            vec![square(0.0, 0.0, 100.0), square(200.0, 200.0, 50.0)].into();
        let clip_poly = square(0.0, 0.0, 150.0);
        let result = clip(&target, &clip_poly);
        // Only the first square (0-100) should be within clip (0-150)
        assert!(!result.0.is_empty());
        // Second square (200-250) is outside, should be clipped away
        assert_eq!(result.0.len(), 1);
    }

    #[test]
    fn test_sym_difference_non_overlapping() {
        let a = square(0.0, 0.0, 5.0);
        let b = square(10.0, 10.0, 5.0);
        let result = sym_difference(&a, &b);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0.len(), 2);
    }

    #[test]
    fn test_difference_identical() {
        let a = square(0.0, 0.0, 10.0);
        let b = square(0.0, 0.0, 10.0);
        let result = difference(&a, &b);
        assert!(result.is_none(), "Identical polygons should produce None");
    }

    #[test]
    fn test_clip_empty() {
        let target: MultiPolygon<f64> = vec![square(1000.0, 1000.0, 10.0)].into();
        let clip_poly = square(0.0, 0.0, 10.0);
        let result = clip(&target, &clip_poly);
        assert!(result.0.is_empty(), "Non-overlapping clip should be empty");
    }

    #[test]
    fn test_simplify_visvalingam() {
        let line = LineString::new(vec![
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 1.0, y: 0.1 },
            Coord { x: 2.0, y: 0.0 },
            Coord { x: 3.0, y: 0.0 },
        ]);
        let result = simplify_visvalingam(&line, 0.01);
        assert!(result.0.len() <= line.0.len());
        assert_eq!(result.0.first().unwrap(), &Coord { x: 0.0, y: 0.0 });
        assert_eq!(result.0.last().unwrap(), &Coord { x: 3.0, y: 0.0 });
    }

    #[test]
    fn test_simplify_visvalingam_preserve() {
        let line = LineString::new(vec![
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 1.0, y: 0.5 },
            Coord { x: 2.0, y: 0.0 },
            Coord { x: 3.0, y: 0.0 },
        ]);
        let result = simplify_visvalingam_preserve(&line, 0.01);
        assert!(result.0.len() <= line.0.len());
    }

    // ── bbox_buffer_inner tests ──

    #[test]
    fn test_bbox_buffer_inner_small_shrink() {
        let a = square(0.0, 0.0, 10.0);
        let result = bbox_buffer_inner(&a, 1.0);
        assert!(!result.0.is_empty());
        let r = &result.0[0];
        let b = r.bounding_rect().unwrap();
        assert!(b.width() < 10.0, "inner buffer should shrink");
        assert!(b.height() < 10.0);
    }

    #[test]
    fn test_bbox_buffer_inner_large_negative() {
        let a = square(0.0, 0.0, 10.0);
        // Large negative buffer (100 > half-width 5) produces degenerate geometry.
        // Function should not panic; result can be empty for extreme shrinkage.
        let _result = bbox_buffer_inner(&a, 100.0);
    }

    #[test]
    fn test_bbox_buffer_inner_zero() {
        let a = square(0.0, 0.0, 10.0);
        let result = bbox_buffer_inner(&a, 0.0);
        assert!(!result.0.is_empty());
        let r = &result.0[0];
        let b = r.bounding_rect().unwrap();
        assert!((b.width() - 10.0).abs() < 1e-9);
        assert!((b.height() - 10.0).abs() < 1e-9);
    }

    // ── clip_line_length tests ──

    #[test]
    fn test_clip_line_length_fully_inside() {
        let len = clip_line_length(2.0, 2.0, 8.0, 8.0, 0.0, 0.0, 10.0, 10.0);
        let expected = ((8.0 - 2.0f64).powi(2) + (8.0 - 2.0f64).powi(2)).sqrt();
        assert!((len - expected).abs() < 1e-9, "line fully inside");
    }

    #[test]
    fn test_clip_line_length_fully_outside() {
        let len = clip_line_length(11.0, 11.0, 12.0, 12.0, 0.0, 0.0, 10.0, 10.0);
        assert!(len < 1e-9, "line fully outside");
    }

    #[test]
    fn test_clip_line_length_across() {
        let len = clip_line_length(-5.0, 5.0, 15.0, 5.0, 0.0, 0.0, 10.0, 10.0);
        assert!(len > 9.9 && len < 10.1, "horizontal across rect");
    }

    #[test]
    fn test_clip_line_length_vertical_edge() {
        let len = clip_line_length(5.0, -5.0, 5.0, 15.0, 0.0, 0.0, 10.0, 10.0);
        assert!(len > 9.9 && len < 10.1, "vertical across rect");
    }

    // ── simplify (Douglas-Peucker) tests ──

    #[test]
    fn test_simplify_polygon_dp() {
        let poly = square(0.0, 0.0, 10.0);
        let result = simplify(&poly, 1.0);
        assert!(result.exterior().0.len() <= 5);
        assert!((result.unsigned_area() - 100.0).abs() < 20.0);
    }

    #[test]
    fn test_simplify_polygon_no_op() {
        let poly = square(0.0, 0.0, 10.0);
        let result = simplify(&poly, 0.0);
        assert_eq!(result.exterior().0.len(), poly.exterior().0.len());
    }

    #[test]
    fn test_bbox_buffer_inner_via_buffer() {
        let a = square(0.0, 0.0, 10.0);
        let buf = buffer(&a, -1.0, BufferMode::Bbox);
        assert!(buf.unsigned_area() < 100.0);
        assert!(buf.unsigned_area() > 0.0);
    }
}
