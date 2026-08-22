//! DuckDB adapter — embedded local spatial analysis engine (SQLite backend).

use geo_core::errors::{GeoError, GeoResult};
use geo_core::plugin::{ExternalAdapter, GeoFeature, Plugin, PluginCategory};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;
use tracing::info;

// ---------------------------------------------------------------------------
// DuckDbStore
// ---------------------------------------------------------------------------

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
    pub fn in_memory() -> GeoResult<Self> {
        let conn = Connection::open_in_memory().map_err(|e| GeoError::Database(e.to_string()))?;
        conn.execute_batch("PRAGMA journal_mode=WAL")
            .map_err(|e| GeoError::Database(e.to_string()))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn open(path: impl AsRef<Path>) -> GeoResult<Self> {
        let conn = Connection::open(path).map_err(|e| GeoError::Database(e.to_string()))?;
        conn.execute_batch("PRAGMA journal_mode=WAL")
            .map_err(|e| GeoError::Database(e.to_string()))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn ingest_geojson_raw(&self, table: &str, fc_json: &str) -> GeoResult<usize> {
        validate_table_name(table)?;
        let fc: serde_json::Value = serde_json::from_str(fc_json).map_err(GeoError::Serde)?;
        let features = fc["features"]
            .as_array()
            .ok_or_else(|| GeoError::Validation("no features array".into()))?;

        let coordinates = |index: usize, feature: &serde_json::Value| -> GeoResult<(f64, f64)> {
            let geometry = feature.get("geometry").ok_or_else(|| {
                GeoError::Validation(format!("feature {index}: missing geometry"))
            })?;
            let geometry_type = geometry["type"].as_str().ok_or_else(|| {
                GeoError::Validation(format!(
                    "feature {index}: geometry type is missing or invalid"
                ))
            })?;
            let raw = geometry.get("coordinates").ok_or_else(|| {
                GeoError::Validation(format!("feature {index}: missing coordinates"))
            })?;
            let position = match geometry_type {
                "Point" => raw.as_array(),
                "MultiPoint" | "LineString" => raw.get(0).and_then(serde_json::Value::as_array),
                "MultiLineString" | "Polygon" => raw
                    .get(0)
                    .and_then(|line| line.get(0))
                    .and_then(serde_json::Value::as_array),
                _ => {
                    return Err(GeoError::Validation(format!(
                        "feature {index}: unsupported geometry type '{geometry_type}'"
                    )))
                }
            }
            .ok_or_else(|| {
                GeoError::Validation(format!(
                    "feature {index}: geometry coordinates are malformed"
                ))
            })?;
            let lon = position
                .first()
                .and_then(serde_json::Value::as_f64)
                .ok_or_else(|| {
                    GeoError::Validation(format!(
                        "feature {index}: longitude is missing or invalid"
                    ))
                })?;
            let lat = position
                .get(1)
                .and_then(serde_json::Value::as_f64)
                .ok_or_else(|| {
                    GeoError::Validation(format!("feature {index}: latitude is missing or invalid"))
                })?;
            if !lon.is_finite()
                || !lat.is_finite()
                || !(-180.0..=180.0).contains(&lon)
                || !(-90.0..=90.0).contains(&lat)
            {
                return Err(GeoError::Validation(format!(
                    "feature {index}: coordinates are outside valid longitude/latitude bounds"
                )));
            }
            Ok((lon, lat))
        };

        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| GeoError::Database(e.to_string()))?;
        tx.execute(
            &format!(
                "CREATE TABLE IF NOT EXISTS \"{table}\" (\
                id INTEGER PRIMARY KEY AUTOINCREMENT, \
                name TEXT, category TEXT, \
                lon REAL, lat REAL, area_ha REAL, props TEXT)"
            ),
            [],
        )
        .map_err(|e| GeoError::Database(e.to_string()))?;
        let mut stmt = tx
            .prepare(&format!(
                "INSERT INTO \"{table}\" (name, category, lon, lat, area_ha, props) VALUES (?,?,?,?,?,?)"
            ))
            .map_err(|e| GeoError::Database(e.to_string()))?;
        for (index, feature) in features.iter().enumerate() {
            let (lon, lat) = coordinates(index, feature)?;
            let props = &feature["properties"];
            let name = props["name"].as_str().unwrap_or("");
            let category = props["type"]
                .as_str()
                .or(props["class"].as_str())
                .unwrap_or("");
            let area = props["area_ha"].as_f64();
            let props_json = serde_json::to_string(props).map_err(GeoError::Serde)?;
            stmt.execute(params![name, category, lon, lat, area, props_json])
                .map_err(|e| GeoError::Database(e.to_string()))?;
        }
        drop(stmt);
        tx.commit().map_err(|e| GeoError::Database(e.to_string()))?;
        info!(count = features.len(), table = %table, "geo-io ingested features");
        Ok(features.len())
    }

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

    pub fn list_tables(&self) -> GeoResult<Vec<String>> {
        let rows =
            self.query_json("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")?;
        Ok(rows
            .iter()
            .filter_map(|r| r["name"].as_str().map(str::to_string))
            .collect())
    }

    pub fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }

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

// ---------------------------------------------------------------------------
// DuckDbAdapter
// ---------------------------------------------------------------------------

pub struct DuckDbAdapter {
    store: Option<DuckDbStore>,
    path: Option<String>,
}

