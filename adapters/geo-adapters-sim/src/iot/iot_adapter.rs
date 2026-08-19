use geo_core::errors::{GeoError, GeoResult};
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
    type Config = geo_core::plugin::EmptyConfig;

    fn new(_config: Self::Config) -> Self {
        Self {
            broker: String::new(),
        }
    }

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
        // IotAdapter only stores a broker string and never opens an MQTT connection,
        // so it is genuinely unavailable until the MQTT backend is wired in.
        // Report Ok(false) instead of a fake Ok(true).
        Ok(false)
    }

    async fn external_version(&self) -> GeoResult<String> {
        // No real MQTT broker handshake is performed, so the version must not be faked.
        Err(GeoError::Unimplemented(
            "iot IotAdapter: external_version not queried — no MQTT connection established (see iot_mqtt::MqttIngestor)"
                .into(),
        ))
    }

    fn requires_network(&self) -> bool {
        true
    }

    async fn push(&self, _t: &str, _d: &[GeoFeature]) -> GeoResult<u64> {
        Err(GeoError::Unimplemented(
            "iot IotAdapter: push not implemented — MQTT publish not wired (see iot_mqtt::MqttIngestor)"
                .into(),
        ))
    }

    async fn pull(&self, _q: &str) -> GeoResult<Vec<GeoFeature>> {
        Err(GeoError::Unimplemented(
            "iot IotAdapter: pull not implemented — no MQTT subscribe/read wiring".into(),
        ))
    }

    async fn execute(&self, _c: &str, _p: serde_json::Value) -> GeoResult<serde_json::Value> {
        // Explicit failure instead of a fake {"status":"ok"}: the MQTT backend is
        // not wired into IotAdapter yet.
        Err(GeoError::Unimplemented(
            "iot IotAdapter: execute not implemented yet (MQTT publish not wired via iot_mqtt::MqttIngestor)"
                .into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo_core::errors::GeoError;

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
        // No MQTT broker is connected, so the version must not be faked as "MQTT 3.1.1".
        assert!(matches!(
            rt.block_on(a.external_version()),
            Err(GeoError::Unimplemented(_))
        ));
    }

    #[test]
    fn test_health_check_reports_unavailable() {
        let a = IotAdapter::new("mqtt://localhost:1883");
        let rt = tokio::runtime::Runtime::new().unwrap();
        // IotAdapter only stores a broker string and never connects, so health
        // must report unavailable instead of a fake Ok(true).
        assert_eq!(rt.block_on(a.health_check()).unwrap(), false);
    }

    #[test]
    fn test_execute_does_not_fake_success() {
        let a = IotAdapter::new("mqtt://localhost:1883");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt
            .block_on(a.execute("publish", serde_json::json!({"topic": "geo/x"})))
            .expect_err("execute must not fake success");
        assert!(matches!(err, GeoError::Unimplemented(_)), "got {err:?}");
    }

    #[test]
    fn test_push_pull_not_implemented() {
        let a = IotAdapter::new("mqtt://localhost:1883");
        let rt = tokio::runtime::Runtime::new().unwrap();
        assert!(matches!(
            rt.block_on(a.push("t", &[])),
            Err(GeoError::Unimplemented(_))
        ));
        assert!(matches!(
            rt.block_on(a.pull("q")),
            Err(GeoError::Unimplemented(_))
        ));
    }
}
