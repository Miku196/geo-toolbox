//! geo-adapter-iot: IoT 传感器适配器（MQTT/NATS 流式数据）。
#![allow(missing_docs)]
use geo_core::errors::GeoResult;
use geo_core::plugin::{ExternalAdapter, Plugin, PluginCategory, GeoFeature};
pub struct IotAdapter { pub broker: String }
impl IotAdapter { pub fn new(broker: &str) -> Self { Self { broker: broker.to_string() } } }
impl Plugin for IotAdapter {
    fn name(&self) -> &str { "iot" } fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
    fn description(&self) -> &str { "IoT sensor adapter (MQTT/NATS streaming)" }
    fn category(&self) -> PluginCategory { PluginCategory::Adapter }
}
impl ExternalAdapter for IotAdapter {
    fn external_endpoint(&self) -> &str { &self.broker }
    async fn health_check(&self) -> GeoResult<bool> { Ok(true) }
    async fn external_version(&self) -> GeoResult<String> { Ok("MQTT 3.1.1".into()) }
    fn requires_network(&self) -> bool { true }
    async fn push(&self, _t: &str, _d: &[GeoFeature]) -> GeoResult<u64> { Ok(0) }
    async fn pull(&self, _q: &str) -> GeoResult<Vec<GeoFeature>> { Ok(vec![]) }
    async fn execute(&self, _c: &str, _p: serde_json::Value) -> GeoResult<serde_json::Value> { Ok(serde_json::json!({"status":"ok"})) }
}
#[cfg(test)] #[test] fn test_iot() { let a = IotAdapter::new("mqtt://localhost:1883"); assert_eq!(a.name(), "iot"); }
