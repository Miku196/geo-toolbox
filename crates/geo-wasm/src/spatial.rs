//! Browser-side spatial operations using the `geo` crate.
//!
//! All functions return JSON strings for reliable JS interop.

use wasm_bindgen::prelude::*;

use geo::algorithm::area::Area;
use geo::algorithm::bounding_rect::BoundingRect;
use geo::algorithm::centroid::Centroid;
use geo::algorithm::convex_hull::ConvexHull;
use geo::algorithm::simplify::Simplify;
use geo_types::{Coord, LineString, MultiPolygon, Polygon};

fn err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}
fn to_json(v: &impl serde::Serialize) -> Result<String, JsValue> {
    serde_json::to_string(v).map_err(err)
}

/// Compute area of a GeoJSON geometry (WGS84 approximation).
/// Returns JSON: `{"area_sqm":..., "area_ha":...}`
#[wasm_bindgen(js_name = computeArea)]
pub fn compute_area(geojson_geom: &str) -> Result<String, JsValue> {
    let v: serde_json::Value = serde_json::from_str(geojson_geom).map_err(err)?;
    let area = match parse_geometry(&v) {
        Some(ParsedGeom::Polygon(p)) => p.unsigned_area(),
        Some(ParsedGeom::MultiPolygon(mp)) => mp.unsigned_area(),
        None => return Err(JsValue::from_str("Unsupported geometry type")),
    };
    let sqm = area * (111_320.0_f64.powi(2));
    to_json(
        &serde_json::json!({"area_sqm":(sqm*100.0).round()/100.0,"area_ha":(sqm/1e4*100.0).round()/100.0}),
    )
}

/// Compute bounding box. Returns JSON: `{"minX":...,"minY":...,"maxX":...,"maxY":...}`
#[wasm_bindgen(js_name = computeBbox)]
pub fn compute_bbox(geojson_geom: &str) -> Result<String, JsValue> {
    let v: serde_json::Value = serde_json::from_str(geojson_geom).map_err(err)?;
    let bbox = match parse_geometry(&v) {
        Some(ParsedGeom::Polygon(p)) => p.bounding_rect(),
        Some(ParsedGeom::MultiPolygon(mp)) => mp.bounding_rect(),
        None => return Err(JsValue::from_str("Unsupported geometry type")),
    };
    match bbox {
        Some(r) => to_json(
            &serde_json::json!({"minX":r.min().x,"minY":r.min().y,"maxX":r.max().x,"maxY":r.max().y}),
        ),
        None => Err(JsValue::from_str("Could not compute bounding box")),
    }
}

/// Compute centroid. Returns JSON: `{"x":...,"y":...}`
#[wasm_bindgen(js_name = computeCentroid)]
pub fn compute_centroid(geojson_geom: &str) -> Result<String, JsValue> {
    let v: serde_json::Value = serde_json::from_str(geojson_geom).map_err(err)?;
    let c = match parse_geometry(&v) {
        Some(ParsedGeom::Polygon(p)) => p.centroid(),
        Some(ParsedGeom::MultiPolygon(mp)) => mp.centroid(),
        None => return Err(JsValue::from_str("Unsupported geometry type")),
    };
    match c {
        Some(p) => to_json(&serde_json::json!({"x":p.x(),"y":p.y()})),
        None => Err(JsValue::from_str("Could not compute centroid")),
    }
}

/// Simplify geometry (Douglas-Peucker). Returns GeoJSON geometry string.
#[wasm_bindgen(js_name = simplifyGeometry)]
pub fn simplify_geometry(geojson_geom: &str, epsilon: f64) -> Result<String, JsValue> {
    let v: serde_json::Value = serde_json::from_str(geojson_geom).map_err(err)?;
    let r = match parse_geometry(&v) {
        Some(ParsedGeom::Polygon(p)) => polygon_to_geojson(&p.simplify(&epsilon)),
        Some(ParsedGeom::MultiPolygon(mp)) => multi_polygon_to_geojson(&mp.simplify(&epsilon)),
        None => return Err(JsValue::from_str("Unsupported geometry type")),
    };
    serde_json::to_string(&r).map_err(err)
}

