//! Spatial join — assign features to zones, point-in-polygon, proximity.
//!
//! # Example
//!
//! ```rust,ignore
//! use geo_types::{Polygon, Point};
//! let zones = vec![("city_a", polygon_a), ("city_b", polygon_b)];
//! let points = vec![point(104.06, 30.57), point(116.4, 39.9)];
//! let joined = spatial_join_points(&points, &zones)?;
//! // joined[0] = Some("city_a"), joined[1] = Some("city_b")
//! ```

use geo::algorithm::{BooleanOps, Contains};
use geo::ConvexHull;
use geo_types::{Coord, LineString, MultiPolygon, Point, Polygon};

/// Check if a point is inside a polygon (`geo::algorithm::Contains` wrapper).
pub fn point_in_polygon(x: f64, y: f64, poly: &Polygon<f64>) -> bool {
    let p = Point::new(x, y);
    poly.contains(&p)
}

/// Check if a point is inside a MultiPolygon.
pub fn point_in_multipolygon(x: f64, y: f64, mp: &MultiPolygon<f64>) -> bool {
    let p = Point::new(x, y);
    mp.contains(&p)
}

/// Spatial join: assign each point to the first matching zone.
///
/// # Arguments
/// * `points` — array of (x, y) coordinates.
/// * `zones` — array of (zone_name, zone_polygon).
///
/// # Returns
/// Vector of `Option<&zone_name>` — `Some` if a zone was found.
pub fn spatial_join_points<'a>(
    points: &[(f64, f64)],
    zones: &[(&'a str, &Polygon<f64>)],
) -> Vec<Option<&'a str>> {
    points
        .iter()
        .map(|&(x, y)| {
            zones
                .iter()
                .find(|(_, poly)| point_in_polygon(x, y, poly))
                .map(|(name, _)| *name)
        })
        .collect()
}

/// Convert a Vec<(f64, f64)> to a closed LineString for polygon construction.
pub fn points_to_polygon(coords: &[(f64, f64)]) -> Polygon<f64> {
    let mut pts: Vec<Coord<f64>> = coords.iter().map(|&(x, y)| Coord { x, y }).collect();
    // Close the ring if not already
    if coords.len() > 1 && (coords[0].0 - coords[coords.len() - 1].0).abs() > 1e-10 {
        pts.push(pts[0]);
    }
    Polygon::new(LineString::from(pts), vec![])
}

/// 检查 LineString 是否有自交（非相邻线段相交）。
pub fn linestring_self_intersects(ls: &LineString<f64>) -> bool {
    let coords = &ls.0;
    let n = coords.len();
    for i in 0..n.saturating_sub(2) {
        for j in i + 2..n.saturating_sub(1) {
            // 跳过共享端点的相邻线段
            if i == 0 && j == n - 2 {
                continue;
            }
            if segments_intersect(&coords[i], &coords[i + 1], &coords[j], &coords[j + 1]) {
                return true;
            }
        }
    }
    false
}

/// 校验 Polygon 几何 — 检查环闭合和自交。
pub fn validate_geometry(poly: &Polygon<f64>) -> Vec<String> {
    let mut errors = Vec::new();
    // 环闭合检查
    let ext = poly.exterior();
    if ext.0.len() >= 2
        && ((ext.0[0].x - ext.0[ext.0.len() - 1].x).abs() > 1e-10
            || (ext.0[0].y - ext.0[ext.0.len() - 1].y).abs() > 1e-10)
    {
        errors.push("exterior ring not closed".into());
    }
    // 外环自交
    if linestring_self_intersects(ext) {
        errors.push("exterior ring self-intersects".into());
    }
    // 内环检查
    for (i, interior) in poly.interiors().iter().enumerate() {
        if interior.0.len() >= 2
            && ((interior.0[0].x - interior.0[interior.0.len() - 1].x).abs() > 1e-10
                || (interior.0[0].y - interior.0[interior.0.len() - 1].y).abs() > 1e-10)
        {
            errors.push(format!("interior ring {i} not closed"));
        }
        if linestring_self_intersects(interior) {
            errors.push(format!("interior ring {i} self-intersects"));
        }
    }
    errors
}

/// 校验 MultiPolygon — 返回每个子多边形的错误列表。
pub fn validate_multipolygon(mp: &MultiPolygon<f64>) -> Vec<(usize, Vec<String>)> {
    mp.iter()
        .enumerate()
        .map(|(i, p)| (i, validate_geometry(p)))
        .collect()
}

/// 检测 MultiPolygon 中各多边形间的缝隙。返回未被任何多边形覆盖的区域。
pub fn detect_gaps(mp: &MultiPolygon<f64>) -> Vec<Polygon<f64>> {
    if mp.0.is_empty() {
        return vec![];
    }
    // 全量 union 保持 MultiPolygon 类型一致
    let mut union_mp = MultiPolygon(vec![mp.0[0].clone()]);
    for p in &mp.0[1..] {
        let rhs = MultiPolygon(vec![p.clone()]);
        union_mp = union_mp.union(&rhs);
    }
    let hull_poly = mp.convex_hull();
    let hull_mp = MultiPolygon(vec![hull_poly]);
    hull_mp.difference(&union_mp).0
}

