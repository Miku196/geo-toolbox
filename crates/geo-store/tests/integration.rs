//! Integration tests for geo-store.
//!
//! Requires Docker Compose services running:
//!   docker compose -f docker-compose.test.yml up -d
//!
//! Run with:
//!   DATABASE_URL=postgres://geo:geo@localhost:5432/geo_test \
//!     cargo test -p geo-store --test integration -- --ignored --nocapture

//! Integration tests for geo-store.
//!
//! Requires Docker Compose services running:
//!   docker compose -f docker-compose.test.yml up -d
//!
//! Run with:
//!   DATABASE_URL=postgres://geo:geo@localhost:5432/geo_test \
//!     cargo test -p geo-store --test integration -- --ignored --nocapture

use geo::Point;
use geo_store::batch_writer::{BatchWriter, SpatialRow};
use geo_store::postgis::PostgisStore;
use geo_store::run_migrations;
use serde_json::json;

/// Build WKB bytes for a Point (little-endian).
fn point_to_wkb(p: &Point<f64>) -> Vec<u8> {
    let mut wkb = Vec::with_capacity(21);
    wkb.push(0x01); // byte order: LE
    wkb.extend_from_slice(&0x20000000u32.to_le_bytes()); // Point 2D
    wkb.extend_from_slice(&p.x().to_le_bytes());
    wkb.extend_from_slice(&p.y().to_le_bytes());
    wkb
}

/// Helper: connect to the test PostGIS instance.
async fn connect() -> PostgisStore {
    let url =
        std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://geo:geo@localhost:5432/geo_test".to_string());
    PostgisStore::connect(&url).await.expect("failed to connect to PostGIS")
}

