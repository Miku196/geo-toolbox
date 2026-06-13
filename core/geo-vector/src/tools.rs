//! Tool registration — Vector ops.
use geo_core::plugin::PluginCategory;
use geo_registry::registry::{ToolDef, ToolResult};
use geo_registry::PluginRegistry;
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
    let polys: Vec<serde_json::Value> = mp.iter().map(|p| polygon_to_json(p)).collect();
    if polys.len() == 1 {
        polys.into_iter().next().unwrap()
    } else {
        serde_json::json!({"type":"MultiPolygon","coordinates":polys})
    }
}

pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta {
        name: "vector".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        description: "Pure-Rust vector ops: buffer, intersect, area, centroid".into(),
        category: PluginCategory::Process,
        healthy: true,
        extra: serde_json::json!({}),
    });
    registry.register_tool_sync("vector", ToolDef {
        name: "vector_buffer".into(), description: "Create a bbox buffer around a Polygon".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"geojson":{"type":"string"},"distance_m":{"type":"number"}},"required":["geojson","distance_m"]}),
    }, |args| -> ToolResult {
        let p = parse_polygon(args["geojson"].as_str().unwrap_or("{}"))?;
        Ok(multipolygon_to_json(&crate::buffer(&p, args["distance_m"].as_f64().unwrap_or(0.0), crate::BufferMode::ConvexHull { quadrant_segments: 8 })))
    });
    registry.register_tool_sync("vector", ToolDef {
        name: "vector_intersect".into(), description: "Compute intersection of two Polygons".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"geojson_a":{"type":"string"},"geojson_b":{"type":"string"}},"required":["geojson_a","geojson_b"]}),
    }, |args| -> ToolResult {
        let a = parse_polygon(args["geojson_a"].as_str().unwrap_or("{}"))?;
        let b = parse_polygon(args["geojson_b"].as_str().unwrap_or("{}"))?;
        let r = crate::intersect(&a,&b).ok_or_else(|| geo_core::GeoError::Validation("no intersection".into()))?;
        Ok(multipolygon_to_json(&r))
    });
    registry.register_tool_sync("vector", ToolDef {
        name: "vector_area".into(), description: "Compute area of a Polygon in m² and ha".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"geojson":{"type":"string"}},"required":["geojson"]}),
    }, |args| -> ToolResult {
        let p = parse_polygon(args["geojson"].as_str().unwrap_or("{}"))?;
        let a = crate::stats::feature_area(&p);
        Ok(serde_json::json!({"area_m2":a,"area_ha":a/10000.0}))
    });
    registry.register_tool_sync("vector", ToolDef {
        name: "vector_centroid".into(), description: "Compute centroid of a Polygon".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"geojson":{"type":"string"}},"required":["geojson"]}),
    }, |args| -> ToolResult {
        let p = parse_polygon(args["geojson"].as_str().unwrap_or("{}"))?;
        let c = crate::centroid(&p).ok_or_else(|| geo_core::GeoError::Validation("centroid failed".into()))?;
        Ok(serde_json::json!({"lon":c.x(),"lat":c.y()}))
    });
}