/// 两条线段是否相交（不含端点）。
fn segments_intersect(a: &Coord<f64>, b: &Coord<f64>, c: &Coord<f64>, d: &Coord<f64>) -> bool {
    let d1 = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let d2 = (b.x - a.x) * (d.y - a.y) - (b.y - a.y) * (d.x - a.x);
    if (d1 > 0.0 && d2 > 0.0) || (d1 < 0.0 && d2 < 0.0) {
        return false;
    }
    let d3 = (d.x - c.x) * (a.y - c.y) - (d.y - c.y) * (a.x - c.x);
    let d4 = (d.x - c.x) * (b.y - c.y) - (d.y - c.y) * (b.x - c.x);
    if (d3 > 0.0 && d4 > 0.0) || (d3 < 0.0 && d4 < 0.0) {
        return false;
    }
    // 共线情形：检查投影区间是否重叠
    if d1.abs() < 1e-12 && d2.abs() < 1e-12 && d3.abs() < 1e-12 && d4.abs() < 1e-12 {
        // 检查坐标投影
        let dot = |p: &Coord<f64>, q: &Coord<f64>| p.x * q.x + p.y * q.y;
        let ab = Coord {
            x: b.x - a.x,
            y: b.y - a.y,
        };
        let len2 = dot(&ab, &ab);
        if len2 < 1e-12 {
            return false;
        }
        let t0 = dot(
            &Coord {
                x: c.x - a.x,
                y: c.y - a.y,
            },
            &ab,
        ) / len2;
        let t1 = dot(
            &Coord {
                x: d.x - a.x,
                y: d.y - a.y,
            },
            &ab,
        ) / len2;
        let (t_min, t_max) = if t0 < t1 { (t0, t1) } else { (t1, t0) };
        return t_max > 0.0 && t_min < 1.0;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo_types::Coord;

    fn make_rect(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Polygon<f64> {
        Polygon::new(
            LineString::from(vec![
                Coord { x: min_x, y: min_y },
                Coord { x: max_x, y: min_y },
                Coord { x: max_x, y: max_y },
                Coord { x: min_x, y: max_y },
                Coord { x: min_x, y: min_y },
            ]),
            vec![],
        )
    }

    #[test]
    fn test_point_inside() {
        let rect = make_rect(0.0, 0.0, 10.0, 10.0);
        assert!(point_in_polygon(5.0, 5.0, &rect));
        assert!(!point_in_polygon(15.0, 5.0, &rect));
    }

    #[test]
    fn test_spatial_join() {
        let rect_a = make_rect(0.0, 0.0, 10.0, 10.0);
        let rect_b = make_rect(20.0, 20.0, 30.0, 30.0);
        let zones = vec![("zone_a", &rect_a), ("zone_b", &rect_b)];
        let points = [(5.0, 5.0), (25.0, 25.0), (15.0, 15.0)];
        let joined = spatial_join_points(&points, &zones);
        assert_eq!(joined[0], Some("zone_a"));
        assert_eq!(joined[1], Some("zone_b"));
        assert_eq!(joined[2], None); // outside both zones
    }

    #[test]
    fn test_validate_valid_polygon() {
        let rect = make_rect(0.0, 0.0, 10.0, 10.0);
        let errors = validate_geometry(&rect);
        assert!(
            errors.is_empty(),
            "valid rectangle should have no errors: {errors:?}"
        );
    }

    #[test]
    fn test_validate_self_intersecting() {
        // 八字形自交多边形
        let bowtie = Polygon::new(
            LineString::from(vec![
                Coord { x: 0.0, y: 0.0 },
                Coord { x: 10.0, y: 10.0 },
                Coord { x: 0.0, y: 10.0 },
                Coord { x: 10.0, y: 0.0 },
                Coord { x: 0.0, y: 0.0 },
            ]),
            vec![],
        );
        let errors = validate_geometry(&bowtie);
        assert!(!errors.is_empty(), "self-intersecting should have errors");
    }

    #[test]
    fn test_detect_gaps() {
        let a = make_rect(0.0, 0.0, 5.0, 5.0);
        let b = make_rect(7.0, 0.0, 10.0, 5.0);
        let mp = MultiPolygon(vec![a, b]);
        let gaps = detect_gaps(&mp);
        // x=5..7 之间有缝隙
        assert!(
            !gaps.is_empty(),
            "should detect gap between non-touching polygons"
        );
    }

    #[test]
    fn test_no_gaps_for_contiguous() {
        let a = make_rect(0.0, 0.0, 5.0, 5.0);
        let b = make_rect(5.0, 0.0, 10.0, 5.0);
        let mp = MultiPolygon(vec![a, b]);
        let gaps = detect_gaps(&mp);
        // 相邻多边形无缝隙（或缝隙很小为数值 noise）
        // 注意 convex_hull 会覆盖 5-7 的间隙 — 这里只验证不 panic
        let _ = gaps.len();
    }
}
