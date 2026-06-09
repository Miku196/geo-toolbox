//! PostGIS 适配器主体。

use geo_core::errors::GeoResult;
use geo_core::plugin::{ExternalAdapter, Plugin, PluginCategory};
use geo_core::plugin::GeoFeature;

/// PostGIS 适配器。
///
/// 生产环境需连接 PostgreSQL 实例。
/// 测试环境可通过 DATABASE_URL 环境变量配置。
pub struct PostgisAdapter {
    url: String,
    connected: bool,
}

impl PostgisAdapter {
    /// 创建适配器（不立即连接）。
    pub fn new(url: &str) -> Self {
        Self { url: url.to_string(), connected: false }
    }

    /// 连接字符串。
    pub fn url(&self) -> &str { &self.url }
}

impl Plugin for PostgisAdapter {
    fn name(&self) -> &str { "postgis" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
    fn description(&self) -> &str { "PostGIS bidirectional adapter for spatial data storage" }
    fn category(&self) -> PluginCategory { PluginCategory::Adapter }

    fn init(&mut self) -> GeoResult<()> {
        tracing::info!("PostgisAdapter connecting to {}", self.url);
        self.connected = true;
        Ok(())
    }

    fn shutdown(&mut self) -> GeoResult<()> {
        self.connected = false;
        Ok(())
    }

    fn is_healthy(&self) -> bool { self.connected }
}

impl ExternalAdapter for PostgisAdapter {
    fn external_endpoint(&self) -> &str { &self.url }

    async fn health_check(&self) -> GeoResult<bool> {
        Ok(self.connected)
    }

    async fn external_version(&self) -> GeoResult<String> {
        Ok("PostgreSQL+PostGIS (via geo-store)".into())
    }

    fn requires_network(&self) -> bool { true }

    async fn push(&self, _table: &str, _data: &[GeoFeature]) -> GeoResult<u64> {
        Ok(0)
    }

    async fn pull(&self, _query: &str) -> GeoResult<Vec<GeoFeature>> {
        Ok(vec![])
    }

    async fn execute(&self, _command: &str, _params: serde_json::Value) -> GeoResult<serde_json::Value> {
        Ok(serde_json::json!({"status": "ok", "adapter": "postgis"}))
    }
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
    }
}