// ── PostGIS tests ─────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires Docker: docker compose -f docker-compose.test.yml up -d"]
async fn test_connect_and_ping() {
    let store = connect().await;
    store.ping().await.expect("ping should succeed");
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_postgis_version() {
    let store = connect().await;
    let version = store.check_postgis().await.expect("PostGIS should be available");
    println!("PostGIS: {version}");
    assert!(version.contains("3."), "Expected PostGIS 3.x, got: {version}");
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_migrations() {
    let store = connect().await;
    run_migrations(store.pool()).await.expect("migrations should succeed");
    store.ping().await.expect("ping after migration");
}

// ── BatchWriter tests ─────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_batch_write_empty() {
    let store = connect().await;
    run_migrations(store.pool()).await.unwrap();

    let mut writer = BatchWriter::new(store.pool().clone(), 100);
    let rows = writer.flush().await.expect("flush empty batch");
    assert_eq!(rows, 0);
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_batch_write_single_row() {
    let store = connect().await;
    run_migrations(store.pool()).await.unwrap();

    let mut writer = BatchWriter::new(store.pool().clone(), 100);
    let point = Point::new(113.9, 22.5);
    let wkb = point_to_wkb(&point);
    writer.push(SpatialRow::new(
        wkb,
        json!({"name": "test-point", "type": "park"}),
        "integration-test",
    ));

    let rows = writer.flush().await.expect("flush 1 row");
    assert_eq!(rows, 1);
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_batch_write_100_rows() {
    let store = connect().await;
    run_migrations(store.pool()).await.unwrap();

    let mut writer = BatchWriter::new(store.pool().clone(), 100);

    for i in 0..100 {
        let point = Point::new(i as f64, i as f64 * 0.5);
        writer.push(SpatialRow::new(
            point_to_wkb(&point),
            json!({"seq": i}),
            "batch-test",
        ));
    }

    let rows = writer.flush().await.expect("flush 100 rows");
    assert_eq!(rows, 100);

    // Verify via query
    let result = store
        .query_json("SELECT COUNT(*) AS cnt FROM spatial_assets WHERE source = 'batch-test'")
        .await
        .expect("query");
    assert_eq!(result[0]["cnt"], json!(100));
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_batch_write_1000_rows() {
    let store = connect().await;
    run_migrations(store.pool()).await.unwrap();

    let mut writer = BatchWriter::new(store.pool().clone(), 500);

    for i in 0..1000 {
        let point = Point::new(i as f64 % 180.0, i as f64 % 90.0);
        writer.push(SpatialRow::new(
            point_to_wkb(&point),
            json!({"seq": i, "batch": "performance-1k"}),
            "perf-test",
        ));
    }

    let start = std::time::Instant::now();
    let rows = writer.flush().await.expect("flush 1000 rows");
    let elapsed = start.elapsed();

    assert_eq!(rows, 1000);
    println!("1000 rows via COPY: {:.2?} ({:.0} rows/s)", elapsed, 1000.0 / elapsed.as_secs_f64());
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_insert_geometry() {
    let store = connect().await;
    run_migrations(store.pool()).await.unwrap();

    let point = Point::new(113.9, 22.5);
    let id = store
        .insert_geometry(None, "camofox", &point_to_wkb(&point), &json!({"name": "Shenzhen"}))
        .await
        .expect("insert_geometry");

    assert!(!id.is_nil());
}

// ── DVC tests ─────────────────────────────────────────────────────

#[test]
#[ignore = "requires DVC CLI installed"]
fn test_dvc_available() {
    assert!(geo_store::dvc_available(), "DVC CLI should be installed");
}

#[test]
#[ignore = "requires DVC CLI + initialized repo"]
fn test_dvc_snapshot_and_hash() {
    // Create temp file
    let tmp = std::env::temp_dir().join("dvc-test-geo-toolbox.csv");
    std::fs::write(&tmp, "id,value\n1,100\n").unwrap();

    // Snapshot
    let snapshot = geo_store::dvc_snapshot(tmp.to_str().unwrap())
        .expect("dvc add should succeed");
    assert!(!snapshot.dvc_hash.is_empty());
    assert_eq!(snapshot.file, tmp.to_str().unwrap());

    // Hash
    let hash = geo_store::dvc_hash(tmp.to_str().unwrap())
        .expect("dvc hash should return value");
    assert_eq!(hash, snapshot.dvc_hash);

    // Cleanup
    let _ = std::fs::remove_file(&tmp);
    let _ = std::fs::remove_file(format!("{}.dvc", tmp.to_str().unwrap()));
}

// ── TimescaleDB tests ─────────────────────────────────────────────

#[cfg(feature = "timescale")]
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_create_gps_hypertable() {
    use geo_store::timescale::create_gps_hypertable;

    let ts_url = std::env::var("TIMESCALE_URL")
        .unwrap_or_else(|_| "postgres://geo:geo@localhost:5433/geo_ts".to_string());

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(3)
        .connect(&ts_url)
        .await
        .expect("connect to TimescaleDB");

    create_gps_hypertable(&pool).await.expect("create hypertable");

    // Verify chunk interval is 1 hour
    let row: (String,) =
        sqlx::query_as("SELECT chunk_time_interval FROM timescaledb_information.hypertables WHERE hypertable_name = 'gps_trajectories'")
            .fetch_one(&pool)
            .await
            .expect("query hypertable info");
    assert!(row.0.contains("01:00:00") || row.0.contains("1 hour"), "chunk should be 1 hour, got: {}", row.0);
}

#[cfg(feature = "timescale")]
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_batch_insert_gps() {
    use geo_store::timescale::{batch_insert_gps, create_gps_hypertable, GpsRecord};

    let ts_url = std::env::var("TIMESCALE_URL")
        .unwrap_or_else(|_| "postgres://geo:geo@localhost:5433/geo_ts".to_string());

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(3)
        .connect(&ts_url)
        .await
        .expect("connect to TimescaleDB");

    // Drop first to get clean state
    let _ = sqlx::query("DROP TABLE IF EXISTS gps_trajectories CASCADE")
        .execute(&pool)
        .await;
    create_gps_hypertable(&pool).await.unwrap();

    let records: Vec<GpsRecord> = (0..50)
        .map(|i| GpsRecord {
            time: format!("2026-06-06T12:{:02}:{:02}Z", i / 60, i % 60),
            device_id: "TEST-001".to_string(),
            lng: 113.9 + (i as f64 * 0.001),
            lat: 22.5 + (i as f64 * 0.001),
            altitude: Some(100.0 + i as f64),
            hdop: Some(1.5),
            satellites: Some(12),
        })
        .collect();

    let count = batch_insert_gps(&pool, &records).await.expect("batch insert");
    assert_eq!(count, 50);

    // Verify
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM gps_trajectories WHERE device_id = 'TEST-001'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.0, 50);
}

// ── MinIO tests ───────────────────────────────────────────────────

#[cfg(feature = "minio")]
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_minio_put_and_get() {
    use geo_store::ObjectStoreClient;

    let client = ObjectStoreClient::s3(
        "http://localhost:9000",
        "minioadmin",
        "minioadmin",
        "geo-test",
    )
    .expect("create minio client");

    let key = "integration-test/test.json";
    let data = bytes::Bytes::from(r#"{"hello": "world"}"#);

    client.put(key, data.clone()).await.expect("put object");

    let retrieved = client.get(key).await.expect("get object");
    assert_eq!(retrieved, data);

    // Cleanup
    client.delete(key).await.expect("delete object");
    let exists = client.exists(key).await.expect("check exists");
    assert!(!exists);
}
