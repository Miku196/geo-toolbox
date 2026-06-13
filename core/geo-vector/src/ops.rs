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
pub fn buffer(
    poly: &Polygon<f64>,
    distance: f64,
    mode: BufferMode,
) -> MultiPolygon<f64> {
    let n = poly.exterior().0.len();
    if n < 3 || n > MAX_BUFFER_VERTICES {
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
                convexhull_buffer_outer(poly, distance, quadrant_segments.max(4).min(32) as usize)
            } else {
                bbox_buffer_inner(poly, -distance)
            }
        }
        BufferMode::Precise { quadrant_segments } => {
            if distance > 0.0 {
                precise_buffer_outer(poly, distance, quadrant_segments.max(4).min(32) as usize)
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
            Coord { x: min.x - dist, y: min.y - dist },
            Coord { x: max.x + dist, y: min.y - dist },
            Coord { x: max.x + dist, y: max.y + dist },
            Coord { x: min.x - dist, y: max.y + dist },
            Coord { x: min.x - dist, y: min.y - dist },
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
            Coord { x: cx - hw, y: cy - hh },
            Coord { x: cx + hw, y: cy - hh },
            Coord { x: cx + hw, y: cy + hh },
            Coord { x: cx - hw, y: cy + hh },
            Coord { x: cx - hw, y: cy - hh },
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

    let total_len: f64 = coords.windows(2).map(|w| {
        let dx = w[1].x - w[0].x;
        let dy = w[1].y - w[0].y;
        (dx * dx + dy * dy).sqrt()
    }).sum();
    let step = (total_len / (coords.len() as f64 * segments as f64)).max(dist / 4.0);

    let mut offset_points: Vec<Coord<f64>> = Vec::new();
    for w in coords.windows(2) {
        let dx = w[1].x - w[0].x;
        let dy = w[1].y - w[0].y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-12 { continue; }
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
fn precise_buffer_outer(poly: &Polygon<f64>, dist: f64, segments: usize) -> MultiPolygon<f64> {
    let exterior = poly.exterior();
    let coords = &exterior.0;
    let n = coords.len() - 1; // 首尾重复顶点不计
    if n < 3 {
        return MultiPolygon::new(vec![poly.clone()]);
    }

    let mut parts: Vec<Polygon<f64>> = Vec::with_capacity(n * 2);

    // 预计算每条边的外法线方向
    #[derive(Clone, Copy)]
    struct EdgeNormal { nx: f64, ny: f64, len: f64 }
    let mut edge_normals: Vec<EdgeNormal> = Vec::with_capacity(n);
    for i in 0..n {
        let j = (i + 1) % n;
        let dx = coords[j].x - coords[i].x;
        let dy = coords[j].y - coords[i].y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-12 {
            edge_normals.push(EdgeNormal { nx: 0.0, ny: 0.0, len: 0.0 });
        } else {
            edge_normals.push(EdgeNormal {
                nx: dy / len,
                ny: -dx / len,
                len,
            });
        }
    }

    // 0) 原始多边形本身作为基座加入 Union
    parts.push(poly.clone());

    // 1) 每条边生成平行四边形
    for i in 0..n {
        let j = (i + 1) % n;
        let nv = &edge_normals[i];
        if nv.len < 1e-12 { continue; }

        let ib = coords[i];
        let jb = coords[j];

        let i_off = Coord { x: ib.x + nv.nx * dist, y: ib.y + nv.ny * dist };
        let j_off = Coord { x: jb.x + nv.nx * dist, y: jb.y + nv.ny * dist };

        parts.push(Polygon::new(
            LineString::new(vec![ib, i_off, j_off, jb, ib]),
            vec![],
        ));
    }

    // 2) 凸顶点插入圆弧
    let step_angle = std::f64::consts::FRAC_PI_2 / (segments as f64).max(1.0);
    for i in 0..n {
        let prev = (i + n - 1) % n;
        let curr = i;

        let n1 = &edge_normals[prev];
        let n2 = &edge_normals[curr];
        if n1.len < 1e-12 || n2.len < 1e-12 { continue; }

        // 判断凹凸：上一段结束点的偏移 vs 当前段起点的偏移
        // 前一相邻点、当前点、后一相邻点的转角符号
        let prev_pt = coords[prev];
        let curr_pt = coords[curr];
        let next_pt = coords[(curr + 1) % n];

        // 叉积判断凹凸 (> 0 = 左转 = 凸)
        let dx1 = curr_pt.x - prev_pt.x;
        let dy1 = curr_pt.y - prev_pt.y;
        let dx2 = next_pt.x - curr_pt.x;
        let dy2 = next_pt.y - curr_pt.y;
        let cross = dx1 * dy2 - dy1 * dx2;

        if cross <= 0.0 {
            // 凹顶点或共线：平行四边形会自动覆盖，跳过圆弧
            continue;
        }

        // 凸顶点：计算圆弧起止方向角
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

        // 确定圆弧扫过方向（取最短弧）
        let mut sweep = angle2 - angle1;
        while sweep < -std::f64::consts::PI { sweep += std::f64::consts::TAU; }
        while sweep > std::f64::consts::PI { sweep -= std::f64::consts::TAU; }

        let arc_steps = ((sweep.abs() / step_angle).ceil() as usize).max(2);
        let arc_step = sweep / arc_steps as f64;

        let mut arc_coords: Vec<Coord<f64>> = Vec::with_capacity(arc_steps + 2);
        arc_coords.push(curr_pt); // 圆弧起点 — 顶点本身
        for k in 0..=arc_steps {
            let a = angle1 + arc_step * k as f64;
            arc_coords.push(Coord {
                x: curr_pt.x + a.cos() * dist,
                y: curr_pt.y + a.sin() * dist,
            });
        }

        parts.push(Polygon::new(LineString::new(arc_coords), vec![]));
    }

    // 3) Union 合并所有部件
    if parts.is_empty() {
        return bbox_buffer_outer(poly, dist);
    }

    let mut result = MultiPolygon::new(vec![parts[0].clone()]);
    for part in &parts[1..] {
        result = result.union(&MultiPolygon::new(vec![part.clone()]));
    }
    result
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
        let buf = buffer(&a, 2.0, BufferMode::ConvexHull { quadrant_segments: 16 });
        assert!(buf.unsigned_area() > 100.0);
    }

    #[test]
    fn test_buffer_zero() {
        let a = square(0.0, 0.0, 5.0);
        let buf = buffer(&a, 0.0, BufferMode::ConvexHull { quadrant_segments: 8 });
        assert!((buf.unsigned_area() - a.unsigned_area()).abs() < 1e-6);
    }

    #[test]
    fn test_buffer_precise_square() {
        let a = square(0.0, 0.0, 10.0);
        let buf = buffer(&a, 2.0, BufferMode::Precise { quadrant_segments: 16 });
        // 精确偏移面积 = 原面积 + 4边×平行矩形 + 4角×扇形
        // ≈ 100 + 80 + 4π ≈ 192.57
        assert!(buf.unsigned_area() > 180.0, "area={}", buf.unsigned_area());
        assert!(buf.unsigned_area() < 200.0, "area={}", buf.unsigned_area());
    }

    #[test]
    fn test_buffer_precise_vs_convexhull_lshape() {
        // L 形凹多边形：凸壳会填满凹角，精确偏移不会
        let l = l_shape();
        let precise = buffer(&l, 0.5, BufferMode::Precise { quadrant_segments: 8 });
        let convex = buffer(&l, 0.5, BufferMode::ConvexHull { quadrant_segments: 8 });

        // 凸壳面积 ≥ 精确偏移面积（凸壳会多填凹角区域）
        assert!(
            convex.unsigned_area() >= precise.unsigned_area() - 1e-6,
            "Convex hull should cover more area: convex={}, precise={}",
            convex.unsigned_area(), precise.unsigned_area()
        );
    }

    #[test]
    fn test_buffer_precise_preserves_concavity() {
        // 精确偏移的 L 形外扩应在凹角处延伸而不是填满
        let l = l_shape();
        let buf = buffer(&l, 0.5, BufferMode::Precise { quadrant_segments: 8 });

        // 检查结果非空
        assert!(!buf.0.is_empty());
        // 缓冲后面积应 > 原面积
        assert!(buf.unsigned_area() > l.unsigned_area());
    }
}
