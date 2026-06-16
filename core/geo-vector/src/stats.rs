//! 矢量统计：面积、质心、长度、密度。

use geo::algorithm::Area;
use geo::algorithm::Centroid;
use geo_types::{MultiPolygon, Point, Polygon};

/// 计算多边形质心。
pub fn centroid(poly: &Polygon<f64>) -> Option<Point<f64>> {
    poly.centroid()
}

/// 计算多边形面积（sq degrees，需乘以纬度因子转为公顷）。
pub fn feature_area(poly: &Polygon<f64>) -> f64 {
    poly.unsigned_area()
}

/// 多面总面积。
pub fn multi_area(mp: &MultiPolygon<f64>) -> f64 {
    mp.iter().map(|p| p.unsigned_area()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo_types::{Coord, LineString};

    #[test]
    fn test_area() {
        let poly = Polygon::new(
            LineString::new(vec![
                Coord { x: 104.0, y: 30.5 },
                Coord { x: 104.1, y: 30.5 },
                Coord { x: 104.1, y: 30.6 },
                Coord { x: 104.0, y: 30.6 },
                Coord { x: 104.0, y: 30.5 },
            ]),
            vec![],
        );
        let area = feature_area(&poly);
        assert!(area > 0.0);
    }
}
