//! Unix-pipeline geospatial processing.
//!
//! Pipe GeoJSON through processing steps:
//! ```bash
//! geo read input.geojson | geo buffer 100 | geo simplify 0.01 | geo write output.geojson
//! cat data.csv | geo read --format csv | geo reproject 4326 3857 | geo write out.json
//! ```
//!
//! Wire format: GeoJSON FeatureCollection on stdin/stdout.
//! stdin is read entirely before processing (not streaming).

use geo_core::errors::GeoResult;
use geo_wiring::PluginRegistry;
use geojson::{FeatureCollection, GeoJson};
use std::io::{self, Read, Write};

use crate::PipelineAction;

/// Execute a pipeline action: read stdin, process, write stdout.
pub fn handle(
    registry: &PluginRegistry,
    action: PipelineAction,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        PipelineAction::Read { input, format } => handle_read(input, format),
        PipelineAction::Buffer { distance, units } => handle_buffer(distance, units),
        PipelineAction::Simplify { epsilon } => handle_simplify(epsilon),
        PipelineAction::Reproject { from_epsg, to_epsg } => handle_reproject(from_epsg, to_epsg),
        PipelineAction::Write { output, format } => handle_write(output, format),
        PipelineAction::Area => handle_area(),
        PipelineAction::Filter { key, value } => handle_filter(&key, &value),
    }
}

// ── stdin helpers ─────────────────────────────────────────────

fn read_stdin() -> io::Result<String> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

fn read_input(input: Option<String>, format: &str) -> io::Result<String> {
    if let Some(path) = input {
        if path == "-" {
            read_stdin()
        } else {
            std::fs::read_to_string(&path).map_err(|e| {
                io::Error::new(io::ErrorKind::NotFound, format!("Cannot read {path}: {e}"))
            })
        }
    } else {
        read_stdin()
    }
}

fn write_stdout(geojson_str: &str) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(geojson_str.as_bytes())?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}

fn write_file(path: &str, content: &str) -> io::Result<()> {
    std::fs::write(path, content)
}

fn parse_fc(input: &str, format: &str) -> Result<FeatureCollection, String> {
    match format.to_lowercase().as_str() {
        "geojson" | "json" => {
            let gj: GeoJson = input.parse().map_err(|e| format!("Invalid GeoJSON: {e}"))?;
            match gj {
                GeoJson::FeatureCollection(fc) => Ok(fc),
                GeoJson::Feature(f) => Ok(FeatureCollection {
                    bbox: None,
                    features: vec![f],
                    foreign_members: None,
                }),
                GeoJson::Geometry(geom) => {
                    let feat = geojson::Feature {
                        bbox: None,
                        geometry: Some(geom),
                        id: None,
                        properties: None,
                        foreign_members: None,
                    };
                    Ok(FeatureCollection {
                        bbox: None,
                        features: vec![feat],
                        foreign_members: None,
                    })
                }
            }
        }
        "csv" => csv_to_fc(input),
        other => Err(format!("Unsupported input format: {other}")),
    }
}

fn fc_to_geojson(fc: &FeatureCollection) -> String {
    let gj = GeoJson::FeatureCollection(fc.clone());
    serde_json::to_string(&gj)
        .unwrap_or_else(|_| r#"{"type":"FeatureCollection","features":[]}"#.into())
}

fn csv_to_fc(csv_text: &str) -> Result<FeatureCollection, String> {
    // CSV with lon,lat columns (or x,y / longitude,latitude)
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(csv_text.as_bytes());

    let headers = rdr
        .headers()
        .map_err(|e| format!("CSV header error: {e}"))?
        .clone();

    let lat_idx = detect_column(&headers, &["lat", "latitude", "y"]);
    let lon_idx = detect_column(&headers, &["lon", "longitude", "lng", "x"]);
    let (lat_idx, lon_idx) = match (lat_idx, lon_idx) {
        (Some(li), Some(lo)) => (li, lo),
        _ => {
            return Err(
                "CSV must have latitude/longitude columns (lat/lon, latitude/longitude, or x/y)"
                    .into(),
            );
        }
    };

    let mut features = Vec::new();
    for result in rdr.records() {
        let record = result.map_err(|e| format!("CSV row error: {e}"))?;
        let lon: f64 = record
            .get(lon_idx)
            .and_then(|s| s.parse().ok())
            .ok_or("Missing longitude")?;
        let lat: f64 = record
            .get(lat_idx)
            .and_then(|s| s.parse().ok())
            .ok_or("Missing latitude")?;

        let mut props = serde_json::Map::new();
        for (i, h) in headers.iter().enumerate() {
            if i != lat_idx && i != lon_idx {
                if let Some(val) = record.get(i) {
                    props.insert(h.to_string(), serde_json::Value::String(val.to_string()));
                }
            }
        }

        let geom = geojson::Geometry::new(geojson::Value::Point(vec![lon, lat]));
        features.push(geojson::Feature {
            bbox: None,
            geometry: Some(geom),
            id: None,
            properties: Some(props),
            foreign_members: None,
        });
    }

    Ok(FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    })
}

