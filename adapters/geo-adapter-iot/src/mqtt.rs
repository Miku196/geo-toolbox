use geo_core::errors::{GeoError, GeoResult};
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS, TlsConfiguration, Transport};
use sqlx::PgPool;
use std::path::PathBuf;
use std::time::Duration;

/// MQTT connection configuration.
#[derive(Debug, Clone)]
pub struct MqttConfig {
    pub broker: String,
    pub port: u16,
    pub topics: Vec<String>,
    pub qos: MqttQos,
    pub keep_alive_secs: u64,
    pub batch_size: usize,
    pub batch_timeout_ms: u64,
    pub client_id_prefix: String,

    // TLS options
    pub tls_ca_cert: Option<PathBuf>,
    pub tls_client_cert: Option<PathBuf>,
    pub tls_client_key: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MqttQos {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

impl MqttQos {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => MqttQos::AtLeastOnce,
            2 => MqttQos::ExactlyOnce,
            _ => MqttQos::AtMostOnce,
        }
    }

    pub fn to_rumqttc(&self) -> QoS {
        match self {
            MqttQos::AtMostOnce => QoS::AtMostOnce,
            MqttQos::AtLeastOnce => QoS::AtLeastOnce,
            MqttQos::ExactlyOnce => QoS::ExactlyOnce,
        }
    }
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            broker: "localhost".into(),
            port: 1883,
            topics: vec!["geo/readings".into()],
            qos: MqttQos::AtMostOnce,
            keep_alive_secs: 30,
            batch_size: 100,
            batch_timeout_ms: 1000,
            client_id_prefix: "geo-ingest".into(),
            tls_ca_cert: None,
            tls_client_cert: None,
            tls_client_key: None,
        }
    }
}

/// MQTT batch flush statistics.
#[derive(Debug, Clone, Default)]
pub struct MqttStats {
    pub total_messages: u64,
    pub total_flushes: u64,
    pub total_errors: u64,
}

/// MQTT ingestor that writes IoT readings to TimescaleDB.
pub struct MqttIngestor {
    pool: PgPool,
    config: MqttConfig,
    stats: std::sync::Arc<std::sync::Mutex<MqttStats>>,
}

impl MqttIngestor {
    /// Create a new MQTT ingestor with config.
    pub fn new(pool: PgPool, config: MqttConfig) -> Self {
        Self {
            pool,
            config,
            stats: std::sync::Arc::new(std::sync::Mutex::new(MqttStats::default())),
        }
    }

    /// Create with defaults (backward compat).
    pub fn new_default(pool: PgPool) -> Self {
        Self::new(pool, MqttConfig::default())
    }

    /// Get current stats.
    pub fn stats(&self) -> MqttStats {
        self.stats.lock().unwrap().clone()
    }

    fn build_mqtt_options(&self) -> GeoResult<MqttOptions> {
        let client_id = format!("{}-{}", self.config.client_id_prefix, uuid::Uuid::new_v4());
        let mut mqtt = MqttOptions::new(&client_id, &self.config.broker, self.config.port);
        mqtt.set_keep_alive(Duration::from_secs(self.config.keep_alive_secs));

        // TLS setup via mqtts:// and certificate config
        if let Some(ca_path) = &self.config.tls_ca_cert {
            let ca = std::fs::read(ca_path).map_err(|e| GeoError::Io(e))?;

            let client_auth = if let (Some(cert_path), Some(key_path)) =
                (&self.config.tls_client_cert, &self.config.tls_client_key)
            {
                let cert = std::fs::read(cert_path).map_err(|e| GeoError::Io(e))?;
                let key = std::fs::read(key_path).map_err(|e| GeoError::Io(e))?;
                Some((cert, key))
            } else {
                None
            };

            let tls_config = TlsConfiguration::Simple {
                ca,
                alpn: None,
                client_auth,
            };
            mqtt.set_transport(Transport::Tls(tls_config));
        }

        Ok(mqtt)
    }

