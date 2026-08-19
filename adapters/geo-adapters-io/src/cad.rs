//! CAD/DXF/Excel/GeoJSON export adapters.

use geo_core::errors::{self, GeoError, GeoResult};
use geo_core::plugin::{ExternalAdapter, GeoFeature, Plugin, PluginCategory};

// ---------------------------------------------------------------------------
// CadAdapter
// ---------------------------------------------------------------------------

pub struct CadAdapter;
impl Default for CadAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl CadAdapter {
    pub fn new() -> Self {
        Self
    }
}
impl Plugin for CadAdapter {
    type Config = geo_core::plugin::EmptyConfig;
    fn new(_config: Self::Config) -> Self {
        Self
    }
    fn name(&self) -> &str {
        "cad"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "CAD format adapter (DXF/DWG)"
    }
    fn category(&self) -> PluginCategory {
        PluginCategory::Adapter
    }
}
impl ExternalAdapter for CadAdapter {
    fn external_endpoint(&self) -> &str {
        "dxf"
    }
    async fn health_check(&self) -> GeoResult<bool> {
        // CadAdapter has no CAD/DXF backend wired on itself (exporting lives on
        // DxfExporter / GeoJsonExporter), so it is genuinely unavailable here.
        // Report Ok(false) instead of a fake Ok(true).
        Ok(false)
    }
    async fn external_version(&self) -> GeoResult<String> {
        // No real DXF/DWG driver is queried for a version by this adapter.
        Err(GeoError::Unimplemented(
            "cad CadAdapter: external_version not queried — no DXF/DWG driver wired".into(),
        ))
    }
    fn requires_network(&self) -> bool {
        false
    }
    async fn push(&self, _t: &str, _d: &[GeoFeature]) -> GeoResult<u64> {
        Err(GeoError::Unimplemented(
            "cad CadAdapter: push not implemented — DXF/DWG export wiring unfinished (use cad_export_geojson / DxfExporter)"
                .into(),
        ))
    }
    async fn pull(&self, _q: &str) -> GeoResult<Vec<GeoFeature>> {
        Err(GeoError::Unimplemented(
            "cad CadAdapter: pull not implemented — no CAD data source wiring".into(),
        ))
    }
    async fn execute(&self, _c: &str, _p: serde_json::Value) -> GeoResult<serde_json::Value> {
        // Explicit failure instead of a fake {"status":"ok"}: the DXF/DWG export
        // backend is not wired to CadAdapter::execute.
        Err(GeoError::Unimplemented(
            "cad CadAdapter: execute not implemented (DXF/DWG exporter not wired to this adapter; use DxfExporter/GeoJsonExporter)"
                .into(),
        ))
    }
}

// ---------------------------------------------------------------------------
// DxfExporter
// ---------------------------------------------------------------------------

pub struct DxfExporter {
    pool: sqlx::postgres::PgPool,
}

impl DxfExporter {
    pub fn new(pool: sqlx::postgres::PgPool) -> Self {
        Self { pool }
    }