fn detect_column(headers: &csv::StringRecord, candidates: &[&str]) -> Option<usize> {
    for (i, h) in headers.iter().enumerate() {
        let lower = h.to_lowercase();
        if candidates.iter().any(|c| lower == *c) {
            return Some(i);
        }
    }
    None
}

// ── Action handlers ───────────────────────────────────────────

fn handle_read(input: Option<String>, format: String) -> Result<(), Box<dyn std::error::Error>> {
    let content = read_input(input, &format)?;
    let fc = parse_fc(&content, &format).map_err(|e| Box::<dyn std::error::Error>::from(e))?;
    write_stdout(&fc_to_geojson(&fc))?;
    Ok(())
}

fn handle_buffer(distance: f64, units: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let input = read_stdin()?;
    let mut fc = parse_fc(&input, "geojson")?;

    for feature in &mut fc.features {
        if let Some(ref geom) = feature.geometry {
            let buffered = buffer_geometry(geom, distance, units.as_deref())?;
            feature.geometry = Some(buffered);
        }
    }

    write_stdout(&fc_to_geojson(&fc))?;
    Ok(())
}

fn buffer_geometry(
    geom: &geojson::Geometry,
    distance: f64,
    _units: Option<&str>,
) -> Result<geojson::Geometry, Box<dyn std::error::Error>> {
    // Buffer was removed from geo 0.28. Use geo-buffer crate or polyline offset.
    let _ = geom;
    let _ = distance;
    Err("buffer: geo 0.28 removed the buffer algorithm. Install 'geo-buffer' crate or use polyline offset.".into())
}

fn handle_simplify(epsilon: f64) -> Result<(), Box<dyn std::error::Error>> {
    let input = read_stdin()?;
    let mut fc = parse_fc(&input, "geojson")?;

    for feature in &mut fc.features {
        if let Some(ref geom) = feature.geometry {
            let simplified = simplify_geometry(geom, epsilon);
            feature.geometry = Some(simplified);
        }
    }

    write_stdout(&fc_to_geojson(&fc))?;
    Ok(())
}

fn simplify_geometry(geom: &geojson::Geometry, epsilon: f64) -> geojson::Geometry {
    // geo 0.28: Simplify is no longer implemented for Geometry<T>. Apply per-variant.
    use geo::algorithm::simplify::Simplify;
    let geo_geom: Option<geo::Geometry<f64>> = geom.value.clone().try_into().ok();
    match geo_geom {
        Some(g) => {
            let simplified = match g {
                geo::Geometry::LineString(ls) => geo::Geometry::LineString(ls.simplify(&epsilon)),
                geo::Geometry::MultiLineString(mls) => {
                    geo::Geometry::MultiLineString(mls.simplify(&epsilon))
                }
                geo::Geometry::Polygon(p) => geo::Geometry::Polygon(p.simplify(&epsilon)),
                geo::Geometry::MultiPolygon(mp) => {
                    geo::Geometry::MultiPolygon(mp.simplify(&epsilon))
                }
                other => other,
            };
            (&simplified).into()
        }
        None => geom.clone(),
    }
}

