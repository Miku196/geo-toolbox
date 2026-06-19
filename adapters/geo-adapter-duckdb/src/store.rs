//! SQLite 空间存储引擎（DuckDB 轻量替代）。
//!
//! 零安装部署，一个二进制搞定。支持内存和文件两种模式。

use geo_core::errors::{GeoError, GeoResult};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;
use tracing::info;

/// 校验表名安全（仅允许字母数字下划线）。
fn validate_table_name(name: &str) -> GeoResult<()> {
    if name.is_empty() || name.len() > 64 {
        return Err(GeoError::Validation("table name must be 1-64 chars".into()));
    }
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(GeoError::Validation(format!(
            "table name '{name}' contains invalid characters (only [a-zA-Z0-9_] allowed)"
        )));
    }
    Ok(())
}

pub struct DuckDbStore {
    conn: Mutex<Connection>,
}

impl DuckDbStore {
    /// 内存数据库（临时分析）。
    pub fn in_memory() -> GeoResult<Self> {
        let conn = Connection::open_in_memory().map_err(|e| GeoError::Database(e.to_string()))?;
        conn.execute_batch("PRAGMA journal_mode=WAL")
            .map_err(|e| GeoError::Database(e.to_string()))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// 文件数据库（持久化）。
    pub fn open(path: impl AsRef<Path>) -> GeoResult<Self> {
        let conn = Connection::open(path).map_err(|e| GeoError::Database(e.to_string()))?;
        conn.execute_batch("PRAGMA journal_mode=WAL")
            .map_err(|e| GeoError::Database(e.to_string()))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// 从 GeoJSON FeatureCollection 字符串导入。
    pub fn ingest_geojson_raw(&self, table: &str, fc_json: &str) -> GeoResult<usize> {
        validate_table_name(table)?;
        let fc: serde_json::Value = serde_json::from_str(fc_json).map_err(GeoError::Serde)?;
        let features = fc["features"]
            .as_array()
            .ok_or_else(|| GeoError::Validation("no features array".into()))?;

        // 建表
        self.conn
            .lock()
            .unwrap()
            .execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS \"{table}\" (\
                id INTEGER PRIMARY KEY AUTOINCREMENT, \
                name TEXT, category TEXT, \
                lon REAL, lat REAL, area_ha REAL, \
                props TEXT)"
                ),
                [],
            )
            .map_err(|e| GeoError::Database(e.to_string()))?;

        let mut count = 0;
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            &format!("INSERT INTO \"{table}\" (name, category, lon, lat, area_ha, props) VALUES (?,?,?,?,?,?)")
        ).map_err(|e| GeoError::Database(e.to_string()))?;

        for feat in features {
            let props = &feat["properties"];
            let geom = &feat["geometry"];
            let coords = &geom["coordinates"];
            let (lon, lat) = if geom["type"] == "Point" {
                (
                    coords[0].as_f64().unwrap_or(0.0),
                    coords[1].as_f64().unwrap_or(0.0),
                )
            } else {
                // Polygon/Multi: 取第一个坐标作为参考点
                let c = &coords[0];
                if c.is_array() && c[0].is_array() {
                    (
                        c[0][0].as_f64().unwrap_or(0.0),
                        c[0][1].as_f64().unwrap_or(0.0),
                    )
                } else {
                    (c[0].as_f64().unwrap_or(0.0), c[1].as_f64().unwrap_or(0.0))
                }
            };

            let name = props["name"].as_str().unwrap_or("").to_string();
            let cat = props["type"]
                .as_str()
                .or(props["class"].as_str())
                .unwrap_or("")
                .to_string();
            let area = props["area_ha"].as_f64();
            let props_str = serde_json::to_string(props).unwrap_or_default();

            stmt.execute(params![name, cat, lon, lat, area, props_str])
                .map_err(|e| GeoError::Database(e.to_string()))?;
            count += 1;
        }

