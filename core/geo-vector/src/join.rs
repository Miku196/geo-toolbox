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

use geo::algorithm::Contains;
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
}
