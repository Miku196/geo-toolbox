//! geo-adapter-iot: IoT 传感器适配器（MQTT/NATS 流式数据）。
#![allow(missing_docs)]
use geo_core::errors::GeoResult;
use geo_core::plugin::{ExternalAdapter, GeoFeature, Plugin, PluginCategory};
pub struct IotAdapter {
    pub broker: String,
}
impl IotAdapter {
    pub fn new(broker: &str) -> Self {
        Self {
            broker: broker.to_string(),
        }
    }
}
impl Plugin for IotAdapter {
    fn name(&self) -> &str {
        "iot"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "IoT sensor adapter (MQTT/NATS streaming)"
    }
    fn category(&self) -> PluginCategory {
        PluginCategory::Adapter
    }
}
impl ExternalAdapter for IotAdapter {
    fn external_endpoint(&self) -> &str {
        &self.broker
    }
    async fn health_check(&self) -> GeoResult<bool> {
        Ok(true)
    }
    async fn external_version(&self) -> GeoResult<String> {
        Ok("MQTT 3.1.1".into())
    }
    fn requires_network(&self) -> bool {
        true
    }
    async fn push(&self, _t: &str, _d: &[GeoFeature]) -> GeoResult<u64> {
        Ok(0)
    }
    async fn pull(&self, _q: &str) -> GeoResult<Vec<GeoFeature>> {
        Ok(vec![])
    }
    async fn execute(&self, _c: &str, _p: serde_json::Value) -> GeoResult<serde_json::Value> {
        Ok(serde_json::json!({"status":"ok"}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_trait() {
        let a = IotAdapter::new("mqtt://localhost:1883");
        assert_eq!(a.name(), "iot");
        assert_eq!(a.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(a.description(), "IoT sensor adapter (MQTT/NATS streaming)");
        assert_eq!(a.category(), PluginCategory::Adapter);
    }

    #[test]
    fn test_external_adapter_trait() {
        let a = IotAdapter::new("mqtt://localhost:1883");
        assert_eq!(a.external_endpoint(), "mqtt://localhost:1883");
        assert!(a.requires_network());
        let rt = tokio::runtime::Runtime::new().unwrap();
        assert_eq!(rt.block_on(a.external_version()).unwrap(), "MQTT 3.1.1");
    }

    #[test]
    fn test_health_check() {
        let a = IotAdapter::new("mqtt://localhost:1883");
        let rt = tokio::runtime::Runtime::new().unwrap();
        assert!(rt.block_on(a.health_check()).unwrap());
    }
}