        info!(count, table = %table, "geo-io ingested features");
        Ok(count)
    }

    /// 查询返回 JSON。
    pub fn query_json(&self, sql: &str) -> GeoResult<Vec<serde_json::Value>> {
        geo_core::errors::validate_select_sql(sql)?;

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| GeoError::Database(e.to_string()))?;

        let col_names: Vec<String> = (0..stmt.column_count())
            .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
            .collect();

        let rows = stmt
            .query_map([], |row| {
                let mut map = serde_json::Map::new();
                for (i, name) in col_names.iter().enumerate() {
                    let val: rusqlite::types::Value = row.get_unwrap(i);
                    let json_val = match val {
                        rusqlite::types::Value::Null => serde_json::Value::Null,
                        rusqlite::types::Value::Integer(i) => serde_json::json!(i),
                        rusqlite::types::Value::Real(f) => serde_json::json!(f),
                        rusqlite::types::Value::Text(s) => serde_json::Value::String(s),
                        rusqlite::types::Value::Blob(_) => {
                            serde_json::Value::String("<blob>".into())
                        }
                    };
                    map.insert(name.clone(), json_val);
                }
                Ok(serde_json::Value::Object(map))
            })
            .map_err(|e| GeoError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| GeoError::Database(e.to_string()))?);
        }
        Ok(results)
    }

    /// 空间范围查询（按经纬度 bbox 过滤）。
    pub fn query_bbox(
        &self,
        table: &str,
        min_lon: f64,
        min_lat: f64,
        max_lon: f64,
        max_lat: f64,
    ) -> GeoResult<Vec<serde_json::Value>> {
        validate_table_name(table)?;
        let sql = format!(
            "SELECT * FROM \"{table}\" WHERE lon BETWEEN {min_lon} AND {max_lon} AND lat BETWEEN {min_lat} AND {max_lat}"
        );
        self.query_json(&sql)
    }

    /// 列出所有表。
    pub fn list_tables(&self) -> GeoResult<Vec<String>> {
        let rows =
            self.query_json("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")?;
        Ok(rows
            .iter()
            .filter_map(|r| r["name"].as_str().map(str::to_string))
            .collect())
    }

    /// 暴露底层连接。
    pub fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }

    /// 健康检查。
    pub fn ping(&self) -> GeoResult<()> {
        let result = self
            .conn
            .lock()
            .unwrap()
            .execute_batch("SELECT 1")
            .map_err(|_| GeoError::Database("ping failed".into()));
        info!(healthy = result.is_ok(), "sqlite ping");
        result
    }

    /// 表行数。
    pub fn count(&self, table: &str) -> GeoResult<i64> {
        validate_table_name(table)?;
        self.conn
            .lock()
            .unwrap()
            .query_row(&format!("SELECT COUNT(*) FROM \"{table}\""), [], |row| {
                row.get(0)
            })
            .map_err(|e| GeoError::Database(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_basic() {
        let store = DuckDbStore::in_memory().unwrap();
        store.ping().unwrap();
    }

    #[test]
    fn test_ingest_and_query() {
        let store = DuckDbStore::in_memory().unwrap();
        let geojson = r#"{
            "type": "FeatureCollection",
            "features": [
                {"type":"Feature","properties":{"name":"A","type":"forest","area_ha":100},
                 "geometry":{"type":"Point","coordinates":[104.06,30.57]}},
                {"type":"Feature","properties":{"name":"B","type":"grassland","area_ha":50},
                 "geometry":{"type":"Point","coordinates":[104.07,30.58]}}
            ]
        }"#;
        let count = store.ingest_geojson_raw("sites", geojson).unwrap();
        assert_eq!(count, 2);

        let rows = store.query_json("SELECT * FROM sites").unwrap();
        assert_eq!(rows.len(), 2);

        let bbox = store.query_bbox("sites", 104.0, 30.5, 104.1, 30.6).unwrap();
        assert_eq!(bbox.len(), 2);
    }

    #[test]
    fn test_list_tables() {
        let store = DuckDbStore::in_memory().unwrap();
        store
            .lock()
            .execute("CREATE TABLE test (id INT)", [])
            .unwrap();
        assert!(store.list_tables().unwrap().contains(&"test".to_string()));
    }

    #[test]
    fn test_bbox_out_of_range() {
        let store = DuckDbStore::in_memory().unwrap();
        store.ingest_geojson_raw("pts", r#"{"type":"FeatureCollection","features":[
            {"type":"Feature","properties":{"name":"Chengdu"},"geometry":{"type":"Point","coordinates":[104.06,30.57]}}
        ]}"#).unwrap();

        // 查询北京区域，不包含成都
        let bbox = store.query_bbox("pts", 116.0, 39.0, 117.0, 40.0).unwrap();
        assert_eq!(bbox.len(), 0);
    }
}
