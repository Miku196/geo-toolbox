//! 矢量运算：缓冲区、相交、合并、裁剪。
//!
//! 基于 geo crate 的 BooleanOps（相交/合并）和 BoundingRect（bbox 缓冲）。

use geo::algorithm::{Area, BoundingRect, BooleanOps};
use geo_types::{Coord, LineString, MultiPolygon, Polygon};

/// 对多边形做 bbox 缓冲（axis-aligned bounding box expansion）。
///
/// 不是真实地理缓冲区，而是按 distance 外扩 bbox 的矩形。
/// 适用于快速 AOI 扩展、粗筛等场景。
pub fn buffer(poly: &Polygon<f64>, distance: f64) -> Polygon<f64> {
    if distance <= 0.0 {
        return poly.clone();
    }
    let bbox = match poly.bounding_rect() {
        Some(r) => r,
        None => return poly.clone(),
    };
    let min = bbox.min();
    let max = bbox.max();
    Polygon::new(
        LineString::new(vec![
            Coord { x: min.x - distance, y: min.y - distance },
            Coord { x: max.x + distance, y: min.y - distance },
            Coord { x: max.x + distance, y: max.y + distance },
            Coord { x: min.x - distance, y: max.y + distance },
            Coord { x: min.x - distance, y: min.y - distance },
        ]),
        vec![],
    )
}

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
            ra.min().x < rb.max().x && ra.max().x > rb.min().x
                && ra.min().y < rb.max().y && ra.max().y > rb.min().y
        }
        _ => false,
    }
}

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
    fn test_buffer_positive() {
        let a = square(0.0, 0.0, 5.0);
        let buf = buffer(&a, 2.0);
        assert!(buf.unsigned_area() > a.unsigned_area());
    }

    #[test]
    fn test_buffer_negative() {
        let a = square(0.0, 0.0, 5.0);
        let buf = buffer(&a, -1.0);
        assert_eq!(buf.unsigned_area(), a.unsigned_area());
    }
}
