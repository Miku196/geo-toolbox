//! Spatial geometry operations.

use geo::algorithm::area::Area;
use geo::algorithm::bounding_rect::BoundingRect;
use geo::algorithm::centroid::Centroid;
use geo::algorithm::convex_hull::ConvexHull;
use geo::algorithm::simplify::Simplify;
use geo_types::{Coord, LineString, MultiPolygon, Polygon};
use pyo3::prelude::*;

/// Compute area of a Polygon or MultiPolygon GeoJSON geometry in sqm.
pub fn compute_area_sqm_impl(geojson_geom: &str) -> PyResult<f64> {
    let v: serde_json::Value = serde_json::from_str(geojson_geom)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    let area = match parse_geometry(&v) {
        Some(ParsedGeom::Polygon(p)) => p.unsigned_area(),
        Some(ParsedGeom::MultiPolygon(mp)) => mp.unsigned_area(),
        None => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Unsupported geometry type",
            ))
        }
    };
    let sqm = area * (111_320.0_f64).powi(2);
    Ok((sqm * 100.0).round() / 100.0)
}

/// Compute bounding box. Returns (min_x, min_y, max_x, max_y).
pub fn compute_bbox_impl(geojson_geom: &str) -> PyResult<(f64, f64, f64, f64)> {
    let v: serde_json::Value = serde_json::from_str(geojson_geom)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    let bbox = match parse_geometry(&v) {
        Some(ParsedGeom::Polygon(p)) => p.bounding_rect(),
        Some(ParsedGeom::MultiPolygon(mp)) => mp.bounding_rect(),
        None => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Unsupported geometry type",
            ))
        }
    };
    match bbox {
        Some(r) => Ok((r.min().x, r.min().y, r.max().x, r.max().y)),
        None => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Could not compute bbox",
        )),
    }
}

/// Compute centroid. Returns (x, y).
pub fn compute_centroid_impl(geojson_geom: &str) -> PyResult<(f64, f64)> {
    let v: serde_json::Value = serde_json::from_str(geojson_geom)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    let c = match parse_geometry(&v) {
        Some(ParsedGeom::Polygon(p)) => p.centroid(),
        Some(ParsedGeom::MultiPolygon(mp)) => mp.centroid(),
        None => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Unsupported geometry type",
            ))
        }
    };
    match c {
        Some(p) => Ok((p.x(), p.y())),
        None => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Could not compute centroid",
        )),
    }
}

/// Simplify geometry (Douglas-Peucker). Returns GeoJSON geometry string.
pub fn simplify_geometry_impl(geojson_geom: &str, epsilon: f64) -> PyResult<String> {
    let v: serde_json::Value = serde_json::from_str(geojson_geom)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    let r = match parse_geometry(&v) {
        Some(ParsedGeom::Polygon(p)) => polygon_to_geojson_value(&p.simplify(&epsilon)),
        Some(ParsedGeom::MultiPolygon(mp)) => {
            multi_polygon_to_geojson_value(&mp.simplify(&epsilon))
        }
        None => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Unsupported geometry type",
            ))
        }
    };
    serde_json::to_string(&r)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
}

/// Compute convex hull. Returns GeoJSON geometry string.
pub fn convex_hull_impl(geojson_geom: &str) -> PyResult<String> {
    let v: serde_json::Value = serde_json::from_str(geojson_geom)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    let r = match parse_geometry(&v) {
        Some(ParsedGeom::Polygon(p)) => polygon_to_geojson_value(&p.convex_hull()),
        Some(ParsedGeom::MultiPolygon(mp)) => polygon_to_geojson_value(&mp.convex_hull()),
        None => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Unsupported geometry type",
            ))
        }
    };
    serde_json::to_string(&r)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
}

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

fn ring_to_coords(r: &LineString<f64>) -> Vec<Vec<f64>> {
    r.coords().map(|c| vec![c.x, c.y]).collect()
}

fn polygon_to_geojson_value(p: &Polygon<f64>) -> serde_json::Value {
    let mut rings = vec![ring_to_coords(p.exterior())];
    rings.extend(p.interiors().iter().map(ring_to_coords));
    serde_json::json!({"type": "Polygon", "coordinates": rings})
}

fn multi_polygon_to_geojson_value(mp: &MultiPolygon<f64>) -> serde_json::Value {
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
