//! TimescaleDB hypertable operations.
//!
//! Requires the `timescale` feature. Provides an **independent connection pool**
//! (separate from PostGIS spatial assets), hypertable creation, continuous
//! aggregates, and streaming batch write for GPS/IoT time-series data.
//!
//! ## Design
//!
//! TimescaleDB writes are isolated from the PostGIS `BatchWriter` pool to
//! avoid lock contention between spatial COPY and time-series inserts.
//!
//! Chunk intervals are tuned for expected data rates:
//! - GPS (≤10 Hz) → 1 hour chunks (Risk 4)
//! - IoT sensors (≤1 Hz) → 1 hour chunks

use geo_core::errors::{GeoError, GeoResult};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

/// TimescaleDB connection pool — independent from PostGIS spatial pool.
///
/// Use a smaller pool size (5) since streaming writes are typically
/// single-producer. This avoids competing with the spatial pool (20 conns).
#[derive(Clone)]
pub struct TimescalePool {
    pool: PgPool,
}

impl TimescalePool {
    /// Connect to TimescaleDB with a dedicated pool.
    ///
    /// Uses the same URL as Postgres (TimescaleDB is an extension),
    /// but a separate pool to isolate spatial and time-series workloads.
    pub async fn connect(url: &str) -> GeoResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .min_connections(1)
            .acquire_timeout(Duration::from_secs(10))
            .connect(url)
            .await
            .map_err(|e| GeoError::Database(e.to_string()))?;
        Ok(Self { pool })
    }

    /// Expose the inner pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Ping health check.
    pub async fn ping(&self) -> GeoResult<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| GeoError::Database(e.to_string()))?;
        Ok(())
    }
}

// ── Hypertable creation ──────────────────────────────────────────

