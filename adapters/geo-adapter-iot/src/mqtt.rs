//! MQTT streaming ingest (feature `mqtt`).
//!
//! Subscribes to MQTT topics and forwards sensor readings
//! directly to TimescaleDB hypertables (bypassing postgis-gateway).

use geo_core::errors::{GeoError, GeoResult};
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use sqlx::PgPool;
use std::time::Duration;

use crate::validator;

/// MQTT ingestor that writes IoT readings to TimescaleDB.
pub struct MqttIngestor {
    pool: PgPool,
}

impl MqttIngestor {
    /// Create a new MQTT ingestor with a TimescaleDB connection.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Start listening on a broker + topic.
    ///
    /// Reads are batched (100 records or 1 second) and flushed
    /// to the `iot_readings` hypertable.
    pub async fn start(&self, broker: &str, port: u16, topic: &str) -> GeoResult<()> {
        let client_id = format!("geo-ingest-{}", uuid::Uuid::new_v4());
        let mut mqtt = MqttOptions::new(&client_id, broker, port);
        mqtt.set_keep_alive(Duration::from_secs(30));

        let (client, mut eventloop) = AsyncClient::new(mqtt, 100);
        client
            .subscribe(topic, QoS::AtMostOnce)
            .await
            .map_err(|e| GeoError::MessageQueue(e.to_string()))?;

        let mut batch: Vec<String> = Vec::with_capacity(100);
        let mut last_flush = tokio::time::Instant::now();

        tracing::info!("MQTT ingestor listening on {broker}:{port}/{topic}");

        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::Publish(p))) => {
                    let payload = String::from_utf8_lossy(&p.payload).to_string();
                    batch.push(payload);

                    if batch.len() >= 100 || last_flush.elapsed().as_secs() >= 1 {
                        self.flush_batch(&batch).await?;
                        batch.clear();
                        last_flush = tokio::time::Instant::now();
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("MQTT error: {e}");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn flush_batch(&self, records: &[String]) -> GeoResult<()> {
        // Parse each JSON record and insert
        for raw in records {
            let rec: serde_json::Value = serde_json::from_str(raw)
                .map_err(|e| GeoError::Serde(e))?;

            let device = rec["device_id"].as_str().unwrap_or("unknown");
            let sensor = rec["sensor_type"].as_str().unwrap_or("unknown");
            let value = rec["value"].as_f64().unwrap_or(0.0);

            // Validate
            validator::validate_iot_reading(sensor, value)?;

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

        tracing::debug!("Flushed {} IoT readings", records.len());
        Ok(())
    }
}
