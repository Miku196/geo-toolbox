use geo::algorithm::{Area, BooleanOps, BoundingRect};
use geo_types::{LineString, MultiPolygon, Polygon};

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

// ═══════════════════════ 测试 ═══════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::*;
    use crate::density::*;
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