/// Create the GPS trajectories hypertable.
///
/// GPS data streams directly into this table with a 1-hour chunk interval
/// (optimized for ≤10 Hz data; see Risk 4). Automatic 1-minute continuous
/// aggregation for real-time dashboards.
pub async fn create_gps_hypertable(pool: &PgPool) -> GeoResult<()> {
    // Base table with PostGIS point geometry
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS gps_trajectories (
            time        TIMESTAMPTZ NOT NULL,
            device_id   TEXT NOT NULL,
            geom        GEOMETRY(Point, 4326),
            altitude    DOUBLE PRECISION,
            hdop        DOUBLE PRECISION,
            satellites  INTEGER
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| geo_core::errors::GeoError::Database(e.to_string()))?;

    // Convert to hypertable
    sqlx::query(
        r#"
        SELECT create_hypertable('gps_trajectories', 'time',
            chunk_time_interval => INTERVAL '1 hour',
            if_not_exists => TRUE
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| geo_core::errors::GeoError::Database(e.to_string()))?;

    // Continuous aggregate: 1-minute downsampling
    sqlx::query(
        r#"
        CREATE MATERIALIZED VIEW IF NOT EXISTS gps_minute_agg
        WITH (timescaledb.continuous) AS
        SELECT
            time_bucket('1 minute', time) AS bucket,
            device_id,
            ST_Centroid(ST_Collect(geom)) AS avg_position,
            AVG(altitude) AS avg_altitude,
            AVG(hdop) AS avg_hdop,
            COUNT(*) AS point_count
        FROM gps_trajectories
        GROUP BY bucket, device_id
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| geo_core::errors::GeoError::Database(e.to_string()))?;

    // Refresh policy: auto-refresh every 2 minutes
    sqlx::query(
        r#"
        SELECT add_continuous_aggregate_policy('gps_minute_agg',
            start_offset    => INTERVAL '1 hour',
            end_offset      => INTERVAL '1 minute',
            schedule_interval => INTERVAL '2 minutes',
            if_not_exists   => TRUE
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| geo_core::errors::GeoError::Database(e.to_string()))?;

    tracing::info!("GPS hypertable + continuous aggregate created");
    Ok(())
}

/// Batch-insert rows into the GPS trajectories hypertable.
///
/// Uses UNNEST for high-throughput streaming writes (no COPY needed —
/// time-series data is row-oriented rather than bulk geometry).
pub async fn batch_insert_gps(pool: &PgPool, records: &[GpsRecord]) -> GeoResult<u64> {
    if records.is_empty() {
        return Ok(0);
    }

    let times: Vec<String> = records.iter().map(|r| r.time.clone()).collect();
    let device_ids: Vec<&str> = records.iter().map(|r| r.device_id.as_str()).collect();
    let lngs: Vec<f64> = records.iter().map(|r| r.lng).collect();
    let lats: Vec<f64> = records.iter().map(|r| r.lat).collect();
    let alts: Vec<Option<f64>> = records.iter().map(|r| r.altitude).collect();
    let hdops: Vec<Option<f64>> = records.iter().map(|r| r.hdop).collect();
    let sats: Vec<Option<i32>> = records.iter().map(|r| r.satellites).collect();

    let count = records.len() as u64;
    sqlx::query(
        r#"
        INSERT INTO gps_trajectories (time, device_id, geom, altitude, hdop, satellites)
        SELECT
            (unnest($1::text[]))::timestamptz,
            unnest($2::text[]),
            ST_SetSRID(ST_MakePoint(unnest($3::float8[]), unnest($4::float8[])), 4326),
            unnest($5::float8[]),
            unnest($6::float8[]),
            unnest($7::int[])
        "#,
    )
    .bind(&times)
    .bind(&device_ids)
    .bind(&lngs)
    .bind(&lats)
    .bind(&alts)
    .bind(&hdops)
    .bind(&sats)
    .execute(pool)
    .await
    .map_err(|e| GeoError::Database(e.to_string()))?;

    tracing::info!("Batch inserted {count} GPS records");
    Ok(count)
}

/// A single GPS fix record for batch insert.
#[derive(Debug, Clone)]
pub struct GpsRecord {
    /// ISO-8601 timestamp (e.g. "2026-06-06T12:00:00Z").
    pub time: String,
    /// Device identifier.
    pub device_id: String,
    /// Longitude (WGS84).
    pub lng: f64,
    /// Latitude (WGS84).
    pub lat: f64,
    /// Altitude in meters (optional).
    pub altitude: Option<f64>,
    /// Horizontal dilution of precision (optional).
    pub hdop: Option<f64>,
    /// Number of satellites used in fix (optional).
    pub satellites: Option<i32>,
}

/// Batch-insert rows into the IoT readings hypertable.
pub async fn batch_insert_iot(pool: &PgPool, records: &[IotRecord]) -> GeoResult<u64> {
    if records.is_empty() {
        return Ok(0);
    }

    let times: Vec<String> = records.iter().map(|r| r.time.clone()).collect();
    let device_ids: Vec<&str> = records.iter().map(|r| r.device_id.as_str()).collect();
    let sensor_types: Vec<&str> = records.iter().map(|r| r.sensor_type.as_str()).collect();
    let values: Vec<f64> = records.iter().map(|r| r.value).collect();
    let lngs: Vec<Option<f64>> = records.iter().map(|r| r.lng).collect();
    let lats: Vec<Option<f64>> = records.iter().map(|r| r.lat).collect();

    let count = records.len() as u64;
    sqlx::query(
        r#"
        INSERT INTO iot_readings (time, device_id, sensor_type, value, geom)
        SELECT
            (unnest($1::text[]))::timestamptz,
            unnest($2::text[]),
            unnest($3::text[]),
            unnest($4::float8[]),
            CASE WHEN unnest($5::float8[]) IS NOT NULL
                 THEN ST_SetSRID(ST_MakePoint(unnest($5::float8[]), unnest($6::float8[])), 4326)
                 ELSE NULL
            END
        "#,
    )
    .bind(&times)
    .bind(&device_ids)
    .bind(&sensor_types)
    .bind(&values)
    .bind(&lngs)
    .bind(&lats)
    .execute(pool)
    .await
    .map_err(|e| GeoError::Database(e.to_string()))?;

    tracing::info!("Batch inserted {count} IoT readings");
    Ok(count)
}

/// A single IoT sensor reading.
#[derive(Debug, Clone)]
pub struct IotRecord {
    /// ISO-8601 timestamp.
    pub time: String,
    /// Device identifier.
    pub device_id: String,
    /// Sensor type: 'temperature', 'humidity', 'pm25', etc.
    pub sensor_type: String,
    /// Measurement value.
    pub value: f64,
    /// Longitude (WGS84, optional — some sensors are stationary).
    pub lng: Option<f64>,
    /// Latitude (WGS84, optional).
    pub lat: Option<f64>,
}

/// Create an IoT sensor readings hypertable.
///
/// 1-hour chunks for sensor data (typically ≤1 Hz).
pub async fn create_iot_hypertable(pool: &PgPool) -> GeoResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS iot_readings (
            time        TIMESTAMPTZ NOT NULL,
            device_id   TEXT NOT NULL,
            sensor_type TEXT NOT NULL,   -- 'temperature' | 'humidity' | 'pm25' | ...
            value       DOUBLE PRECISION NOT NULL,
            geom        GEOMETRY(Point, 4326)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| geo_core::errors::GeoError::Database(e.to_string()))?;

    sqlx::query(
        r#"
        SELECT create_hypertable('iot_readings', 'time',
            chunk_time_interval => INTERVAL '1 hour',
            if_not_exists => TRUE
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| geo_core::errors::GeoError::Database(e.to_string()))?;

    tracing::info!("IoT hypertable created");
    Ok(())
}