    /// Start listening. Subscribes to all configured topics.
    pub async fn start(&self) -> GeoResult<MqttStats> {
        let mqtt_opts = self.build_mqtt_options()?;
        let (client, mut eventloop) = AsyncClient::new(mqtt_opts, 100);

        // Subscribe to all topics
        let qos = self.config.qos.to_rumqttc();
        for topic in &self.config.topics {
            client
                .subscribe(topic, qos)
                .await
                .map_err(|e| GeoError::MessageQueue(format!("subscribe {topic}: {e}")))?;
            tracing::info!("MQTT subscribed to {topic} (QoS {:?})", self.config.qos);
        }

        let mut batch: Vec<String> = Vec::with_capacity(self.config.batch_size);
        let mut last_flush = tokio::time::Instant::now();
        let batch_timeout = Duration::from_millis(self.config.batch_timeout_ms);

        tracing::info!(
            "MQTT ingestor started: {}:{}/{:?} (QoS {:?})",
            self.config.broker,
            self.config.port,
            self.config.topics,
            self.config.qos,
        );

        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::Publish(p))) => {
                    let payload = String::from_utf8_lossy(&p.payload).to_string();
                    batch.push(payload);

                    if batch.len() >= self.config.batch_size
                        || last_flush.elapsed() >= batch_timeout
                    {
                        self.flush_batch(&batch).await?;
                        batch.clear();
                        last_flush = tokio::time::Instant::now();
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("MQTT error: {e}");
                    if let Ok(mut s) = self.stats.lock() {
                        s.total_errors += 1;
                    }
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// Replay records from an external file (CSV or JSONL).
    ///
    /// Each line/record should match the IoT JSON schema.
    /// Returns number of successfully ingested records.
    pub async fn replay_file(&self, path: &str) -> GeoResult<u64> {
        let content = std::fs::read_to_string(path).map_err(|e| GeoError::Io(e))?;

        if path.ends_with(".csv") {
            let mut reader = csv::Reader::from_reader(content.as_bytes());
            let mut count = 0u64;
            let mut batch: Vec<String> = Vec::with_capacity(self.config.batch_size);

            for result in reader.records() {
                let record = result.map_err(|e| GeoError::Io(e.into()))?;
                // Convert CSV row to JSON string
                let mut map = serde_json::Map::new();
                for (i, field) in record.iter().enumerate() {
                    let key = match i {
                        0 => "device_id",
                        1 => "sensor_type",
                        2 => "value",
                        3 => "lng",
                        4 => "lat",
                        _ => break,
                    };
                    if let Ok(v) = field.parse::<f64>() {
                        map.insert(
                            key.to_string(),
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(v).unwrap_or_default(),
                            ),
                        );
                    } else {
                        map.insert(
                            key.to_string(),
                            serde_json::Value::String(field.to_string()),
                        );
                    }
                }
                batch.push(serde_json::Value::Object(map).to_string());
                count += 1;

                if batch.len() >= self.config.batch_size {
                    self.flush_batch(&batch).await?;
                    batch.clear();
                }
            }
            if !batch.is_empty() {
                self.flush_batch(&batch).await?;
            }
            Ok(count)
        } else {
            // JSONL — one JSON object per line
            let mut count = 0u64;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                // Validate JSON
                let _: serde_json::Value =
                    serde_json::from_str(trimmed).map_err(|e| GeoError::Serde(e))?;
                // Ingest directly
                let records = vec![trimmed.to_string()];
                self.flush_batch(&records).await?;
                count += 1;

                if let Ok(mut s) = self.stats.lock() {
                    s.total_messages += 1;
                }
            }
            Ok(count)
        }
    }

    async fn flush_batch(&self, records: &[String]) -> GeoResult<()> {
        for raw in records {
            let rec: serde_json::Value =
                serde_json::from_str(raw).map_err(|e| GeoError::Serde(e))?;

            let device = rec["device_id"].as_str().unwrap_or("unknown");
            let sensor = rec["sensor_type"].as_str().unwrap_or("unknown");
            let value = rec["value"].as_f64().unwrap_or(0.0);

            let lon = rec["lng"].as_f64();
            let lat = rec["lat"].as_f64();

            if let (Some(lon), Some(lat)) = (lon, lat) {
                sqlx::query(
                    "INSERT INTO iot_readings (time, device_id, sensor_type, value, geom)
                     VALUES (NOW(), $1, $2, $3, ST_SetSRID(ST_MakePoint($4, $5), 4326))",
                )
                .bind(device)
                .bind(sensor)
                .bind(value)
                .bind(lon)
                .bind(lat)
                .execute(&self.pool)
                .await
                .map_err(|e| GeoError::Database(e.to_string()))?;
            } else {
                sqlx::query(
                    "INSERT INTO iot_readings (time, device_id, sensor_type, value)
                     VALUES (NOW(), $1, $2, $3)",
                )
                .bind(device)
                .bind(sensor)
                .bind(value)
                .execute(&self.pool)
                .await
                .map_err(|e| GeoError::Database(e.to_string()))?;
            }
        }

        if let Ok(mut s) = self.stats.lock() {
            s.total_messages += records.len() as u64;
            s.total_flushes += 1;
        }

        tracing::debug!("Flushed {} IoT readings", records.len());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> MqttConfig {
        MqttConfig {
            broker: "test.mosquitto.org".into(),
            port: 1883,
            topics: vec!["geo/test".into(), "geo/#".into()],
            qos: MqttQos::AtLeastOnce,
            batch_size: 10,
            batch_timeout_ms: 500,
            ..Default::default()
        }
    }

    #[test]
    fn test_mqtt_config_default() {
        let cfg = MqttConfig::default();
        assert_eq!(cfg.broker, "localhost");
        assert_eq!(cfg.port, 1883);
        assert_eq!(cfg.topics, vec!["geo/readings"]);
        assert_eq!(cfg.qos, MqttQos::AtMostOnce);
        assert_eq!(cfg.batch_size, 100);
    }

    #[test]
    fn test_mqtt_config_custom() {
        let cfg = test_config();
        assert_eq!(cfg.qos, MqttQos::AtLeastOnce);
        assert_eq!(cfg.topics.len(), 2);
        assert_eq!(cfg.batch_size, 10);
    }

    #[test]
    fn test_qos_conversion() {
        assert_eq!(MqttQos::from_u8(0), MqttQos::AtMostOnce);
        assert_eq!(MqttQos::from_u8(1), MqttQos::AtLeastOnce);
        assert_eq!(MqttQos::from_u8(2), MqttQos::ExactlyOnce);
        assert_eq!(MqttQos::from_u8(255), MqttQos::AtMostOnce);

        assert_eq!(MqttQos::AtMostOnce.to_rumqttc(), QoS::AtMostOnce);
        assert_eq!(MqttQos::AtLeastOnce.to_rumqttc(), QoS::AtLeastOnce);
        assert_eq!(MqttQos::ExactlyOnce.to_rumqttc(), QoS::ExactlyOnce);
    }

    #[test]
    fn test_mqtt_stats() {
        let mut stats = MqttStats::default();
        assert_eq!(stats.total_messages, 0);
        stats.total_messages = 42;
        stats.total_flushes = 5;
        assert_eq!(stats.total_messages, 42);
        assert_eq!(stats.total_flushes, 5);
    }

    #[test]
    fn test_config_client_id_prefix() {
        let cfg = MqttConfig {
            client_id_prefix: "custom-prefix".into(),
            ..Default::default()
        };
        assert_eq!(cfg.client_id_prefix, "custom-prefix");
    }

    #[test]
    fn test_tls_options() {
        let cfg = MqttConfig {
            tls_ca_cert: Some(PathBuf::from("/etc/ssl/certs/ca.pem")),
            ..Default::default()
        };
        assert!(cfg.tls_ca_cert.is_some());
        assert!(cfg.tls_client_cert.is_none());
    }
}
