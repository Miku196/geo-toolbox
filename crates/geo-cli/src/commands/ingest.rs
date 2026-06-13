//! Ingest subcommand handler.

use super::super::IngestAction;
use geo_registry::PluginRegistry;
use serde_json::json;

/// Handle `ingest camofox | nmea | mqtt`.
pub async fn handle(
    registry: &PluginRegistry,
    action: IngestAction,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        IngestAction::Camofox { file } => {
            let result = registry
                .dispatch("ingest_camofox", json!({"file": file}))
                .await?;
            println!(
                "CamoFox ingest: {} accepted, {} rejected",
                result["accepted"], result["rejected"]
            );
            println!("File: {}", result["file"]);
        }
        IngestAction::Nmea { file } => {
            let result = registry
                .dispatch("ingest_nmea", json!({"file": file}))
                .await?;
            let fixes = result["total_fixes"].as_u64().unwrap_or(0);
            println!("NMEA parsed: {fixes} fixes");
            if let Some(records) = result["records"].as_array() {
                for r in records {
                    let t = r["type"].as_str().unwrap_or("?");
                    let lat = r["lat"].as_f64().unwrap_or(0.0);
                    let lng = r["lng"].as_f64().unwrap_or(0.0);
                    println!("  {t} @ ({lng:.6}, {lat:.6})");
                }
            }
        }
        #[cfg(feature = "mqtt")]
        IngestAction::Mqtt {
            broker,
            port,
            topic,
        } => {
            let ts_url = std::env::var("TIMESCALE_URL")
                .unwrap_or_else(|_| std::env::var("DATABASE_URL").unwrap_or_default());
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(5)
                .connect(&ts_url)
                .await?;
            println!("MQTT ingestor connecting to {broker}:{port}/{topic} ...");
            let ingestor = geo_adapter_iot::mqtt::MqttIngestor::new(pool);
            ingestor.start(&broker, port, &topic).await?;
        }
    }
    Ok(())
}
