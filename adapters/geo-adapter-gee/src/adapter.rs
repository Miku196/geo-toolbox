use geo_core::errors::GeoResult;
use geo_core::plugin::{ExternalAdapter, Plugin, PluginCategory, GeoFeature};

pub struct GeeAdapter { endpoint: String }
impl GeeAdapter {
    pub fn new(endpoint: &str) -> Self { Self { endpoint: endpoint.to_string() } }
}
impl Plugin for GeeAdapter {
    fn name(&self) -> &str { "gee" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
    fn description(&self) -> &str { "GEE task dispatcher via message queue" }
    fn category(&self) -> PluginCategory { PluginCategory::Adapter }
}
impl ExternalAdapter for GeeAdapter {
    fn external_endpoint(&self) -> &str { &self.endpoint }
    async fn health_check(&self) -> GeoResult<bool> { Ok(true) }
    async fn external_version(&self) -> GeoResult<String> { Ok("GEE Python worker".into()) }
    fn requires_network(&self) -> bool { true }
    async fn push(&self, _table: &str, _data: &[GeoFeature]) -> GeoResult<u64> { Ok(0) }
    async fn pull(&self, _query: &str) -> GeoResult<Vec<GeoFeature>> { Ok(vec![]) }
    async fn execute(&self, _command: &str, _params: serde_json::Value) -> GeoResult<serde_json::Value> { Ok(serde_json::json!({"status":"ok"})) }
}

#[test]
fn test_gee_adapter() {
    let a = GeeAdapter::new("nats://localhost:4222");
    assert_eq!(a.name(), "gee");
    assert_eq!(a.category(), PluginCategory::Adapter);
}
