//! Benchmark: CRS coordinate transform strategies (Risk 1).
//!
//! Compares three approaches for PROJ instance management:
//! 1. New Proj per call (baseline)
//! 2. Thread-local cache (current implementation)
//! 3. Pre-allocated pool
//!
//! Run:
//!   cargo bench -p geo-core --bench crs_transform --features proj

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use proj::Proj;
use std::cell::RefCell;

/// Baseline: create a new Proj for every call.
fn bench_new_each_time(c: &mut Criterion) {
    let from_proj = "+proj=longlat +datum=WGS84 +no_defs";
    let to_proj = "+proj=utm +zone=49 +datum=WGS84 +units=m +no_defs";

    c.bench_function("proj_new_each_call", |b| {
        b.iter(|| {
            let proj =
                Proj::new_known_crs(from_proj, to_proj, None).expect("create proj");
            // Transform Shenzhen coordinate
            let (x, y) = proj.convert((113.9, 22.5)).expect("convert");
            criterion::black_box((x, y));
        })
    });
}

/// Thread-local cache (current geo-core approach).
fn bench_thread_local(c: &mut Criterion) {
    thread_local! {
        static CACHE: RefCell<Option<Proj>> = RefCell::new(None);
    }

    c.bench_function("proj_thread_local", |b| {
        b.iter(|| {
            let (x, y) = CACHE.with(|cell| {
                let mut cache = cell.borrow_mut();
                let proj = cache.get_or_insert_with(|| {
                    Proj::new_known_crs(
                        "+proj=longlat +datum=WGS84 +no_defs",
                        "+proj=utm +zone=49 +datum=WGS84 +units=m +no_defs",
                        None,
                    )
                    .expect("create proj")
                });
                proj.convert((113.9, 22.5)).expect("convert")
            });
            criterion::black_box((x, y));
        })
    });
}

/// Pre-instantiated Proj reused across iterations.
fn bench_pre_allocated(c: &mut Criterion) {
    let proj = Proj::new_known_crs(
        "+proj=longlat +datum=WGS84 +no_defs",
        "+proj=utm +zone=49 +datum=WGS84 +units=m +no_defs",
        None,
    )
    .expect("create proj");

    // Note: Proj is not Send+Sync, so we can only benchmark single-threaded.
    // In multi-thread benchmarks, the thread_local approach would be needed.
    c.bench_function("proj_pre_allocated_single_thread", |b| {
        b.iter(|| {
            let (x, y) = proj.convert((113.9, 22.5)).expect("convert");
            criterion::black_box((x, y));
        })
    });
}

/// Multiple CRS pairs (common geo-toolbox transforms).
fn bench_multiple_transforms(c: &mut Criterion) {
    // Common transforms: WGS84 → UTM 49N, UTM 49N → WGS84, WGS84 → Equal Area
    let pairs = vec![
        ("4326→32649", "+proj=longlat +datum=WGS84 +no_defs", "+proj=utm +zone=49 +datum=WGS84 +units=m +no_defs"),
        ("32649→4326", "+proj=utm +zone=49 +datum=WGS84 +units=m +no_defs", "+proj=longlat +datum=WGS84 +no_defs"),
        ("4326→3405", "+proj=longlat +datum=WGS84 +no_defs", "+proj=cea +lon_0=0 +lat_ts=30 +x_0=0 +y_0=0 +datum=WGS84 +units=m +no_defs"),
    ];

    thread_local! {
        static CACHE: RefCell<std::collections::HashMap<&'static str, Proj>> =
            RefCell::new(std::collections::HashMap::new());
    }

    let mut group = c.benchmark_group("proj_multi_crs");

    for (label, from_p, to_p) in &pairs {
        let from_p = *from_p;
        let to_p = *to_p;
        let label = *label;

        group.bench_with_input(
            BenchmarkId::new("thread_local", label),
            &(from_p, to_p),
            |b, &(fp, tp)| {
                b.iter(|| {
                    CACHE.with(|cell| {
                        let mut cache = cell.borrow_mut();
                        let proj = cache.entry(label).or_insert_with(|| {
                            Proj::new_known_crs(fp, tp, None).expect("create proj")
                        });
                        let (x, y) = proj.convert((113.9, 22.5)).expect("convert");
                        criterion::black_box((x, y));
                    });
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("new_each", label),
            &(from_p, to_p),
            |b, &(fp, tp)| {
                b.iter(|| {
                    let proj = Proj::new_known_crs(fp, tp, None).expect("create proj");
                    let (x, y) = proj.convert((113.9, 22.5)).expect("convert");
                    criterion::black_box((x, y));
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_new_each_time,
    bench_thread_local,
    bench_pre_allocated,
    bench_multiple_transforms,
);
criterion_main!(benches);
