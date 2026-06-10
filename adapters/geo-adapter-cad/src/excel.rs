//! Excel dashboard generation from PostGIS queries.

use geo_core::errors::{self, GeoError, GeoResult};
use rust_xlsxwriter::*;
use sqlx::postgres::PgPool;
use sqlx::{Column, Row};
use std::collections::HashMap;

pub struct ExcelDashboard {
    pool: PgPool,
}

macro_rules! xe {
    ($e:expr) => { $e.map_err(|e| GeoError::Other(format!("xlsx: {e}"))) };
}

impl ExcelDashboard {
    pub fn new(pool: PgPool) -> Self { Self { pool } }

    pub async fn from_sql(&self, sql: &str, output_path: &str, sheet_name: &str) -> GeoResult<()> {
        errors::validate_select_sql(sql)?;
        let rows = sqlx::query(sql).fetch_all(&self.pool).await
            .map_err(|e| GeoError::Database(e.to_string()))?;
        if rows.is_empty() {
            return Err(GeoError::Validation("Query returned 0 rows".into()));
        }

        let mut workbook = Workbook::new();
        let sheet = workbook.add_worksheet();
        xe!(sheet.set_name(sheet_name))?;

        let columns: Vec<String> = rows[0].columns().iter().map(|c| c.name().to_string()).collect();
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
                let ct = column_types.get(col_name).map(|s| s.as_str()).unwrap_or("text");
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
        // 安全：使用参数化查询，aoi_id 作为 $1 绑定
        let rows = sqlx::query(
            r#"SELECT landcover_class AS "Landcover Class",
               ROUND(area_ha::numeric,1) AS "Area (ha)",
               ROUND(emission_tco2e::numeric,1) AS "tCO₂e",
               audit_status AS "Audit Status"
               FROM carbon_accounting_results WHERE aoi_id = $1
               ORDER BY calculation_at DESC"#
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

        let columns: Vec<String> = rows[0].columns().iter().map(|c| c.name().to_string()).collect();
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
                let ct = column_types.get(col_name).map(|s| s.as_str()).unwrap_or("text");
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

    fn detect_types(&self, rows: &[sqlx::postgres::PgRow], columns: &[String]) -> HashMap<String, String> {
        let mut types = HashMap::new();
        for col in columns {
            let mut d = "text";
            for row in rows.iter().take(5) {
                if let Some(i) = row.columns().iter().position(|c| c.name() == col) {
                    if row.try_get::<f64, _>(i).is_ok() { d = "number"; break; }
                    if row.try_get::<i64, _>(i).is_ok() { d = "integer"; break; }
                }
            }
            types.insert(col.clone(), d.into());
        }
        types
    }

    fn write_cell(&self, sheet: &mut Worksheet, row: u32, col: u16,
                  pg_row: &sqlx::postgres::PgRow, col_name: &str, col_type: &str) -> GeoResult<()> {
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
