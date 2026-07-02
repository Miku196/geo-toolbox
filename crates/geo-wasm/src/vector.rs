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

// ── Additional vector operations ─────────────────────────────────

/// Clip a MultiPolygon to a clipping polygon's boundary.
/// Returns the portion of `target` that falls inside `clip_poly`.
///
/// ## Returns
/// GeoJSON MultiPolygon string
#[wasm_bindgen(js_name = clipGeometry)]
pub fn clip_geometry(target_geojson: &str, clip_geojson: &str) -> Result<String, JsValue> {
    clip_geometry_inner(target_geojson, clip_geojson).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn clip_geometry_inner(target_geojson: &str, clip_geojson: &str) -> GeoResult<String> {
    let vt: serde_json::Value =
        serde_json::from_str(target_geojson).map_err(geo_core::errors::GeoError::Serde)?;
    let vc: serde_json::Value =
        serde_json::from_str(clip_geojson).map_err(geo_core::errors::GeoError::Serde)?;

    let target_mp = match parse_geometry(&vt) {
        Some(ParsedGeom::MultiPolygon(mp)) => mp,
        Some(ParsedGeom::Polygon(p)) => MultiPolygon::new(vec![p]),
        None => {
            return Err(geo_core::errors::GeoError::Validation(
                "Invalid target geometry".into(),
            ))
        }
    };
    let clip_poly = match parse_geometry(&vc) {
        Some(ParsedGeom::Polygon(p)) => p,
        _ => {
            return Err(geo_core::errors::GeoError::Validation(
                "Clip geometry must be a Polygon".into(),
            ))
        }
    };

    let result = geo_vector::clip(&target_mp, &clip_poly);
    geojson_to_string(&multi_polygon_to_geojson(&result))
}

/// Compute the difference of two polygons (a minus b).
///
/// ## Returns
/// GeoJSON MultiPolygon string, or `null` if the result is empty.
#[wasm_bindgen(js_name = difference)]
pub fn difference(geojson_a: &str, geojson_b: &str) -> Result<String, JsValue> {
    difference_inner(geojson_a, geojson_b).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn difference_inner(geojson_a: &str, geojson_b: &str) -> GeoResult<String> {
    let va: serde_json::Value =
        serde_json::from_str(geojson_a).map_err(geo_core::errors::GeoError::Serde)?;
    let vb: serde_json::Value =
        serde_json::from_str(geojson_b).map_err(geo_core::errors::GeoError::Serde)?;

    let a = match parse_geometry(&va) {
        Some(ParsedGeom::Polygon(p)) => p,
        _ => {
            return Err(geo_core::errors::GeoError::Validation(
                "First arg must be a Polygon".into(),
            ))
        }
    };
    let b = match parse_geometry(&vb) {
        Some(ParsedGeom::Polygon(p)) => p,
        _ => {
            return Err(geo_core::errors::GeoError::Validation(
                "Second arg must be a Polygon".into(),
            ))
        }
    };

    match geo_vector::difference(&a, &b) {
        Some(mp) => geojson_to_string(&multi_polygon_to_geojson(&mp)),
        None => Ok("null".into()),
    }
}

/// Compute the symmetric difference of two polygons (XOR).
///
/// ## Returns
/// GeoJSON MultiPolygon string, or `null` if the result is empty.
#[wasm_bindgen(js_name = symDifference)]
pub fn sym_difference(geojson_a: &str, geojson_b: &str) -> Result<String, JsValue> {
    sym_difference_inner(geojson_a, geojson_b).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn sym_difference_inner(geojson_a: &str, geojson_b: &str) -> GeoResult<String> {
    let va: serde_json::Value =
        serde_json::from_str(geojson_a).map_err(geo_core::errors::GeoError::Serde)?;
    let vb: serde_json::Value =
        serde_json::from_str(geojson_b).map_err(geo_core::errors::GeoError::Serde)?;

    let a = match parse_geometry(&va) {
        Some(ParsedGeom::Polygon(p)) => p,
        _ => {
            return Err(geo_core::errors::GeoError::Validation(
                "First arg must be a Polygon".into(),
            ))
        }
    };
    let b = match parse_geometry(&vb) {
        Some(ParsedGeom::Polygon(p)) => p,
        _ => {
            return Err(geo_core::errors::GeoError::Validation(
                "Second arg must be a Polygon".into(),
            ))
        }
    };

    match geo_vector::sym_difference(&a, &b) {
        Some(mp) => geojson_to_string(&multi_polygon_to_geojson(&mp)),
        None => Ok("null".into()),
    }
}

/// Simplify a polygon using the Visvalingam-Whyatt algorithm.
/// Returns GeoJSON Polygon string.
#[wasm_bindgen(js_name = simplifyVisvalingam)]
pub fn simplify_visvalingam(geojson_geom: &str, epsilon: f64) -> Result<String, JsValue> {
    simplify_visvalingam_inner(geojson_geom, epsilon).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn simplify_visvalingam_inner(geojson_geom: &str, epsilon: f64) -> GeoResult<String> {
    let v: serde_json::Value =
        serde_json::from_str(geojson_geom).map_err(geo_core::errors::GeoError::Serde)?;
    let poly = match parse_geometry(&v) {
        Some(ParsedGeom::Polygon(p)) => p,
        _ => {
            return Err(geo_core::errors::GeoError::Validation(
                "Expected a Polygon".into(),
            ))
        }
    };
    let result = geo_vector::simplify_visvalingam_preserve(&poly.exterior().clone(), epsilon);
    let rings = vec![ring_to_coords(&result)];
    let geojson = serde_json::json!({"type": "Polygon", "coordinates": rings});
    geojson_to_string(&geojson)
}

/// Kernel density estimation from a set of points.
/// Returns a flat array of density values (row-major grid from bbox).
///
/// Parameters:
/// - `points`: flat array of [x0, y0, x1, y1, ...]
/// - `grid_cols`, `grid_rows`: output grid dimensions
/// - `min_x`, `min_y`, `max_x`, `max_y`: bounding box for the grid
/// - `bandwidth`: kernel bandwidth (in coordinate units)
///
/// Returns JSON array of density values.
#[wasm_bindgen(js_name = kernelDensity)]
pub fn kernel_density(
    points: Vec<f64>,
    grid_cols: usize,
    grid_rows: usize,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    bandwidth: f64,
) -> Result<String, JsValue> {
    let pts: Vec<(f64, f64)> = points
        .chunks(2)
        .filter_map(|c| {
            if c.len() == 2 {
                Some((c[0], c[1]))
            } else {
                None
            }
        })
        .collect();
    let result = geo_vector::kernel_density(
        &pts,
        grid_cols,
        grid_rows,
        (min_x, min_y, max_x, max_y),
        bandwidth,
    );
    serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Line density estimation from a set of line segments.
/// Returns a flat array of density values (row-major grid from bbox).
///
/// Parameters:
/// - `lines`: flat array of [x0, y0, x1, y1, ...] for each segment
/// - `grid_cols`, `grid_rows`: output grid dimensions
/// - `min_x`, `min_y`, `max_x`, `max_y`: bounding box for the grid
///
/// Returns JSON array of density values.
#[wasm_bindgen(js_name = lineDensity)]
pub fn line_density(
    lines: Vec<f64>,
    grid_cols: usize,
    grid_rows: usize,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
) -> Result<String, JsValue> {
    let segs: Vec<(f64, f64, f64, f64)> = lines
        .chunks(4)
        .filter_map(|c| {
            if c.len() == 4 {
                Some((c[0], c[1], c[2], c[3]))
            } else {
                None
            }
        })
        .collect();
    let result =
        geo_vector::line_density(&segs, grid_cols, grid_rows, (min_x, min_y, max_x, max_y));
    serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Test whether a point lies inside a polygon.
/// Returns `true` or `false`.
#[wasm_bindgen(js_name = pointInPolygon)]
pub fn point_in_polygon(x: f64, y: f64, geojson_poly: &str) -> Result<JsValue, JsValue> {
    let v: serde_json::Value =
        serde_json::from_str(geojson_poly).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let poly = match parse_geometry(&v) {
        Some(ParsedGeom::Polygon(p)) => p,
        _ => return Err(JsValue::from_str("Expected a Polygon GeoJSON")),
    };
    Ok(JsValue::from_bool(geo_vector::point_in_polygon(
        x, y, &poly,
    )))
}

/// Spatial join: classify points by which polygon zone they fall into.
///
/// Parameters:
/// - `points`: flat array [x0, y0, x1, y1, ...]
/// - `zones_geojson`: JSON object `{"zone_name": geojson_poly, ...}`
///
/// Returns JSON array of zone names (or null) for each point.
#[wasm_bindgen(js_name = spatialJoinPoints)]
pub fn spatial_join_points(points: Vec<f64>, zones_geojson: &str) -> Result<String, JsValue> {
    let pts: Vec<(f64, f64)> = points
        .chunks(2)
        .filter_map(|c| {
            if c.len() == 2 {
                Some((c[0], c[1]))
            } else {
                None
            }
        })
        .collect();

    let zones_val: serde_json::Value =
        serde_json::from_str(zones_geojson).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let zones_map = zones_val
        .as_object()
        .ok_or_else(|| JsValue::from_str("zones_geojson must be a JSON object"))?;

    let mut zones: Vec<(String, Polygon<f64>)> = Vec::new();
    for (name, geom_val) in zones_map {
        if let Some(ParsedGeom::Polygon(p)) = parse_geometry(geom_val) {
            zones.push((name.clone(), p));
        }
    }
    let zone_refs: Vec<(&str, &Polygon<f64>)> =
        zones.iter().map(|(n, p)| (n.as_str(), p)).collect();
    let result = geo_vector::spatial_join_points(&pts, &zone_refs);
    let json_result: Vec<Option<&str>> = result.into_iter().collect();
    serde_json::to_string(&json_result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Validate a polygon geometry (self-intersections, ring orientation, etc.).
/// Returns JSON array of validation messages. Empty array = valid.
#[wasm_bindgen(js_name = validateGeometry)]
pub fn validate_geometry(geojson_geom: &str) -> Result<String, JsValue> {
    let v: serde_json::Value =
        serde_json::from_str(geojson_geom).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let poly = match parse_geometry(&v) {
        Some(ParsedGeom::Polygon(p)) => p,
        _ => return Err(JsValue::from_str("Expected a Polygon GeoJSON")),
    };
    let issues = geo_vector::validate_geometry(&poly);
    serde_json::to_string(&issues).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Detect gaps/holes between polygons in a MultiPolygon.
/// Returns GeoJSON MultiPolygon of gap areas.
#[wasm_bindgen(js_name = detectGaps)]
pub fn detect_gaps(geojson_mp: &str) -> Result<String, JsValue> {
    let v: serde_json::Value =
        serde_json::from_str(geojson_mp).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let mp = match parse_geometry(&v) {
        Some(ParsedGeom::MultiPolygon(mp)) => mp,
        Some(ParsedGeom::Polygon(p)) => MultiPolygon::new(vec![p]),
        None => return Err(JsValue::from_str("Expected a MultiPolygon GeoJSON")),
    };
    let gaps = geo_vector::detect_gaps(&mp);
    // Build GeoJSON MultiPolygon from gap polygons
    let coords: Vec<serde_json::Value> = gaps
        .iter()
        .map(|p| {
            let mut rings = vec![ring_to_coords(p.exterior())];
            rings.extend(p.interiors().iter().map(ring_to_coords));
            serde_json::json!(rings)
        })
        .collect();
    let geojson = serde_json::json!({"type": "MultiPolygon", "coordinates": coords});
    serde_json::to_string(&geojson).map_err(|e| JsValue::from_str(&e.to_string()))
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
