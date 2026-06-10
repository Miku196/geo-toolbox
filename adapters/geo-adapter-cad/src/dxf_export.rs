//! DXF (Drawing Exchange Format) export from PostGIS vectors.
//!
//! Uses `dxf` crate 0.6 to produce AutoCAD-compatible .dxf files.
//! Outputs LINE entities for all geometry types.

use geo_core::crs::CrsRegistry;
use geo_core::errors::{self, GeoError, GeoResult};
use sqlx::postgres::PgPool;
use sqlx::Row;

pub struct DxfExporter { pool: PgPool }

impl DxfExporter {
    pub fn new(pool: PgPool) -> Self { Self { pool } }

    pub async fn from_sql(
        &self, sql: &str, output_path: &str,
        source_epsg: u16, target_epsg: u16,
    ) -> GeoResult<usize> {
        errors::validate_select_sql(sql)?;
        let rows = sqlx::query(sql).fetch_all(&self.pool).await
            .map_err(|e| GeoError::Database(e.to_string()))?;

        let mut drawing = dxf::Drawing::new();
        let crs = CrsRegistry::new();
        let mut count = 0usize;

        for row in &rows {
            let geojson_str: String = row.try_get("geom_json").unwrap_or_default();
            if geojson_str.is_empty() { continue; }

            let geom: serde_json::Value = serde_json::from_str(&geojson_str).unwrap_or_default();
            let gtype = geom["type"].as_str().unwrap_or("Point");
            let coords = &geom["coordinates"];

            let xform = |x: f64, y: f64| -> (f64, f64) {
                if source_epsg != target_epsg {
                    crs.transform_point(source_epsg, target_epsg, x, y).unwrap_or((x, y))
                } else { (x, y) }
            };

            match gtype {
                "Point" => {
                    if let (Some(x), Some(y)) = (coords[0].as_f64(), coords[1].as_f64()) {
                        let (dx, dy) = xform(x, y);
                        drawing.add_entity(dxf::entities::Entity::new(
                            dxf::entities::EntityType::Line(dxf::entities::Line::new(
                                dxf::Point::new(dx, dy, 0.0),
                                dxf::Point::new(dx + 0.1, dy + 0.1, 0.0),
                            )),
                        ));
                        count += 1;
                    }
                }
                "LineString" | "MultiPoint" => {
                    let pts: Vec<&serde_json::Value> = if gtype == "MultiPoint" {
                        coords.as_array().map(|a| a.iter().collect()).unwrap_or_default()
                    } else {
                        coords.as_array().map(|a| a.iter().collect()).unwrap_or_default()
                    };
                    for w in pts.windows(2) {
                        if let (Some(x1), Some(y1), Some(x2), Some(y2)) =
                            (w[0][0].as_f64(), w[0][1].as_f64(), w[1][0].as_f64(), w[1][1].as_f64())
                        {
                            let (d1x, d1y) = xform(x1, y1);
                            let (d2x, d2y) = xform(x2, y2);
                            drawing.add_entity(dxf::entities::Entity::new(
                                dxf::entities::EntityType::Line(dxf::entities::Line::new(
                                    dxf::Point::new(d1x, d1y, 0.0),
                                    dxf::Point::new(d2x, d2y, 0.0),
                                )),
                            ));
                            count += 1;
                        }
                    }
                }
                "Polygon" => {
                    if let Some(rings) = coords.as_array() {
                        for ring in rings {
                            if let Some(pts) = ring.as_array() {
                                let points: Vec<&serde_json::Value> = pts.iter().collect();
                                for w in points.windows(2) {
                                    if let (Some(x1), Some(y1), Some(x2), Some(y2)) =
                                        (w[0][0].as_f64(), w[0][1].as_f64(), w[1][0].as_f64(), w[1][1].as_f64())
                                    {
                                        let (d1x, d1y) = xform(x1, y1);
                                        let (d2x, d2y) = xform(x2, y2);
                                        drawing.add_entity(dxf::entities::Entity::new(
                                            dxf::entities::EntityType::Line(dxf::entities::Line::new(
                                                dxf::Point::new(d1x, d1y, 0.0),
                                                dxf::Point::new(d2x, d2y, 0.0),
                                            )),
                                        ));
                                        count += 1;
                                    }
                                }
                                // Close ring
                                if points.len() >= 2 {
                                    let first = &points[0];
                                    let last = &points[points.len() - 1];
                                    if let (Some(x1), Some(y1), Some(x2), Some(y2)) =
                                        (last[0].as_f64(), last[1].as_f64(), first[0].as_f64(), first[1].as_f64())
                                    {
                                        let (d1x, d1y) = xform(x1, y1);
                                        let (d2x, d2y) = xform(x2, y2);
                                        drawing.add_entity(dxf::entities::Entity::new(
                                            dxf::entities::EntityType::Line(dxf::entities::Line::new(
                                                dxf::Point::new(d1x, d1y, 0.0),
                                                dxf::Point::new(d2x, d2y, 0.0),
                                            )),
                                        ));
                                        count += 1;
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        drawing.save_file(output_path)
            .map_err(|e| GeoError::Other(format!("dxf: {e}")))?;
        tracing::info!("DXF: {output_path} ({count} entities)");
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_empty_drawing() {
        let d = dxf::Drawing::new();
        let tmp = std::env::temp_dir().join("geo_dxf_test.dxf");
        d.save_file(&tmp).unwrap();
        assert!(tmp.exists());
        let _ = std::fs::remove_file(&tmp);
    }
}