impl DuckDbAdapter {
    pub fn in_memory() -> GeoResult<Self> {
        Ok(Self {
            store: Some(DuckDbStore::in_memory()?),
            path: None,
        })
    }

    pub fn open(path: &str) -> GeoResult<Self> {
        Ok(Self {
            store: Some(DuckDbStore::open(path)?),
            path: Some(path.to_string()),
        })
    }

    pub fn store(&self) -> Option<&DuckDbStore> {
        self.store.as_ref()
    }
}

impl Plugin for DuckDbAdapter {
    type Config = geo_core::plugin::EmptyConfig;
    fn new(_: Self::Config) -> Self {
        Self {
            store: None,
            path: None,
        }
    }
    fn name(&self) -> &str {
        "duckdb"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "Embedded database adapter (SQLite backend)"
    }
    fn category(&self) -> PluginCategory {
        PluginCategory::Adapter
    }
    fn is_healthy(&self) -> bool {
        self.store.is_some()
    }
}

impl ExternalAdapter for DuckDbAdapter {
    fn external_endpoint(&self) -> &str {
        self.path.as_deref().unwrap_or(":memory:")
    }
    async fn health_check(&self) -> GeoResult<bool> {
        match &self.store {
            Some(s) => {
                s.ping()?;
                Ok(true)
            }
            None => Ok(false),
        }
    }
    async fn external_version(&self) -> GeoResult<String> {
        Ok(rusqlite::version().to_string())
    }
    fn requires_network(&self) -> bool {
        false
    }
    async fn push(&self, _table: &str, _data: &[GeoFeature]) -> GeoResult<u64> {
        Ok(0)
    }
    async fn pull(&self, _query: &str) -> GeoResult<Vec<GeoFeature>> {
        Ok(vec![])
    }
    async fn execute(&self, cmd: &str, params: serde_json::Value) -> GeoResult<serde_json::Value> {
        let store = self
            .store
            .as_ref()
            .ok_or_else(|| geo_core::GeoError::Other("not connected".into()))?;
        match cmd {
            "query" => {
                let rows = store.query_json(params["sql"].as_str().unwrap_or("SELECT 1"))?;
                Ok(serde_json::json!({"rows": rows}))
            }
            "ingest" => {
                let fc = params["geojson"].as_str().unwrap_or("");
                let table = params["table"].as_str().unwrap_or("data");
                let n = store.ingest_geojson_raw(table, fc)?;
                Ok(serde_json::json!({"ingested": n}))
            }
            _ => Err(geo_core::GeoError::Unimplemented(format!(
                "unknown cmd: {cmd}"
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// Tool registration — DuckDB
// ---------------------------------------------------------------------------

use geo_registry::{register_plugin, PluginRegistry};
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "duckdb", "DuckDB embedded spatial database", PluginCategory::Store, [
        async "duckdb_query" => "Execute SQL on in-memory DuckDB, return JSON" ; serde_json::json!({"type":"object","properties":{"sql":{"type":"string"}},"required":["sql"]}) => |args| Box::pin(async move {
        let store = DuckDbStore::in_memory().map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
        Ok(serde_json::json!(store.query_json(args["sql"].as_str().unwrap_or("SELECT 1")).map_err(|e| geo_core::GeoError::Database(e.to_string()))?))
    }),
        async "duckdb_ingest_geojson" => "Ingest GeoJSON into DuckDB" ; serde_json::json!({"type":"object","properties":{"table":{"type":"string"},"geojson":{"type":"string"}},"required":["table","geojson"]}) => |args| Box::pin(async move {
        let store = DuckDbStore::in_memory().map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
        let t = args["table"].as_str().unwrap_or("features");
        let count = store.ingest_geojson_raw(t, args["geojson"].as_str().unwrap_or("{}")).map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
        Ok(serde_json::json!({"table":t,"ingested":count}))
    })]);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_in_memory() {
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
    }

    #[test]
    fn test_ingest_rejects_invalid_coordinates_without_partial_inserts() {
        let store = DuckDbStore::in_memory().unwrap();
        let geojson = r#"{
            "type": "FeatureCollection",
            "features": [
                {"type":"Feature","properties":{"name":"valid"},
                 "geometry":{"type":"Point","coordinates":[104.06,30.57]}},
                {"type":"Feature","properties":{"name":"invalid"},
                 "geometry":{"type":"Point","coordinates":[104.07]}}
            ]
        }"#;

        let err = store.ingest_geojson_raw("sites", geojson).unwrap_err();
        assert!(matches!(err, GeoError::Validation(_)));
        assert!(!store.list_tables().unwrap().contains(&"sites".to_string()));
    }

    #[test]
    fn test_ingest_rejects_unsupported_geometry() {
        let store = DuckDbStore::in_memory().unwrap();
        let geojson = r#"{
            "type": "FeatureCollection",
            "features": [{
                "type":"Feature",
                "properties":{"name":"unsupported"},
                "geometry":{"type":"GeometryCollection","geometries":[]}
            }]
        }"#;

        let err = store.ingest_geojson_raw("sites", geojson).unwrap_err();
        assert!(matches!(err, GeoError::Validation(_)));
        assert!(!store.list_tables().unwrap().contains(&"sites".to_string()));
    }

    #[test]
    fn test_adapter_create() {
        let a = DuckDbAdapter::in_memory().unwrap();
        assert!(!a.requires_network());
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
}
