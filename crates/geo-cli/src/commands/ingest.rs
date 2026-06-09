//! Ingest subcommand handler.

use super::super::IngestAction;

/// Handle `ingest camofox | nmea | mqtt`.
pub async fn handle(action: IngestAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        IngestAction::Camofox { file } => {
            let content = tokio::fs::read_to_string(&file).await?;

            let (rows, result) = geo_io::camofox::parse_camofox_file(&content, &file)?;

            println!("CamoFox ingest: {} accepted, {} rejected",
                result.accepted, result.rejected);

            if !result.errors.is_empty() {
                println!("\nRejected:");
                for e in &result.errors {
                    println!("  - {e}");
                }
            }

            // Write to PostGIS using simple INSERT
            if let Ok(db_url) = std::env::var("DATABASE_URL") {
                let store = geo_adapter_postgis::PostgisStore::connect(&db_url).await?;
                let mut written = 0u64;
                for row in &rows {
                    let props: serde_json::Value = serde_json::from_str(&row.properties)?;
                    sqlx::query(
                        "INSERT INTO spatial_assets (source, properties) VALUES ($1, $2)"
                    )
                    .bind(&row.source)
                    .bind(&props)
                    .execute(store.pool())
                    .await?;
                    written += 1;
                }
                println!("\nWritten {written} rows to spatial_assets");
            } else {
                println!("\n(skipped DB write — set DATABASE_URL to write to PostGIS)");
            }

            // Show first 3 records
            println!("\nSample records:");
            for (i, row) in rows.iter().take(3).enumerate() {
                let props: serde_json::Value = serde_json::from_str(&row.properties)?;
                let name = props["name"].as_str().unwrap_or("?");
                let lat = props["lat"].as_f64().unwrap_or(0.0);
                let lng = props["lon"].as_f64().unwrap_or(0.0);
                let cat = props["type"].as_str().unwrap_or("?");
                println!("  {}. {name} ({cat}) @ ({lng}, {lat})", i + 1);
            }
        }

        IngestAction::Nmea { file } => {
            let content = tokio::fs::read_to_string(&file).await?;
            let mut fixes = 0u32;
            let mut errs = 0u32;

            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                match geo_io::nmea::parse_nmea_line(line) {
                    Ok(msg) => {
                        match msg {
                            geo_io::nmea::NmeaMessage::Gga(fix) => {
                                println!(
                                    "GGA  {} | fix={} sat={} hdop={:.1} alt={:.1}m @ ({:.6}, {:.6})",
                                    fix.time, fix.quality, fix.satellites,
                                    fix.hdop, fix.altitude, fix.lat, fix.lng,
                                );
                                fixes += 1;
                            }
                            geo_io::nmea::NmeaMessage::Rmc(rmc) => {
                                println!(
                                    "RMC  {} | status={} speed={:.1}kt track={:.1}° @ ({:.6}, {:.6})",
                                    rmc.time, rmc.status, rmc.speed_knots,
                                    rmc.track, rmc.lat, rmc.lng,
                                );
                                fixes += 1;
                            }
                            _ => { /* skip unknown */ }
                        }
                    }
                    Err(e) => {
                        eprintln!("  [ERR] {e}");
                        errs += 1;
                    }
                }
            }
            println!("\nParsed {fixes} fixes, {errs} errors");
        }

        #[cfg(feature = "mqtt")]
        IngestAction::Mqtt { broker, port, topic } => {
            let ts_url = std::env::var("TIMESCALE_URL")
                .unwrap_or_else(|_| {
                    std::env::var("DATABASE_URL")
                        .unwrap_or_else(|_| "postgres://geo:geo@localhost:5432/geo_ts".into())
                });

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
