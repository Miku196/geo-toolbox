//! GeoFeature type — a lightweight wrapper for geometry + landcover class.
//!
//! Unlike full GeoJSON Feature, this is a minimal struct optimized for
//! carbon calculation throughput.

use geo::algorithm::area::Area;
use geo_types::{Geometry, Polygon, MultiPolygon};
/// A spatial feature with landcover classification.
///
/// The geometry must be a Polygon or MultiPolygon in EPSG:4326 (WGS84).
/// Area is computed in the engine using equirectangular approximation.
#[derive(Debug, Clone)]
pub struct GeoFeature {
    /// Landcover class identifier (e.g., "forest", "grassland").
    pub landcover_class: String,
    /// GeoJSON geometry as a parsed Rust geometry.
    pub geometry: Geometry<f64>,
}

impl GeoFeature {
    /// Create a new feature from a landcover class and GeoJSON geometry string.
    pub fn new(class: impl Into<String>, geojson_geometry: &str) -> Result<Self, String> {
        let geom_value: serde_json::Value = serde_json::from_str(geojson_geometry)
            .map_err(|e| format!("Invalid JSON: {e}"))?;

        let geometry = parse_geojson_geometry(&geom_value)?;

        Ok(Self {
            landcover_class: class.into(),
            geometry,
        })
    }

    /// Create a feature from a pre-parsed Geometry and class name.
    pub fn from_geometry(class: impl Into<String>, geometry: Geometry<f64>) -> Self {
        Self {
            landcover_class: class.into(),
            geometry,
        }
    }

    /// Parse from a full GeoJSON Feature (with properties.class/category/landcover).
    /// Also checks `properties.subcategory` for finer factor matching.
    pub fn from_feature_json(feature_json: &str) -> Result<Self, String> {
        let feat: serde_json::Value = serde_json::from_str(feature_json)
            .map_err(|e| format!("Invalid JSON: {e}"))?;

        let props = &feat["properties"];
        // Try multiple property names for landcover class
        let class = props["class"].as_str()
            .or_else(|| props["category"].as_str())
            .or_else(|| props["landcover"].as_str())
            .or_else(|| props["landuse"].as_str())
            .unwrap_or("unknown");

        // Subcategory for granular factor matching
        let subcategory: Option<String> = props["subcategory"].as_str()
            .map(|s| s.to_string());

        // Build composite key: "category:subcategory" or "category"
        let landcover_class = match subcategory {
            Some(ref sub) if !sub.is_empty() => format!("{}:{}", class, sub),
            _ => class.to_string(),
        };

        let geometry = parse_geojson_geometry(&feat["geometry"])?;

        Ok(Self { landcover_class, geometry })
    }

    /// Compute area in hectares.
    ///
    /// Uses equirectangular approximation for WGS84 geometries.
    /// For accurate results, reproject to EPSG:3405 first.
    pub fn area_ha(&self) -> f64 {
        compute_area_ha(&self.geometry)
    }
}

/// Compute approximate area in hectares for a WGS84 geometry.
///
/// Uses equirectangular approximation with latitude-dependent scaling.
/// For carbon accounting within ±3% accuracy for areas < 10,000 km².
pub fn compute_area_ha(geom: &Geometry<f64>) -> f64 {
    use geo::algorithm::centroid::Centroid;
    
    let area_sq_deg = match geom {
        Geometry::Polygon(p) => p.unsigned_area(),
        Geometry::MultiPolygon(mp) => mp.unsigned_area(),
        _ => return 0.0,
    };

    // Scale by centroid latitude: 1° lon ≈ 111.32km × cos(lat)
    //  1° lat ≈ 111.32km (nearly constant)
    let centroid = geom.centroid().unwrap_or(geo_types::point!(x:0.0, y:0.0));
    let lat_mid = centroid.y().to_radians();
    let m_per_deg_lat = 111_320.0;
    let m_per_deg_lon = 111_320.0 * lat_mid.cos();
    let m2_per_sq_deg = m_per_deg_lat * m_per_deg_lon;
    
    area_sq_deg * m2_per_sq_deg / 10_000.0  // m² → ha
}

/// Parse a GeoJSON geometry object into a geo_types::Geometry.
fn parse_geojson_geometry(value: &serde_json::Value) -> Result<Geometry<f64>, String> {
    let geom_type = value["type"].as_str()
        .ok_or_else(|| "Missing geometry type".to_string())?;
    let coords = &value["coordinates"];

    match geom_type {
        "Polygon" => {
            let polygon = parse_polygon(coords)?;
            Ok(Geometry::Polygon(polygon))
        }
        "MultiPolygon" => {
            let mp = parse_multi_polygon(coords)?;
            Ok(Geometry::MultiPolygon(mp))
        }
        _ => Err(format!("Unsupported geometry type: {geom_type}")),
    }
}

fn parse_polygon(coords: &serde_json::Value) -> Result<Polygon<f64>, String> {
    let rings = coords.as_array()
        .ok_or("Polygon coords not an array")?;
    if rings.is_empty() {
        return Err("Polygon has no rings".into());
    }
    let exterior = parse_ring(&rings[0])?;
    let interiors: Vec<_> = rings[1..]
        .iter()
        .filter_map(|r| parse_ring(r).ok())
        .collect();
    Ok(Polygon::new(exterior, interiors))
}

fn parse_multi_polygon(coords: &serde_json::Value) -> Result<MultiPolygon<f64>, String> {
    let polys = coords.as_array()
        .ok_or("MultiPolygon coords not an array")?;
    let polygons: Vec<Polygon<f64>> = polys
        .iter()
        .map(|p| parse_polygon(p))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(MultiPolygon::new(polygons))
}

fn parse_ring(ring: &serde_json::Value) -> Result<geo_types::LineString<f64>, String> {
    let points = ring.as_array()
        .ok_or("Ring is not an array")?;
    let coords: Vec<geo_types::Coord<f64>> = points
        .iter()
        .filter_map(|p| {
            let arr = p.as_array()?;
            if arr.len() >= 2 {
                Some(geo_types::Coord {
                    x: arr[0].as_f64()?,
                    y: arr[1].as_f64()?,
                })
            } else { None }
        })
        .collect();
    if coords.len() < 3 {
        return Err("Ring has fewer than 3 points".into());
    }
    Ok(geo_types::LineString::new(coords))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_feature() {
        let json = r#"{
            "type": "Feature",
            "properties": { "class": "forest" },
            "geometry": {
                "type": "Polygon",
                "coordinates": [[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]
            }
        }"#;
        let feat = GeoFeature::from_feature_json(json).unwrap();
        assert_eq!(feat.landcover_class, "forest");
        assert!(feat.area_ha() > 0.0);
    }
}
