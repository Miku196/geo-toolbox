//! PostGIS 适配器主体。
//!
//! 实现 ExternalAdapter trait：push 写入空间数据、pull 查询空间数据、
//! execute 执行任意 SQL。内部复用 PostgisStore 的连接池。

use crate::PostgisStore;
use geo_core::errors::{GeoError, GeoResult};
use geo_core::plugin::GeoFeature;
use geo_core::plugin::{ExternalAdapter, Plugin, PluginCategory};
use sqlx::Column;

/// PostGIS 适配器。
///
/// 生产环境需连接 PostgreSQL 实例。
/// 测试环境可通过 DATABASE_URL 环境变量配置。
///
/// # 安全
///
/// - `execute` 只允许 SELECT 查询，拒绝 DDL/DML
/// - `push` 使用参数化查询防止 SQL 注入
/// - 生产环境应使用只读数据库用户
pub struct PostgisAdapter {
    url: String,
    store: Option<PostgisStore>,
}

impl PostgisAdapter {
    /// 创建适配器（不立即连接）。
    /// 调用 `init()` 后建立数据库连接。
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            store: None,
        }
    }

    /// 连接字符串。
    pub fn url(&self) -> &str {
        &self.url
    }

    /// 获取内部存储句柄（init 后可用）。
    pub fn store(&self) -> Option<&PostgisStore> {
        self.store.as_ref()
    }
}

impl Plugin for PostgisAdapter {
    type Config = geo_core::plugin::EmptyConfig;
    fn new(_config: Self::Config) -> Self {
        Self {
            url: String::new(),
            store: None,
        }
    }
    fn name(&self) -> &str {
        "postgis"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "PostGIS bidirectional adapter for spatial data storage"
    }
    fn category(&self) -> PluginCategory {
        PluginCategory::Adapter
    }

    fn init(&mut self) -> GeoResult<()> {
        if self.url.is_empty() {
            tracing::warn!("PostgisAdapter: DATABASE_URL is empty, skipping connection");
            return Ok(());
        }

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| GeoError::Other(format!("tokio runtime: {e}")))?;

        let store = rt.block_on(PostgisStore::connect(&self.url))?;
        rt.block_on(store.ping())?;
        self.store = Some(store);
        tracing::info!("PostgisAdapter connected to {}", self.url);
        Ok(())
    }

    fn shutdown(&mut self) -> GeoResult<()> {
        if let Some(store) = self.store.take() {
            drop(store);
        }
        tracing::info!("PostgisAdapter shut down");
        Ok(())
    }

    fn is_healthy(&self) -> bool {
        self.store
            .as_ref()
            .map(|s| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("tokio runtime for is_healthy");
                rt.block_on(s.ping()).is_ok()
            })
            .unwrap_or(false)
    }
}

impl ExternalAdapter for PostgisAdapter {
    fn external_endpoint(&self) -> &str {
        &self.url
    }

    async fn health_check(&self) -> GeoResult<bool> {
        if let Some(ref store) = self.store {
            match store.ping().await {
                Ok(()) => return Ok(true),
                Err(e) => {
                    tracing::warn!("PostGIS ping via store failed: {e}");
                }
            }
        }

        if self.url.is_empty() {
            return Ok(false);
        }

        match sqlx::PgPool::connect(&self.url).await {
            Ok(pool) => {
                let result = sqlx::query_scalar::<_, i32>("SELECT 1")
                    .fetch_one(&pool)
                    .await;
                pool.close().await;
                match result {
                    Ok(v) => Ok(v == 1),
                    Err(e) => {
                        tracing::warn!("PostGIS health check query failed: {e}");
                        Ok(false)
                    }
                }
            }
            Err(e) => {
                tracing::warn!("PostGIS health check connection failed: {e}");
                Ok(false)
            }
        }
    }

    async fn external_version(&self) -> GeoResult<String> {
        if let Some(ref store) = self.store {
            return store.check_postgis().await;
        }
        Ok("PostgreSQL+PostGIS (via geo-store)".into())
    }

    fn requires_network(&self) -> bool {
        true
    }

    /// 推送 GeoFeature 数据到指定表。
    ///
    /// 使用 `ST_GeomFromGeoJSON` 将 GeoJSON 几何转为 PostGIS 几何类型。
    /// 自动建表（如不存在），批量 INSERT 写入。
    async fn push(&self, table: &str, data: &[GeoFeature]) -> GeoResult<u64> {
        if data.is_empty() {
            return Ok(0);
        }

        geo_core::errors::validate_sql_identifier(table)?;

        let store = self
            .store
            .as_ref()
            .ok_or_else(|| GeoError::Other("PostgisAdapter not initialized".into()))?;

        push_features(store, table, data).await
    }

