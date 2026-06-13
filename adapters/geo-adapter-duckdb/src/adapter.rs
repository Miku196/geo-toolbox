//! DuckDB 适配器 (SQLite 后端) — ExternalAdapter trait 实现。

use crate::store::DuckDbStore;
use geo_core::errors::GeoResult;
use geo_core::plugin::{ExternalAdapter, GeoFeature, Plugin, PluginCategory};

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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_create() {
        let a = DuckDbAdapter::in_memory().unwrap();
        assert!(!a.requires_network());
    }
}