fn handle_reproject(from_epsg: u16, to_epsg: u16) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(feature = "proj-crs"))]
    {
        return Err(
            "reproject requires 'proj-crs' feature (compiled with --features proj-crs)".into(),
        );
    }
    #[cfg(feature = "proj-crs")]
    {
        let input = read_stdin()?;
        let mut fc = parse_fc(&input, "geojson")?;

        let from_str = format!("EPSG:{from_epsg}");
        let to_str = format!("EPSG:{to_epsg}");

        let transform = proj::Proj::new_known_crs(&from_str, &to_str, None)
            .map_err(|e| format!("PROJ error: {e}"))?;

        for feature in &mut fc.features {
            if let Some(ref geom) = feature.geometry {
                let reprojected = reproject_geometry(geom, &transform)?;
                feature.geometry = Some(reprojected);
            }
        }

        write_stdout(&fc_to_geojson(&fc))?;
        Ok(())
    }
}

#[cfg(feature = "proj-crs")]
fn reproject_geometry(
    geom: &geojson::Geometry,
    transform: &proj::Proj,
) -> Result<geojson::Geometry, Box<dyn std::error::Error>> {
    use proj::Coord;

    let value = match &geom.value {
        geojson::Value::Point(coords) => {
            let c = Coord {
                x: coords[0],
                y: coords[1],
                t: f64::NAN,
                z: f64::NAN,
            };
            let out = transform
                .convert(c)
                .map_err(|e| format!("Transform error: {e}"))?;
            geojson::Value::Point(vec![out.x, out.y])
        }
        geojson::Value::MultiPoint(points) => {
            let transformed: Vec<Vec<f64>> = points
                .iter()
                .map(|p| {
                    let c = Coord {
                        x: p[0],
                        y: p[1],
                        t: f64::NAN,
                        z: f64::NAN,
                    };
                    transform.convert(c).map(|out| vec![out.x, out.y])
                })
                .collect::<Result<_, _>>()
                .map_err(|e| format!("Transform error: {e}"))?;
            geojson::Value::MultiPoint(transformed)
        }
        geojson::Value::LineString(coords) => {
            let transformed: Vec<Vec<f64>> = coords
                .iter()
                .map(|p| {
                    let c = Coord {
                        x: p[0],
                        y: p[1],
                        t: f64::NAN,
                        z: f64::NAN,
                    };
                    transform.convert(c).map(|out| vec![out.x, out.y])
                })
                .collect::<Result<_, _>>()
                .map_err(|e| format!("Transform error: {e}"))?;
            geojson::Value::LineString(transformed)
        }
        geojson::Value::MultiLineString(lines) => {
            let transformed: Vec<Vec<Vec<f64>>> = lines
                .iter()
                .map(|line| {
                    line.iter()
                        .map(|p| {
                            let c = Coord {
                                x: p[0],
                                y: p[1],
                                t: f64::NAN,
                                z: f64::NAN,
                            };
                            transform.convert(c).map(|out| vec![out.x, out.y])
                        })
                        .collect::<Result<_, _>>()
                })
                .collect::<Result<_, _>>()
                .map_err(|e| format!("Transform error: {e}"))?;
            geojson::Value::MultiLineString(transformed)
        }
        geojson::Value::Polygon(rings) => {
            let transformed: Vec<Vec<Vec<f64>>> = rings
                .iter()
                .map(|ring| {
                    ring.iter()
                        .map(|p| {
                            let c = Coord {
                                x: p[0],
                                y: p[1],
                                t: f64::NAN,
                                z: f64::NAN,
                            };
                            transform.convert(c).map(|out| vec![out.x, out.y])
                        })
                        .collect::<Result<_, _>>()
                })
                .collect::<Result<_, _>>()
                .map_err(|e| format!("Transform error: {e}"))?;
            geojson::Value::Polygon(transformed)
        }
        geojson::Value::MultiPolygon(polys) => {
            let transformed: Vec<Vec<Vec<Vec<f64>>>> = polys
                .iter()
                .map(|poly| {
                    poly.iter()
                        .map(|ring| {
                            ring.iter()
                                .map(|p| {
                                    let c = Coord {
                                        x: p[0],
                                        y: p[1],
                                        t: f64::NAN,
                                        z: f64::NAN,
                                    };
                                    transform.convert(c).map(|out| vec![out.x, out.y])
                                })
                                .collect::<Result<_, _>>()
                        })
                        .collect::<Result<_, _>>()
                })
                .collect::<Result<_, _>>()
                .map_err(|e| format!("Transform error: {e}"))?;
            geojson::Value::MultiPolygon(transformed)
        }
        _ => geom.value.clone(),
    };

    Ok(geojson::Geometry {
        bbox: None,
        value,
        foreign_members: None,
    })
}