    /// 从 PostGIS 查询数据，返回 GeoFeature 列表。
    ///
    /// 查询结果会尝试提取 geometry 列（geom/geometry/geojson 名），
    /// 其余列放入 properties。
    async fn pull(&self, query: &str) -> GeoResult<Vec<GeoFeature>> {
        let store = self
            .store
            .as_ref()
            .ok_or_else(|| GeoError::Other("PostgisAdapter not initialized".into()))?;

        // Security: validate SELECT-only SQL
        geo_core::errors::validate_select_sql(query)?;

        let rows = sqlx::query(query)
            .fetch_all(store.pool())
            .await
            .map_err(|e| GeoError::Database(e.to_string()))?;

        features_from_rows(&rows)
    }

    /// 执行通用 SQL 并返回 JSON 结果。
    ///
    /// 安全：仅允许 SELECT 查询，拒绝 INSERT/UPDATE/DELETE/DROP/TRUNCATE/ALTER。
    /// 生产环境建议使用只读数据库用户。
    async fn execute(
        &self,
        command: &str,
        params: serde_json::Value,
    ) -> GeoResult<serde_json::Value> {
        let store = self
            .store
            .as_ref()
            .ok_or_else(|| GeoError::Other("PostgisAdapter not initialized".into()))?;

        let sql = build_sql(command, &params)?;
        geo_core::errors::validate_select_sql(&sql)?;

        let rows = sqlx::query(&sql)
            .fetch_all(store.pool())
            .await
            .map_err(|e| GeoError::Database(e.to_string()))?;

        let result: Vec<serde_json::Value> = rows
            .iter()
            .map(|row| {
                use sqlx::Row;
                let mut map = serde_json::Map::new();
                for (i, col) in row.columns().iter().enumerate() {
                    let val = try_get_value(row, i);
                    map.insert(col.name().to_string(), val);
                }
                serde_json::Value::Object(map)
            })
            .collect();

        Ok(serde_json::json!({
            "adapter": "postgis",
            "rows": result,
            "count": result.len()
        }))
    }
}

// ── Internal helpers ──

/// Ensure a spatial table exists with the standard schema.
async fn ensure_table(store: &PostgisStore, table: &str) -> GeoResult<()> {
    geo_core::errors::validate_sql_identifier(table)?;
    let create_sql = format!(
        "CREATE TABLE IF NOT EXISTS {table} (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            aoi_id UUID,
            source TEXT NOT NULL DEFAULT 'push',
            geom geometry(Geometry, 4326),
            properties JSONB DEFAULT '{{}}'::jsonb,
            ingested_at TIMESTAMPTZ DEFAULT now()
        )"
    );
    sqlx::query(&create_sql)
        .execute(store.pool())
        .await
        .map_err(|e| GeoError::Database(format!("create table {table}: {e}")))?;
    Ok(())
}

/// Push features into a table, one-by-one with parameterized INSERT.
async fn push_features(store: &PostgisStore, table: &str, data: &[GeoFeature]) -> GeoResult<u64> {
    ensure_table(store, table).await?;

    let mut count: u64 = 0;

    for feature in data {
        let source = "push";

        let geom_str = feature.geometry.as_str().unwrap_or_default();
        let has_geometry = !geom_str.is_empty() && geom_str != "{}" && geom_str != "null";
        let result = if !has_geometry {
            // No geometry — insert only properties
            sqlx::query(&format!(
                "INSERT INTO {table} (source, properties) VALUES ($1, $2) RETURNING id"
            ))
            .bind(source)
            .bind(&feature.properties)
            .fetch_optional(store.pool())
            .await
        } else {
            // With geometry — use ST_GeomFromGeoJSON
            sqlx::query(&format!(
                "INSERT INTO {table} (source, geom, properties) VALUES ($1, ST_GeomFromGeoJSON($2), $3) RETURNING id"
            ))
            .bind(source)
            .bind(&feature.geometry)
            .bind(&feature.properties)
            .fetch_optional(store.pool())
            .await
        };

        match result {
            Ok(_) => count += 1,
            Err(e) => {
                tracing::warn!("push_features: failed to insert feature into {table}: {e}");
            }
        }
    }

    tracing::info!(
        "push_features: inserted {count}/{len} features into {table}",
        count = count,
        len = data.len()
    );
    Ok(count)
}

//// Check if a GeoJSON geometry value is effectively empty.
fn is_geom_empty(geom: &serde_json::Value) -> bool {
    match geom {
        serde_json::Value::Null => true,
        serde_json::Value::String(s) => s.is_empty() || s == "{}",
        serde_json::Value::Object(o) => o.is_empty(),
        serde_json::Value::Array(a) => a.is_empty(),
        _ => false,
    }
}

/// Convert a geometry string (GeoJSON) to serde_json::Value.
fn geom_str_to_value(s: &str) -> serde_json::Value {
    if s.is_empty() {
        serde_json::Value::Null
    } else if let Ok(v) = serde_json::from_str::<serde_json::Value>(s) {
        v
    } else {
        serde_json::Value::String(s.to_string())
    }
}