/// Compute convex hull. Returns GeoJSON geometry string.
#[wasm_bindgen(js_name = convexHull)]
pub fn convex_hull(geojson_geom: &str) -> Result<String, JsValue> {
    let v: serde_json::Value = serde_json::from_str(geojson_geom).map_err(err)?;
    let r = match parse_geometry(&v) {
        Some(ParsedGeom::Polygon(p)) => polygon_to_geojson(&p.convex_hull()),
        Some(ParsedGeom::MultiPolygon(mp)) => polygon_to_geojson(&mp.convex_hull()),
        None => return Err(JsValue::from_str("Unsupported geometry type")),
    };
    serde_json::to_string(&r).map_err(err)
}

// ── Internal parsing ────────────────────────────────────────────

enum ParsedGeom {
    Polygon(Polygon<f64>),
    MultiPolygon(MultiPolygon<f64>),
}

fn parse_geometry(v: &serde_json::Value) -> Option<ParsedGeom> {
    match v["type"].as_str()? {
        "Polygon" => {
            let ext = parse_ring(&v["coordinates"][0])?;
            let ints: Vec<_> = v["coordinates"].as_array()?[1..]
                .iter()
                .filter_map(parse_ring)
                .collect();
            Some(ParsedGeom::Polygon(Polygon::new(ext, ints)))
        }
        "MultiPolygon" => {
            let polys: Vec<Polygon<f64>> = v["coordinates"]
                .as_array()?
                .iter()
                .filter_map(|pc| {
                    let ext = parse_ring(&pc[0])?;
                    let ints: Vec<_> = pc.as_array()?[1..].iter().filter_map(parse_ring).collect();
                    Some(Polygon::new(ext, ints))
                })
                .collect();
            Some(ParsedGeom::MultiPolygon(MultiPolygon::new(polys)))
        }
        _ => None,
    }
}

fn parse_ring(v: &serde_json::Value) -> Option<LineString<f64>> {
    let pts: Vec<Coord<f64>> = v
        .as_array()?
        .iter()
        .filter_map(|p| {
            let a = p.as_array()?;
            if a.len() >= 2 {
                Some(Coord {
                    x: a[0].as_f64()?,
                    y: a[1].as_f64()?,
                })
            } else {
                None
            }
        })
        .collect();
    if pts.len() >= 3 {
        Some(LineString::new(pts))
    } else {
        None
    }
}

// ── GeoJSON serialization ───────────────────────────────────────

fn ring_to_coords(r: &LineString<f64>) -> Vec<Vec<f64>> {
    r.coords().map(|c| vec![c.x, c.y]).collect()
}

fn polygon_to_geojson(p: &Polygon<f64>) -> serde_json::Value {
    let mut rings = vec![ring_to_coords(p.exterior())];
    rings.extend(p.interiors().iter().map(ring_to_coords));
    serde_json::json!({"type":"Polygon","coordinates":rings})
}

fn multi_polygon_to_geojson(mp: &MultiPolygon<f64>) -> serde_json::Value {
    let coords: Vec<serde_json::Value> = mp
        .iter()
        .map(|p| {
            let mut rings = vec![ring_to_coords(p.exterior())];
            rings.extend(p.interiors().iter().map(ring_to_coords));
            serde_json::json!(rings)
        })
        .collect();
    serde_json::json!({"type":"MultiPolygon","coordinates":coords})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> String {
        r#"{"type":"Polygon","coordinates":[[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}"#.into()
    }

    #[test]
    fn test_area() {
        let r = compute_area(&sample()).unwrap();
        assert!(r.contains("area_ha"));
    }

    #[test]
    fn test_bbox() {
        let r = compute_bbox(&sample()).unwrap();
        assert!(r.contains("minX"));
    }

    #[test]
    fn test_centroid() {
        let r = compute_centroid(&sample()).unwrap();
        assert!(r.contains("\"x\""));
    }

    #[test]
    fn test_simplify() {
        let r = simplify_geometry(&sample(), 0.01).unwrap();
        assert!(r.contains("Polygon"));
    }
}
