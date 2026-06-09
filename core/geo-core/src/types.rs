//! Geometry type aliases and validation helpers.
//!
//! Re-exports [`geo_types`] with convenience wrappers for common
//! geo-toolbox operations like coordinate validation and bounding boxes.

use geo_types::{Geometry, Point};
use serde::{Deserialize, Serialize};

pub use geo_types;

/// A spatial row ready for batch insertion into a spatial database.
///
/// Used by ingestion parsers and batch writers to pass geometry data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialRow {
    /// WKB bytes for the geometry column.
    pub wkb: Vec<u8>,
    /// JSON properties.
    pub properties: String,
    /// Data source identifier.
    pub source: String,
    /// CRS in "EPSG:nnnn" format.
    pub crs: String,
}

impl SpatialRow {
    /// Create from WKB bytes + optional JSON properties.
    pub fn new(wkb: Vec<u8>, properties: serde_json::Value, source: &str) -> Self {
        Self {
            wkb,
            properties: properties.to_string(),
            source: source.to_string(),
            crs: "EPSG:4326".to_string(),
        }
    }
}

/// A 2D bounding box.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BBox {
    /// Minimum longitude / x.
    pub min_x: f64,
    /// Minimum latitude / y.
    pub min_y: f64,
    /// Maximum longitude / x.
    pub max_x: f64,
    /// Maximum latitude / y.
    pub max_y: f64,
}

impl BBox {
    /// Create a new bounding box. Pairs are sorted so min ≤ max.
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self {
            min_x: x1.min(x2),
            min_y: y1.min(y2),
            max_x: x1.max(x2),
            max_y: y1.max(y2),
        }
    }

    /// Width in x-direction.
    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    /// Height in y-direction.
    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
    }

    /// Does this bbox contain the point (x, y)?
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }
}

/// Validate a single (lon, lat) coordinate pair.
///
/// Returns `Ok(())` if lon ∈ [-180, 180] and lat ∈ [-90, 90].
pub fn validate_coord(lon: f64, lat: f64) -> Result<(), crate::GeoError> {
    if !(-180.0..=180.0).contains(&lon) || !(-90.0..=90.0).contains(&lat) {
        Err(crate::GeoError::Validation(format!(
            "Coordinate out of range: lon={}, lat={}",
            lon, lat
        )))
    } else {
        Ok(())
    }
}

/// Build a [`Point`] from (lon, lat), validating the coordinate first.
pub fn point(lon: f64, lat: f64) -> Result<Point<f64>, crate::GeoError> {
    validate_coord(lon, lat)?;
    Ok(Point::new(lon, lat))
}

/// Simple WKT-to-Geometry conversion for common geometry types.
///
/// This is a minimal parser for WKT point/line/polygon strings.
/// For production use, enable the `wkt` feature or use GDAL.
pub fn from_wkt(wkt_str: &str) -> Result<Geometry<f64>, crate::GeoError> {
    let trimmed = wkt_str.trim();

    if let Some(coords) = trimmed.strip_prefix("POINT(").and_then(|s| s.strip_suffix(")")) {
        let parts: Vec<&str> = coords.split_whitespace().collect();
        if parts.len() >= 2 {
            let x: f64 = parts[0]
                .parse()
                .map_err(|e| crate::GeoError::Validation(format!("bad x: {e}")))?;
            let y: f64 = parts[1]
                .parse()
                .map_err(|e| crate::GeoError::Validation(format!("bad y: {e}")))?;
            return Ok(Geometry::Point(Point::new(x, y)));
        }
    }

    Err(crate::GeoError::Validation(format!(
        "unsupported WKT: {trimmed}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_coord_valid() {
        assert!(validate_coord(113.9, 22.5).is_ok());
    }

    #[test]
    fn test_validate_coord_invalid_lat() {
        assert!(validate_coord(0.0, 100.0).is_err());
    }

    #[test]
    fn test_bbox_contains() {
        let bb = BBox::new(10.0, 20.0, 30.0, 40.0);
        assert!(bb.contains(20.0, 30.0));
        assert!(!bb.contains(0.0, 0.0));
    }

    #[test]
    fn test_from_wkt_point() {
        let g = from_wkt("POINT(113.9 22.5)").unwrap();
        match g {
            Geometry::Point(p) => {
                assert!((p.x() - 113.9).abs() < 0.001);
                assert!((p.y() - 22.5).abs() < 0.001);
            }
            _ => panic!("expected Point"),
        }
    }

    #[test]
    fn test_point_valid() {
        let p = point(113.9, 22.5).unwrap();
        assert!((p.x() - 113.9).abs() < 0.001);
        assert!((p.y() - 22.5).abs() < 0.001);
    }

    #[test]
    fn test_point_invalid() {
        assert!(point(0.0, 100.0).is_err());
    }
}