// Convert sqlx rows into GeoFeature list.
fn features_from_rows(rows: &[sqlx::postgres::PgRow]) -> GeoResult<Vec<GeoFeature>> {
    use sqlx::Row;

    let mut features = Vec::with_capacity(rows.len());

    for row in rows {
        let mut properties = serde_json::Map::new();
        let mut geometry = String::new();

        for (i, col) in row.columns().iter().enumerate() {
            let col_name = col.name().to_string();

            if col_name == "geom" || col_name == "geometry" || col_name == "geojson" {
                geometry = row.try_get::<String, _>(i).unwrap_or_default();
            } else {
                let val = try_get_value(row, i);
                properties.insert(col_name, val);
            }
        }

        features.push(GeoFeature {
            id: uuid::Uuid::new_v4().to_string(),
            geometry: geom_str_to_value(&geometry),
            properties: serde_json::Value::Object(properties),
        });
    }

    Ok(features)
}

/// Try to extract a value from a row column as a JSON-compatible type.
fn try_get_value(row: &sqlx::postgres::PgRow, index: usize) -> serde_json::Value {
    use sqlx::Row;

    if let Ok(v) = row.try_get::<i64, _>(index) {
        return serde_json::Value::Number(v.into());
    }
    if let Ok(v) = row.try_get::<i32, _>(index) {
        return serde_json::Value::Number(v.into());
    }
    if let Ok(v) = row.try_get::<f64, _>(index) {
        if v.is_nan() || v.is_infinite() {
            return serde_json::Value::Null;
        }
        return serde_json::json!(v);
    }
    if let Ok(v) = row.try_get::<String, _>(index) {
        return serde_json::Value::String(v);
    }
    if let Ok(v) = row.try_get::<bool, _>(index) {
        return serde_json::Value::Bool(v);
    }
    if let Ok(v) = row.try_get::<serde_json::Value, _>(index) {
        return v;
    }
    serde_json::Value::Null
}

/// Build SQL from command + params.
fn build_sql(command: &str, params: &serde_json::Value) -> GeoResult<String> {
    if let Some(sql) = params.get("sql").and_then(|v| v.as_str()) {
        if !sql.is_empty() {
            return Ok(sql.to_string());
        }
    }
    if !command.is_empty() {
        return Ok(command.to_string());
    }
    Err(GeoError::Validation(
        "No SQL provided in command or params.sql".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let adapter = PostgisAdapter::new("postgres://localhost/test");
        assert_eq!(adapter.name(), "postgis");
        assert_eq!(adapter.category(), PluginCategory::Adapter);
        assert!(adapter.requires_network());
        assert_eq!(adapter.url(), "postgres://localhost/test");
        assert!(adapter.store().is_none());
    }

    #[test]
    fn test_adapter_empty_url() {
        let adapter = PostgisAdapter::new("");
        assert_eq!(adapter.external_endpoint(), "");
        assert!(!adapter.is_healthy());
    }

    #[test]
    fn test_push_empty_data() {
        let adapter = PostgisAdapter::new("postgres://localhost/test");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(adapter.push("test_table", &[]));
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_push_no_store() {
        let adapter = PostgisAdapter::new("postgres://localhost/test");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let feature = GeoFeature {
            id: "f1".into(),
            geometry: serde_json::json!({"type":"Point","coordinates":[120.0,30.0]}),
            properties: serde_json::json!({"name": "test"}),
        };
        let result = rt.block_on(adapter.push("test_table", &[feature]));
        assert!(result.is_err());
    }

    #[test]
    fn test_is_geom_empty() {
        assert!(is_geom_empty(&serde_json::Value::Null));
        assert!(is_geom_empty(&serde_json::json!("")));
        assert!(is_geom_empty(&serde_json::json!("{}")));
        assert!(!is_geom_empty(&serde_json::json!({"type":"Point"})));
    }

    #[test]
    fn test_geom_str_to_value() {
        let v = geom_str_to_value("");
        assert!(v.is_null());
        let v = geom_str_to_value(r#"{"type":"Point"}"#);
        assert!(v.is_object());
    }

    #[test]
    fn test_build_sql_from_params() {
        let sql = build_sql("", &serde_json::json!({"sql": "SELECT 1"})).unwrap();
        assert_eq!(sql, "SELECT 1");
    }

    #[test]
    fn test_build_sql_from_command() {
        let sql = build_sql("SELECT version()", &serde_json::json!({})).unwrap();
        assert_eq!(sql, "SELECT version()");
    }

    #[test]
    fn test_build_sql_empty() {
        let result = build_sql("", &serde_json::json!({}));
        assert!(result.is_err());
    }
}