    pub async fn from_sql(
        &self,
        sql: &str,
        output_path: &str,
        source_epsg: u16,
        target_epsg: u16,
    ) -> GeoResult<usize> {
        errors::validate_select_sql(sql)?;
        let rows = sqlx::query(sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| GeoError::Database(e.to_string()))?;

        let mut drawing = dxf::Drawing::new();
        let crs = geo_core::crs::CrsRegistry::new();
        let mut count = 0usize;

        for row in &rows {
            let geojson_str: String = row.try_get("geom_json").unwrap_or_default();
            if geojson_str.is_empty() {
                continue;
            }

            let geom: serde_json::Value = serde_json::from_str(&geojson_str).unwrap_or_default();
            let gtype = geom["type"].as_str().unwrap_or("Point");
            let coords = &geom["coordinates"];

            let xform = |x: f64, y: f64| -> (f64, f64) {
                if source_epsg != target_epsg {
                    crs.transform_point(source_epsg, target_epsg, x, y)
                        .unwrap_or((x, y))
                } else {
                    (x, y)
                }
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
                    let pts: Vec<&serde_json::Value> = coords
                        .as_array()
                        .map(|a| a.iter().collect())
                        .unwrap_or_default();
                    for w in pts.windows(2) {
                        if let (Some(x1), Some(y1), Some(x2), Some(y2)) = (
                            w[0][0].as_f64(),
                            w[0][1].as_f64(),
                            w[1][0].as_f64(),
                            w[1][1].as_f64(),
                        ) {
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
                                    if let (Some(x1), Some(y1), Some(x2), Some(y2)) = (
                                        w[0][0].as_f64(),
                                        w[0][1].as_f64(),
                                        w[1][0].as_f64(),
                                        w[1][1].as_f64(),
                                    ) {
                                        let (d1x, d1y) = xform(x1, y1);
                                        let (d2x, d2y) = xform(x2, y2);
                                        drawing.add_entity(dxf::entities::Entity::new(
                                            dxf::entities::EntityType::Line(
                                                dxf::entities::Line::new(
                                                    dxf::Point::new(d1x, d1y, 0.0),
                                                    dxf::Point::new(d2x, d2y, 0.0),
                                                ),
                                            ),
                                        ));
                                        count += 1;
                                    }
                                }
                                if points.len() >= 2 {
                                    let first = &points[0];
                                    let last = &points[points.len() - 1];
                                    if let (Some(x1), Some(y1), Some(x2), Some(y2)) = (
                                        last[0].as_f64(),
                                        last[1].as_f64(),
                                        first[0].as_f64(),
                                        first[1].as_f64(),
                                    ) {
                                        let (d1x, d1y) = xform(x1, y1);
                                        let (d2x, d2y) = xform(x2, y2);
                                        drawing.add_entity(dxf::entities::Entity::new(
                                            dxf::entities::EntityType::Line(
                                                dxf::entities::Line::new(
                                                    dxf::Point::new(d1x, d1y, 0.0),
                                                    dxf::Point::new(d2x, d2y, 0.0),
                                                ),
                                            ),
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

        drawing
            .save_file(output_path)
            .map_err(|e| GeoError::Other(format!("dxf: {e}")))?;
        tracing::info!("DXF: {output_path} ({count} entities)");
        Ok(count)
    }
}

// ---------------------------------------------------------------------------
// ExcelDashboard
// ---------------------------------------------------------------------------

use rust_xlsxwriter::*;
use sqlx::{Column, Row};
use std::collections::HashMap;

macro_rules! xe {
    ($e:expr) => {
        $e.map_err(|e| GeoError::Other(format!("xlsx: {e}")))
    };
}

pub struct ExcelDashboard {
    pool: sqlx::postgres::PgPool,
}

impl ExcelDashboard {
    pub fn new(pool: sqlx::postgres::PgPool) -> Self {
        Self { pool }
    }

    pub async fn from_sql(&self, sql: &str, output_path: &str, sheet_name: &str) -> GeoResult<()> {
        errors::validate_select_sql(sql)?;
        let rows = sqlx::query(sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| GeoError::Database(e.to_string()))?;
        if rows.is_empty() {
            return Err(GeoError::Validation("Query returned 0 rows".into()));
        }

        let mut workbook = Workbook::new();
        let sheet = workbook.add_worksheet();
        xe!(sheet.set_name(sheet_name))?;

        let columns: Vec<String> = rows[0]
            .columns()
            .iter()
            .map(|c| c.name().to_string())
            .collect();
        let column_types = self.detect_types(&rows, &columns);

        let header_fmt = Format::new()
            .set_bold()
            .set_background_color(Color::RGB(0x4472C4))
            .set_font_color(Color::White)
            .clone();

        for (i, name) in columns.iter().enumerate() {
            xe!(sheet.write_string_with_format(0, i as u16, name, &header_fmt))?;
        }

        for (row_idx, row) in rows.iter().enumerate() {
            let r = (row_idx + 1) as u32;
            for (col_idx, col_name) in columns.iter().enumerate() {
                let ct = column_types
                    .get(col_name)
                    .map(|s| s.as_str())
                    .unwrap_or("text");
                self.write_cell(sheet, r, col_idx as u16, row, col_name, ct)?;
            }
        }

        for (i, name) in columns.iter().enumerate() {
            let w = (name.len() as u16).max(12);
            xe!(sheet.set_column_width(i as u16, w + 4))?;
        }
        xe!(sheet.set_freeze_panes(1, 0))?;
        xe!(workbook.save(output_path))?;
        tracing::info!("Excel: {output_path} ({} rows)", rows.len());
        Ok(())
    }

    pub async fn carbon_report(&self, aoi_id: uuid::Uuid, output_path: &str) -> GeoResult<()> {
        let rows = sqlx::query(
            r#"SELECT landcover_class AS "Landcover Class",
               ROUND(area_ha::numeric,1) AS "Area (ha)",
               ROUND(emission_tco2e::numeric,1) AS "tCO₂e",
               audit_status AS "Audit Status"
               FROM carbon_accounting_results WHERE aoi_id = $1
               ORDER BY calculation_at DESC"#,
        )
        .bind(aoi_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| GeoError::Database(e.to_string()))?;

        if rows.is_empty() {
            return Err(GeoError::Validation("Query returned 0 rows".into()));
        }

        let mut workbook = Workbook::new();
        let sheet = workbook.add_worksheet();
        xe!(sheet.set_name("Carbon Report"))?;

        let columns: Vec<String> = rows[0]
            .columns()
            .iter()
            .map(|c| c.name().to_string())
            .collect();
        let column_types = self.detect_types(&rows, &columns);

        let header_fmt = Format::new()
            .set_bold()
            .set_background_color(Color::RGB(0x4472C4))
            .set_font_color(Color::White)
            .clone();

        for (i, name) in columns.iter().enumerate() {
            xe!(sheet.write_string_with_format(0, i as u16, name, &header_fmt))?;
        }

        for (row_idx, row) in rows.iter().enumerate() {
            let r = (row_idx + 1) as u32;
            for (col_idx, col_name) in columns.iter().enumerate() {
                let ct = column_types
                    .get(col_name)
                    .map(|s| s.as_str())
                    .unwrap_or("text");
                self.write_cell(sheet, r, col_idx as u16, row, col_name, ct)?;
            }
        }

        for (i, name) in columns.iter().enumerate() {
            let w = (name.len() as u16).max(12);
            xe!(sheet.set_column_width(i as u16, w + 4))?;
        }
        xe!(sheet.set_freeze_panes(1, 0))?;
        xe!(workbook.save(output_path))?;
        tracing::info!("Excel: {output_path} ({} rows)", rows.len());
        Ok(())
    }

    fn detect_types(
        &self,
        rows: &[sqlx::postgres::PgRow],
        columns: &[String],
    ) -> HashMap<String, String> {
        let mut types = HashMap::new();
        for col in columns {
            let mut d = "text";
            for row in rows.iter().take(5) {
                if let Some(i) = row.columns().iter().position(|c| c.name() == col) {
                    if row.try_get::<f64, _>(i).is_ok() {
                        d = "number";
                        break;
                    }
                    if row.try_get::<i64, _>(i).is_ok() {
                        d = "integer";
                        break;
                    }
                }
            }
            types.insert(col.clone(), d.into());
        }
        types
    }

    fn write_cell(
        &self,
        sheet: &mut Worksheet,
        row: u32,
        col: u16,
        pg_row: &sqlx::postgres::PgRow,
        col_name: &str,
        col_type: &str,
    ) -> GeoResult<()> {
        let idx = pg_row.columns().iter().position(|c| c.name() == col_name);
        let Some(i) = idx else {
            xe!(sheet.write_string(row, col, ""))?;
            return Ok(());
        };

        if col_type == "number" {
            if let Ok(v) = pg_row.try_get::<f64, _>(i) {
                if v.is_finite() {
                    xe!(sheet.write_number(row, col, v))?;
                } else {
                    xe!(sheet.write_string(row, col, "N/A"))?;
                }
                return Ok(());
            }
        }
        if col_type == "integer" {
            if let Ok(v) = pg_row.try_get::<i64, _>(i) {
                xe!(sheet.write_number(row, col, v as f64))?;
                return Ok(());
            }
        }
        let s: String = pg_row.try_get(i).unwrap_or_else(|_| "NULL".into());
        xe!(sheet.write_string(row, col, &s))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// GeoJsonExporter
// ---------------------------------------------------------------------------

/// Exports PostGIS query results to GeoJSON files.
pub struct GeoJsonExporter {
    pool: sqlx::postgres::PgPool,
}

impl GeoJsonExporter {
    pub fn new(pool: sqlx::postgres::PgPool) -> Self {
        Self { pool }
    }

    pub async fn from_sql(&self, sql: &str, output_path: &str) -> GeoResult<usize> {
        errors::validate_select_sql(sql)?;
        let rows = sqlx::query(sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| GeoError::Database(e.to_string()))?;

        let features: Vec<serde_json::Value> = rows
            .iter()
            .filter_map(|row| {
                let json_str: Option<String> = row.try_get("feature").ok();
                json_str.and_then(|s| serde_json::from_str(&s).ok())
            })
            .collect();

        let fc = serde_json::json!({
            "type": "FeatureCollection",
            "features": features,
        });

        let json = serde_json::to_string_pretty(&fc)?;
        tokio::fs::write(output_path, &json).await?;

        let count = fc["features"].as_array().map(|a| a.len()).unwrap_or(0);
        tracing::info!("GeoJSON exported: {output_path} ({count} features)");
        Ok(count)
    }

    pub async fn from_aggregate_sql(&self, sql: &str, output_path: &str) -> GeoResult<usize> {
        let geojson_str: Option<String> = sqlx::query_scalar(sql)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| GeoError::Database(e.to_string()))?
            .flatten();

        let geojson_str = geojson_str
            .unwrap_or_else(|| r#"{"type":"FeatureCollection","features":[]}"#.to_string());

        let parsed: serde_json::Value = serde_json::from_str(&geojson_str)?;
        let pretty = serde_json::to_string_pretty(&parsed)?;
        tokio::fs::write(output_path, &pretty).await?;

        let count = parsed["features"].as_array().map(|a| a.len()).unwrap_or(0);
        tracing::info!("GeoJSON exported: {output_path} ({count} features)");
        Ok(count)
    }

    pub async fn export_aoi(&self, aoi_id: uuid::Uuid, output_path: &str) -> GeoResult<usize> {
        let geojson_str: Option<String> = sqlx::query_scalar(
            r#"
            SELECT jsonb_build_object(
                'type', 'FeatureCollection',
                'features', COALESCE(jsonb_agg(
                    jsonb_build_object(
                        'type', 'Feature',
                        'geometry', ST_AsGeoJSON(geom)::jsonb,
                        'properties', properties
                    )
                ), '[]'::jsonb)
            )::text AS geojson
            FROM spatial_assets
            WHERE aoi_id = $1
            "#,
        )
        .bind(aoi_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| GeoError::Database(e.to_string()))?
        .flatten();

        let geojson_str = geojson_str
            .unwrap_or_else(|| r#"{"type":"FeatureCollection","features":[]}"#.to_string());

        let parsed: serde_json::Value = serde_json::from_str(&geojson_str)?;
        let pretty = serde_json::to_string_pretty(&parsed)?;
        tokio::fs::write(output_path, &pretty).await?;

        let count = parsed["features"].as_array().map(|a| a.len()).unwrap_or(0);
        tracing::info!("GeoJSON exported: {output_path} ({count} features)");
        Ok(count)
    }
}

// ---------------------------------------------------------------------------
// Tool registration — CAD
// ---------------------------------------------------------------------------

pub fn register_tools(registry: &mut geo_registry::PluginRegistry) {
    geo_registry::register_plugin!(registry, "cad", "CAD format exporter: GeoJSON from PostGIS", PluginCategory::Output, [
        async "cad_export_geojson" => "Export PostGIS query to GeoJSON file" ; serde_json::json!({"type":"object","properties":{"sql":{"type":"string"},"output":{"type":"string"},"db_url":{"type":"string"}},"required":["sql","output","db_url"]}) => |args| Box::pin(async move {
        let db = args["db_url"].as_str().unwrap_or("");
        if db.is_empty() { return Err(geo_core::GeoError::invalid_input("db_url","required")); }
        let pool = sqlx::postgres::PgPoolOptions::new().max_connections(2).connect(db).await.map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
        let count = GeoJsonExporter::new(pool).from_sql(args["sql"].as_str().unwrap_or(""), args["output"].as_str().unwrap_or("")).await.map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(serde_json::json!({"output":args["output"].as_str().unwrap_or(""),"features":count}))
    })]);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use geo_core::errors::GeoError;

    #[test]
    fn test_cad_adapter() {
        let a = CadAdapter::new();
        assert!(!a.requires_network());
    }

    #[tokio::test]
    async fn test_cad_execute_does_not_fake_success() {
        // The DXF export backend is not wired to CadAdapter::execute: it must fail
        // explicitly instead of returning a fake {"status":"ok"}.
        let a = CadAdapter::new();
        let err = a
            .execute("dxf_export", serde_json::json!({"input": "x.geojson"}))
            .await
            .expect_err("execute must not fake success");
        assert!(matches!(err, GeoError::Unimplemented(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn test_cad_push_pull_not_implemented() {
        let a = CadAdapter::new();
        assert!(matches!(
            a.push("t", &[]).await,
            Err(GeoError::Unimplemented(_))
        ));
        assert!(matches!(a.pull("q").await, Err(GeoError::Unimplemented(_))));
    }

    #[tokio::test]
    async fn test_cad_health_check_reports_unavailable() {
        let a = CadAdapter::new();
        // With no CAD/DXF backend wired, health must not blindly report Ok(true).
        assert_eq!(a.health_check().await.unwrap(), false);
    }

    #[tokio::test]
    async fn test_cad_external_version_not_faked() {
        let a = CadAdapter::new();
        assert!(matches!(
            a.external_version().await,
            Err(GeoError::Unimplemented(_))
        ));
    }

    #[test]
    fn test_empty_fc() {
        let fc = serde_json::json!({
            "type": "FeatureCollection",
            "features": []
        });
        assert_eq!(fc["type"], "FeatureCollection");
        assert!(fc["features"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_empty_drawing() {
        let d = dxf::Drawing::new();
        let tmp = std::env::temp_dir().join("geo_dxf_test.dxf");
        d.save_file(&tmp).unwrap();
        assert!(tmp.exists());
        let _ = std::fs::remove_file(&tmp);
    }
}
