//! Benchmark: COPY batch write vs single INSERT for PostGIS.
//!
//! Requires Docker:
//!   docker compose -f docker-compose.test.yml up -d
//!
//! Run:
//!   DATABASE_URL=postgres://geo:geo@localhost:5432/geo_test \
//!     cargo bench -p geo-store --bench batch_write

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use geo::Point;
use geo_store::batch_writer::{BatchWriter, SpatialRow};
use geo_store::postgis::PostgisStore;
use geo_store::run_migrations;
use serde_json::json;
use std::time::Duration;
use tokio::runtime::Runtime;

fn point_to_wkb(p: &Point<f64>) -> Vec<u8> {
    let mut wkb = Vec::with_capacity(21);
    wkb.push(0x01);
    wkb.extend_from_slice(&0x20000000u32.to_le_bytes());
    wkb.extend_from_slice(&p.x().to_le_bytes());
    wkb.extend_from_slice(&p.y().to_le_bytes());
    wkb
}

async fn setup() -> PostgisStore {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://geo:geo@localhost:5432/geo_test".to_string());
    let store = PostgisStore::connect(&url).await.expect("connect");
    run_migrations(store.pool()).await.expect("migrate");
    store
}

fn bench_copy_batch(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("postgis_write");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    let store = rt.block_on(setup());

    for size in [100u64, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("COPY_batch", size),
            &size,
            |b, &s| {
                b.to_async(&rt).iter(|| async {
                    let mut writer = BatchWriter::new(store.pool().clone(), s as usize);
                    for i in 0..s {
                        let point = Point::new(
                            (i % 360) as f64 - 180.0,
                            (i % 180) as f64 - 90.0,
                        );
                        writer.push(SpatialRow::new(
                            point_to_wkb(&point),
                            json!({"seq": i, "bench": "copy"}),
                            "bench-copy",
                        ));
                    }
                    writer.flush().await.expect("flush");
                })
            },
        );
    }

    group.finish();
}

fn bench_single_insert(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("postgis_insert_vs_copy");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(15));

    let store = rt.block_on(setup());

    for size in [100u64, 1_000] {
        group.bench_with_input(
            BenchmarkId::new("single_INSERT", size),
            &size,
            |b, &s| {
                b.to_async(&rt).iter(|| async {
                    for i in 0..s {
                        let point = Point::new(
                            (i % 360) as f64 - 180.0,
                            (i % 180) as f64 - 90.0,
                        );
                        store
                            .insert_geometry(
                                None,
                                "bench-insert",
                                &point_to_wkb(&point),
                                &json!({"seq": i, "bench": "insert"}),
                            )
                            .await
                            .expect("insert");
                    }
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_copy_batch, bench_single_insert);
criterion_main!(benches);