fn handle_write(output: String, format: String) -> Result<(), Box<dyn std::error::Error>> {
    let input = read_stdin()?;
    let fc = parse_fc(&input, "geojson")?;

    match format.to_lowercase().as_str() {
        "geojson" | "json" => {
            let gj = fc_to_geojson(&fc);
            write_file(&output, &gj)?;
        }
        "csv" => {
            let mut wtr = csv::Writer::from_path(&output)?;
            // Write header
            wtr.write_record(&["lon", "lat"])?;
            for feature in &fc.features {
                if let Some(ref geom) = feature.geometry {
                    write_geom_csv(&mut wtr, geom)?;
                }
            }
            wtr.flush()?;
        }
        other => {
            return Err(format!("Unsupported output format: {other}").into());
        }
    }

    eprintln!("✅ Wrote {} features to {output}", fc.features.len());
    Ok(())
}

fn write_geom_csv<W: std::io::Write>(
    wtr: &mut csv::Writer<W>,
    geom: &geojson::Geometry,
) -> Result<(), Box<dyn std::error::Error>> {
    /// Visit each coordinate pair and write as csv row
    use geojson::Value;

    fn visit<W: std::io::Write>(
        wtr: &mut csv::Writer<W>,
        coords: &[f64],
    ) -> Result<(), Box<dyn std::error::Error>> {
        wtr.write_record(&[coords[0].to_string(), coords[1].to_string()])?;
        Ok(())
    }

    fn visit_ring<W: std::io::Write>(
        wtr: &mut csv::Writer<W>,
        ring: &[Vec<f64>],
    ) -> Result<(), Box<dyn std::error::Error>> {
        for p in ring {
            visit(wtr, p)?;
        }
        Ok(())
    }

    match &geom.value {
        Value::Point(c) => visit(wtr, c)?,
        Value::MultiPoint(points) => {
            for p in points {
                visit(wtr, p)?;
            }
        }
        Value::LineString(coords) => {
            for c in coords {
                visit(wtr, c)?;
            }
        }
        Value::MultiLineString(lines) => {
            for line in lines {
                for c in line {
                    visit(wtr, c)?;
                }
            }
        }
        Value::Polygon(rings) => {
            for ring in rings {
                visit_ring(wtr, ring)?;
            }
        }
        Value::MultiPolygon(polys) => {
            for poly in polys {
                for ring in poly {
                    visit_ring(wtr, ring)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_area() -> Result<(), Box<dyn std::error::Error>> {
    use geo::algorithm::area::Area;

    let input = read_stdin()?;
    let fc = parse_fc(&input, "geojson")?;

    let mut total_area_sqm = 0.0;
    let mut feature_areas = Vec::new();

    for (i, feature) in fc.features.iter().enumerate() {
        if let Some(ref geom) = feature.geometry {
            let geo_geom: Option<geo::Geometry<f64>> = geom.value.clone().try_into().ok();
            if let Some(g) = geo_geom {
                let area = g.unsigned_area();
                total_area_sqm += area;
                feature_areas.push((i, area));
            }
        }
    }

    let result = serde_json::json!({
        "total_area_m2": total_area_sqm,
        "total_area_km2": total_area_sqm / 1_000_000.0,
        "total_area_ha": total_area_sqm / 10_000.0,
        "feature_count": fc.features.len(),
        "features": feature_areas.iter().map(|(i, a)| {
            serde_json::json!({"index": i, "area_m2": a})
        }).collect::<Vec<_>>(),
    });

    write_stdout(&serde_json::to_string_pretty(&result)?)?;
    Ok(())
}

fn handle_filter(key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
    let input = read_stdin()?;
    let mut fc = parse_fc(&input, "geojson")?;

    fc.features.retain(|f| {
        f.properties
            .as_ref()
            .and_then(|props| {
                props.get(key).map(|v| {
                    let v_str = match v {
                        serde_json::Value::String(s) => s.as_str(),
                        serde_json::Value::Number(n) => return n.to_string() == value,
                        _ => "",
                    };
                    v_str == value
                })
            })
            .unwrap_or(false)
    });

    write_stdout(&fc_to_geojson(&fc))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_geojson_fc() {
        let input = r#"{"type":"FeatureCollection","features":[{"type":"Feature","geometry":{"type":"Point","coordinates":[104.0,30.5]},"properties":{"name":"test"}}]}"#;
        let fc = parse_fc(input, "geojson").unwrap();
        assert_eq!(fc.features.len(), 1);
    }

    #[test]
    fn test_parse_geojson_feature() {
        let input = r#"{"type":"Feature","geometry":{"type":"Point","coordinates":[104.0,30.5]},"properties":{}}"#;
        let fc = parse_fc(input, "geojson").unwrap();
        assert_eq!(fc.features.len(), 1);
    }

    #[test]
    fn test_csv_to_fc() {
        let input = "name,lat,lon,temp\nBeijing,39.9,116.4,25.0\nShanghai,31.2,121.5,28.0\n";
        let fc = csv_to_fc(input).unwrap();
        assert_eq!(fc.features.len(), 2);
        let f0 = &fc.features[0];
        assert!(f0.properties.as_ref().unwrap().get("name").unwrap() == "Beijing");
    }

    #[test]
    fn test_csv_xy_headers() {
        let input = "city,x,y\nChengdu,104.1,30.6\n";
        let fc = csv_to_fc(input).unwrap();
        assert_eq!(fc.features.len(), 1);
    }

    #[test]
    fn test_fc_to_geojson_roundtrip() {
        let input = r#"{"type":"FeatureCollection","features":[{"type":"Feature","geometry":{"type":"Point","coordinates":[104.0,30.5]},"properties":{"name":"test"}}]}"#;
        let fc = parse_fc(input, "geojson").unwrap();
        let output = fc_to_geojson(&fc);
        let fc2 = parse_fc(&output, "geojson").unwrap();
        assert_eq!(fc2.features.len(), 1);
    }

    #[test]
    fn test_detect_column() {
        let headers = csv::StringRecord::from(vec!["name", "lat", "lon", "temp"]);
        assert_eq!(detect_column(&headers, &["lat", "latitude"]), Some(1));
        assert_eq!(detect_column(&headers, &["lon", "longitude", "x"]), Some(2));
        assert_eq!(detect_column(&headers, &["elevation"]), None);
    }

    #[test]
    fn test_buffer_simple() {
        let geom = geojson::Geometry::new(geojson::Value::Point(vec![104.0, 30.5]));
        let result = buffer_geometry(&geom, 1000.0, None).unwrap();
        match result.value {
            geojson::Value::Polygon(_) => {} // buffer of point = polygon
            _ => panic!("Expected polygon from point buffer"),
        }
    }

    #[test]
    fn test_simplify_simple() {
        // Line with many collinear points
        let coords: Vec<Vec<f64>> = (0..100)
            .map(|i| vec![104.0 + i as f64 * 0.001, 30.5])
            .collect();
        let geom = geojson::Geometry::new(geojson::Value::LineString(coords));
        let simplified = simplify_geometry(&geom, 0.01);
        match simplified.value {
            geojson::Value::LineString(ref pts) => {
                // Should have far fewer points than 100
                assert!(
                    pts.len() < 20,
                    "Expected simplified line, got {} pts",
                    pts.len()
                );
            }
            _ => panic!("Expected LineString"),
        }
    }

    #[test]
    fn test_parse_unsupported_format() {
        let result = parse_fc("{}", "shapefile");
        assert!(result.is_err());
    }
}
