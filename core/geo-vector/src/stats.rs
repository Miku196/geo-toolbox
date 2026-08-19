//! 矢量统计：面积、质心、长度、密度。

use geo::algorithm::Area;
use geo::algorithm::Centroid;
use geo_types::{Coord, MultiPolygon, Point, Polygon};

/// 一纬度差的近似米数（赤道子午线方向，约 111.32 km/度）。
const METERS_PER_DEGREE: f64 = 111_320.0;

/// 计算多边形质心。
pub fn centroid(poly: &Polygon<f64>) -> Option<Point<f64>> {
    poly.centroid()
}

/// 计算多边形角度面积（单位：平方度 sq_deg）。
///
/// 注意：这是**度数**面积，不是米制面积。米制面积请用 [`feature_area_m2`] /
/// [`feature_area_ha`]，避免把度数平方当作 m²/ha。
pub fn feature_area(poly: &Polygon<f64>) -> f64 {
    poly.unsigned_area()
}

/// [`feature_area`] 的显式命名别名：角度面积（平方度 sq_deg），
/// 与米制面积明确区分、避免混淆。
pub fn feature_area_sq_deg(poly: &Polygon<f64>) -> f64 {
    feature_area(poly)
}

/// 等距圆柱（Plate Carrée）投影：以多边形平均纬度为基准，把 (lon, lat)
/// 度坐标近似换算为平面米制坐标。经度方向按 `cos(平均纬度)` 收缩，
/// 纬度方向按 `METERS_PER_DEGREE` 米/度。
fn project_to_meters(coords: &[Coord<f64>]) -> Vec<Coord<f64>> {
    if coords.is_empty() {
        return Vec::new();
    }
    let mid_lat: f64 = coords.iter().map(|c| c.y).sum::<f64>() / coords.len() as f64;
    let cos_mid = mid_lat.to_radians().cos();
    coords
        .iter()
        .map(|c| Coord {
            x: c.x * METERS_PER_DEGREE * cos_mid,
            y: c.y * METERS_PER_DEGREE,
        })
        .collect()
}

/// 鞋带公式面积（返回绝对值）。
fn shoelace_area(coords: &[Coord<f64>]) -> f64 {
    let n = coords.len();
    if n < 3 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        s += coords[i].x * coords[j].y - coords[j].x * coords[i].y;
    }
    s.abs() * 0.5
}

/// 计算多边形真实米制面积（m²），基于等距圆柱近似。
/// 1°×1° 赤道正方形 ≈ (111_320 m)² ≈ 1.24e10 m²。
pub fn feature_area_m2(poly: &Polygon<f64>) -> f64 {
    shoelace_area(&project_to_meters(&poly.exterior().0))
}

/// 计算多边形真实米制面积（公顷 ha，ha = m² / 10_000）。
pub fn feature_area_ha(poly: &Polygon<f64>) -> f64 {
    feature_area_m2(poly) / 10_000.0
}

/// 多面角度总面积（平方度）。
pub fn multi_area(mp: &MultiPolygon<f64>) -> f64 {
    mp.iter().map(|p| p.unsigned_area()).sum()
}

/// 多面真实米制总面积（m²）。
pub fn multi_area_m2(mp: &MultiPolygon<f64>) -> f64 {
    mp.iter().map(feature_area_m2).sum()
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

    /// 1°×1° 正方形 @ 赤道: 真实米制面积 ≈ 1°经度(111.32km) × 1°纬度(111.32km)。
    /// 宽 1° 经度在赤道 = 111.32 km，高 1° 纬度 = 111.32 km → ≈ 1.239e10 m²。
    #[test]
    fn test_feature_area_m2_equator_1x1() {
        let poly = Polygon::new(
            LineString::new(vec![
                Coord { x: 0.0, y: 0.0 },
                Coord { x: 1.0, y: 0.0 },
                Coord { x: 1.0, y: 1.0 },
                Coord { x: 0.0, y: 1.0 },
                Coord { x: 0.0, y: 0.0 },
            ]),
            vec![],
        );
        let m2 = feature_area_m2(&poly);
        // 1.23e10 量级
        assert!(
            m2 > 1.20e10 && m2 < 1.30e10,
            "1x1 deg at equator should be ~1.23e10 m2, got {m2}"
        );
        // ha = m2 / 10000
        assert!((feature_area_ha(&poly) - m2 / 10000.0).abs() < 1.0);
        // 度面积仍是度数，不应与米制混淆
        let sq_deg = feature_area_sq_deg(&poly);
        assert!((sq_deg - 1.0).abs() < 1e-9, "1x1 deg square is 1 sq deg, got {sq_deg}");
    }

    /// 米制可验证矩形: 经度跨度 dLon 经米制换算后，随着纬度升高面积应减小（cos 校正）。
    #[test]
    fn test_feature_area_m2_scale_with_latitude() {
        let mk = |y0: f64, y1: f64| -> Polygon<f64> {
            Polygon::new(
                LineString::new(vec![
                    Coord { x: 0.0, y: y0 },
                    Coord { x: 1.0, y: y0 },
                    Coord { x: 1.0, y: y1 },
                    Coord { x: 0.0, y: y1 },
                    Coord { x: 0.0, y: y0 },
                ]),
                vec![],
            )
        };
        let a0 = feature_area_m2(&mk(0.0, 1.0));
        let a60 = feature_area_m2(&mk(60.0, 61.0));
        assert!(
            a0 > a60,
            "equator area {a0} should exceed high-latitude area {a60}"
        );
    }
}
