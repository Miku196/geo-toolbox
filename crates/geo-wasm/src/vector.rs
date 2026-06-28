//! Browser-side vector geometry operations — buffer, intersect, union.
//!
//! Thin WASM wrappers around [`geo_vector::ops`].
//! All geometry passed as GeoJSON strings, returned as GeoJSON strings.

use geo_core::errors::GeoResult;
use geo_types::{Coord, LineString, MultiPolygon, Polygon};
use wasm_bindgen::prelude::*;

// ── GeoJSON parsing (reused from spatial.rs pattern) ─────────────

#[allow(dead_code)]
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

// ── GeoJSON serialization ────────────────────────────────────────

fn ring_to_coords(r: &LineString<f64>) -> Vec<Vec<f64>> {
    r.coords().map(|c| vec![c.x, c.y]).collect()
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
    serde_json::json!({"type": "MultiPolygon", "coordinates": coords})
}

fn geojson_to_string(val: &serde_json::Value) -> GeoResult<String> {
    serde_json::to_string(val).map_err(geo_core::errors::GeoError::Serde)
}

// ── Public API ──────────────────────────────────────────────────

/// Buffer a polygon by a given distance (in degrees).
///
/// ## Parameters
/// - `geojson_geom`: GeoJSON Polygon or MultiPolygon string
/// - `distance`: buffer distance (in degrees)
/// - `mode`: "bbox" | "convex_hull" | "precise" (default: "precise")
/// - `quadrant_segments`: optional, for convex_hull/precise modes (default: 8)
///
/// ## Returns
/// GeoJSON MultiPolygon string
#[wasm_bindgen(js_name = computeBuffer)]
pub fn compute_buffer(
    geojson_geom: &str,
    distance: f64,
    mode: &str,
    quadrant_segments: Option<u8>,
) -> Result<String, JsValue> {
    compute_buffer_inner(geojson_geom, distance, mode, quadrant_segments)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

fn compute_buffer_inner(
    geojson_geom: &str,
    distance: f64,
    mode: &str,
    quadrant_segments: Option<u8>,
) -> GeoResult<String> {
    let v: serde_json::Value =
        serde_json::from_str(geojson_geom).map_err(geo_core::errors::GeoError::Serde)?;
    let poly = match parse_geometry(&v) {
        Some(ParsedGeom::Polygon(p)) => p,
        Some(ParsedGeom::MultiPolygon(_)) => {
            return Err(geo_core::errors::GeoError::Validation(
                "Buffer expects Polygon input, use union_all for MultiPolygon".into(),
            ));
        }
        None => {
            return Err(geo_core::errors::GeoError::Validation(
                "Unsupported geometry type for buffer".into(),
            ));
        }
    };

    let buffer_mode = match mode {
        "bbox" => geo_vector::BufferMode::Bbox,
        "convex_hull" => geo_vector::BufferMode::ConvexHull {
            quadrant_segments: quadrant_segments.unwrap_or(8),
        },
        _ => geo_vector::BufferMode::Precise {
            quadrant_segments: quadrant_segments.unwrap_or(8),
        },
    };

    let result = geo_vector::buffer(&poly, distance, buffer_mode);
    geojson_to_string(&multi_polygon_to_geojson(&result))
}

/// Compute the intersection of two polygons.
///
/// ## Returns
/// GeoJSON MultiPolygon string, or `null` if no intersection.
#[wasm_bindgen(js_name = computeIntersect)]
pub fn compute_intersect(geojson_a: &str, geojson_b: &str) -> Result<String, JsValue> {
    compute_intersect_inner(geojson_a, geojson_b).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn compute_intersect_inner(geojson_a: &str, geojson_b: &str) -> GeoResult<String> {
    let va: serde_json::Value =
        serde_json::from_str(geojson_a).map_err(geo_core::errors::GeoError::Serde)?;
    let vb: serde_json::Value =
        serde_json::from_str(geojson_b).map_err(geo_core::errors::GeoError::Serde)?;

    let a = match parse_geometry(&va) {
        Some(ParsedGeom::Polygon(p)) => p,
        _ => {
            return Err(geo_core::errors::GeoError::Validation(
                "First arg must be a Polygon".into(),
            ));
        }
    };
    let b = match parse_geometry(&vb) {
        Some(ParsedGeom::Polygon(p)) => p,
        _ => {
            return Err(geo_core::errors::GeoError::Validation(
                "Second arg must be a Polygon".into(),
            ));
        }
    };

    match geo_vector::intersect(&a, &b) {
        Some(mp) => geojson_to_string(&multi_polygon_to_geojson(&mp)),
        None => Ok("null".into()),
    }
}

/// Compute the union of an array of polygons.
///
/// ## Parameters
/// - `geojson_polys`: JSON array of GeoJSON Polygon strings, or a single MultiPolygon
///
/// ## Returns
/// GeoJSON MultiPolygon string
#[wasm_bindgen(js_name = unionAll)]
pub fn union_all(geojson_polys: &str) -> Result<String, JsValue> {
    union_all_inner(geojson_polys).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn union_all_inner(geojson_polys: &str) -> GeoResult<String> {
    let val: serde_json::Value =
        serde_json::from_str(geojson_polys).map_err(geo_core::errors::GeoError::Serde)?;

    let polygons: Vec<Polygon<f64>> = if val.is_array() {
        val.as_array()
            .unwrap()
            .iter()
            .filter_map(|v| match parse_geometry(v) {
                Some(ParsedGeom::Polygon(p)) => Some(p),
                _ => None,
            })
            .collect()
    } else {
        return Err(geo_core::errors::GeoError::Validation(
            "Expected an array of Polygon GeoJSON objects".into(),
        ));
    };

    if polygons.is_empty() {
        return Err(geo_core::errors::GeoError::Validation(
            "No valid polygons found".into(),
        ));
    }

    match geo_vector::union_all(&polygons) {
        Some(mp) => geojson_to_string(&multi_polygon_to_geojson(&mp)),
        None => Err(geo_core::errors::GeoError::Validation(
            "Union produced no result".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_poly() -> String {
        r#"{"type":"Polygon","coordinates":[[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}"#.into()
    }

    #[test]
    fn test_buffer() {
        let result = compute_buffer_inner(&sample_poly(), 0.01, "precise", None).unwrap();
        assert!(result.contains("MultiPolygon"));

        let bbox_result = compute_buffer_inner(&sample_poly(), 0.01, "bbox", None).unwrap();
        assert!(bbox_result.contains("MultiPolygon"));
    }

    #[test]
    fn test_intersect() {
        let a = sample_poly();
        let b = r#"{"type":"Polygon","coordinates":[[[104.05,30.45],[104.15,30.45],[104.15,30.55],[104.05,30.55],[104.05,30.45]]]}"#;
        let result = compute_intersect_inner(&a, b).unwrap();
        assert!(result.contains("MultiPolygon"));
    }

    #[test]
    fn test_intersect_no_overlap() {
        let a = sample_poly();
        let b = r#"{"type":"Polygon","coordinates":[[[200.0,0.0],[201.0,0.0],[201.0,1.0],[200.0,1.0],[200.0,0.0]]]}"#;
        let result = compute_intersect_inner(&a, b).unwrap();
        assert_eq!(result, "null");
    }

    #[test]
    fn test_union() {
        let polys = format!(
            "[{}, {}]",
            sample_poly(),
            r#"{"type":"Polygon","coordinates":[[[104.05,30.45],[104.15,30.45],[104.15,30.55],[104.05,30.55],[104.05,30.45]]]}"#,
        );
        let result = union_all_inner(&polys).unwrap();
        assert!(result.contains("MultiPolygon"));
    }
}
