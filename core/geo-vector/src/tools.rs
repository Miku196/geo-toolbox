//! Tool registration — Vector ops.
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};
use geo_types::{LineString, Polygon};

fn parse_polygon(geojson: &str) -> Result<Polygon<f64>, geo_core::GeoError> {
    let v: serde_json::Value = serde_json::from_str(geojson).map_err(geo_core::GeoError::Serde)?;
    let ring = v["coordinates"]
        .as_array()
        .and_then(|r| r[0].as_array())
        .ok_or_else(|| geo_core::GeoError::invalid_input("geojson", "expected Polygon"))?;
    let pts: Vec<_> = ring
        .iter()
        .filter_map(|c| {
            Some(geo_types::Coord {
                x: c[0].as_f64()?,
                y: c[1].as_f64()?,
            })
        })
        .collect();
    Ok(Polygon::new(LineString::from(pts), vec![]))
}
fn polygon_to_json(poly: &Polygon<f64>) -> serde_json::Value {
    let c: Vec<Vec<f64>> = poly
        .exterior()
        .points()
        .map(|p| vec![p.x(), p.y()])
        .collect();
    serde_json::json!({"type":"Polygon","coordinates":[c]})
}
fn multipolygon_to_json(mp: &geo_types::MultiPolygon<f64>) -> serde_json::Value {
    let polys: Vec<serde_json::Value> = mp.iter().map(polygon_to_json).collect();
    if polys.len() == 1 {
        polys.into_iter().next().unwrap()
    } else {
        serde_json::json!({"type":"MultiPolygon","coordinates":polys})
    }
}

pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "vector", "Pure-Rust vector ops: buffer, intersect, area, centroid", PluginCategory::Process, [
        sync "vector_buffer" => "Create a bbox buffer around a Polygon" ; serde_json::json!({"type":"object","properties":{"geojson":{"type":"string"},"distance_m":{"type":"number"}},"required":["geojson","distance_m"]}) => |args| -> ToolResult {
        let p = parse_polygon(args["geojson"].as_str().unwrap_or("{}"))?;
        Ok(multipolygon_to_json(&crate::ops::buffer(&p, args["distance_m"].as_f64().unwrap_or(0.0), crate::ops::BufferMode::ConvexHull { quadrant_segments: 8 })))
    },
        sync "vector_intersect" => "Compute intersection of two Polygons" ; serde_json::json!({"type":"object","properties":{"geojson_a":{"type":"string"},"geojson_b":{"type":"string"}},"required":["geojson_a","geojson_b"]}) => |args| -> ToolResult {
        let a = parse_polygon(args["geojson_a"].as_str().unwrap_or("{}"))?;
        let b = parse_polygon(args["geojson_b"].as_str().unwrap_or("{}"))?;
        let r = crate::ops::intersect(&a,&b).ok_or_else(|| geo_core::GeoError::Validation("no intersection".into()))?;
        Ok(multipolygon_to_json(&r))
    },
        sync "vector_area" => "Compute area of a Polygon in m² and ha" ; serde_json::json!({"type":"object","properties":{"geojson":{"type":"string"}},"required":["geojson"]}) => |args| -> ToolResult {
        let p = parse_polygon(args["geojson"].as_str().unwrap_or("{}"))?;
        let a = crate::stats::feature_area(&p);
        Ok(serde_json::json!({"area_m2":a,"area_ha":a/10000.0}))
    },
        sync "vector_centroid" => "Compute centroid of a Polygon" ; serde_json::json!({"type":"object","properties":{"geojson":{"type":"string"}},"required":["geojson"]}) => |args| -> ToolResult {
        let p = parse_polygon(args["geojson"].as_str().unwrap_or("{}"))?;
        let c = crate::stats::centroid(&p).ok_or_else(|| geo_core::GeoError::Validation("centroid failed".into()))?;
        Ok(serde_json::json!({"lon":c.x(),"lat":c.y()}))
    },
        sync "vector_difference" => "A - B: erase/remove area of B from A" ; serde_json::json!({"type":"object","properties":{"geojson_a":{"type":"string"},"geojson_b":{"type":"string"}},"required":["geojson_a","geojson_b"]}) => |args| -> ToolResult {
        let a = parse_polygon(args["geojson_a"].as_str().unwrap_or("{}"))?;
        let b = parse_polygon(args["geojson_b"].as_str().unwrap_or("{}"))?;
        let r = crate::ops::difference(&a,&b).ok_or_else(|| geo_core::GeoError::Validation("difference produced empty result".into()))?;
        Ok(multipolygon_to_json(&r))
    },
        sync "vector_sym_difference" => "A XOR B: non-overlapping parts of two Polygons" ; serde_json::json!({"type":"object","properties":{"geojson_a":{"type":"string"},"geojson_b":{"type":"string"}},"required":["geojson_a","geojson_b"]}) => |args| -> ToolResult {
        let a = parse_polygon(args["geojson_a"].as_str().unwrap_or("{}"))?;
        let b = parse_polygon(args["geojson_b"].as_str().unwrap_or("{}"))?;
        let r = crate::ops::sym_difference(&a,&b).ok_or_else(|| geo_core::GeoError::Validation("sym_difference produced empty result".into()))?;
        Ok(multipolygon_to_json(&r))
    },
        sync "vector_clip" => "Clip a MultiPolygon by a Polygon boundary" ; serde_json::json!({"type":"object","properties":{"geojson_target":{"type":"string"},"geojson_clip":{"type":"string"}},"required":["geojson_target","geojson_clip"]}) => |args| -> ToolResult {
        let target_v: serde_json::Value = serde_json::from_str(args["geojson_target"].as_str().unwrap_or("{}"))?;
        let _coords = target_v["coordinates"].as_array().and_then(|c| c[0].as_array()).cloned().unwrap_or_default();
        let polys: Vec<geo_types::Polygon<f64>> = if target_v["type"] == "MultiPolygon" {
            target_v["coordinates"].as_array().map(|arr| {
                arr.iter().filter_map(|p| {
                    let ring = p[0].as_array()?;
                    let pts: Vec<_> = ring.iter().filter_map(|c| Some(geo_types::Coord{x: c[0].as_f64()?, y: c[1].as_f64()?})).collect();
                    Some(geo_types::Polygon::new(geo_types::LineString::from(pts), vec![]))
                }).collect()
            }).unwrap_or_default()
        } else {
            let ring = target_v["coordinates"][0].as_array().cloned().unwrap_or_default();
            let pts: Vec<_> = ring.iter().filter_map(|c| Some(geo_types::Coord{x: c[0].as_f64()?, y: c[1].as_f64()?})).collect();
            vec![geo_types::Polygon::new(geo_types::LineString::from(pts), vec![])]
        };
        let mp: geo_types::MultiPolygon<f64> = polys.into();
        let clip_p = parse_polygon(args["geojson_clip"].as_str().unwrap_or("{}"))?;
        let result = crate::ops::clip(&mp, &clip_p);
        Ok(multipolygon_to_json(&result))
    }]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::Area;
    use geo_types::MultiPolygon;

    fn square_geojson(x: f64, y: f64, s: f64) -> String {
        format!(
            r#"{{"type":"Polygon","coordinates":[[[{x},{y}],[{x2},{y}],[{x2},{y2}],[{x},{y2}],[{x},{y}]]]}}"#,
            x = x,
            y = y,
            x2 = x + s,
            y2 = y + s
        )
    }

    #[test]
    fn test_parse_polygon_valid() {
        let geojson = square_geojson(0.0, 0.0, 10.0);
        let poly = parse_polygon(&geojson).unwrap();
        assert!((poly.unsigned_area() - 100.0).abs() < 1e-9);
    }

    #[test]
    fn test_parse_polygon_invalid_json() {
        let result = parse_polygon("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_polygon_point_not_polygon() {
        let point_json = r#"{"type":"Point","coordinates":[0.0,0.0]}"#;
        let result = parse_polygon(point_json);
        assert!(result.is_err(), "Point should fail polygon parse");
    }

    #[test]
    fn test_parse_polygon_empty() {
        let result = parse_polygon("{}");
        assert!(result.is_err(), "Empty JSON should fail");
    }

    #[test]
    fn test_multipolygon_to_json_single() {
        let geojson = square_geojson(0.0, 0.0, 5.0);
        let poly = parse_polygon(&geojson).unwrap();
        let mp = MultiPolygon::new(vec![poly]);
        let json = multipolygon_to_json(&mp);
        assert_eq!(json["type"], "Polygon");
        assert!(json["coordinates"].is_array());
    }

    #[test]
    fn test_multipolygon_to_json_multiple() {
        let p1 = parse_polygon(&square_geojson(0.0, 0.0, 5.0)).unwrap();
        let p2 = parse_polygon(&square_geojson(10.0, 10.0, 5.0)).unwrap();
        let mp = MultiPolygon::new(vec![p1, p2]);
        let json = multipolygon_to_json(&mp);
        assert_eq!(json["type"], "MultiPolygon");
        assert_eq!(json["coordinates"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_parse_polygon_roundtrip() {
        let geojson = square_geojson(1.0, 2.0, 3.0);
        let poly = parse_polygon(&geojson).unwrap();
        let json = multipolygon_to_json(&MultiPolygon::new(vec![poly.clone()]));
        let poly2 = parse_polygon(&json.to_string()).unwrap();
        assert!((poly.unsigned_area() - poly2.unsigned_area()).abs() < 1e-9);
    }
}
