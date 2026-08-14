//! Benchmark: Temporal analysis — trend detection, breakpoint detection, season decomposition.
//!
//! Tests:
//! - Mann-Kendall trend test at 10, 100, 1000 years
//! - Linear trend (OLS + Theil-Sen) at various lengths
//! - Pettitt breakpoint detection
//! - Seasonal Mann-Kendall
//! - BFAST simplified breakpoint detection
//! - Seasonal decomposition

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use geo_temporal::decompose::{seasonal_decompose, DecomposeMode};
use geo_temporal::trend::{
    bfast_simple, linear_trend, mann_kendall, pettitt_test, seasonal_mann_kendall, sen_slope,
};

/// Generate synthetic NDVI time series with trend + noise.
/// y = base + trend_slope * t + seasonal_amplitude * sin(2π * t / season_period) + noise
fn make_ndvi_series(
    n: usize,
    base: f64,
    trend_slope: f64,
    seasonal_amp: f64,
    season_period: usize,
    noise_scale: f64,
    seed: u64,
) -> Vec<f64> {
    let mut state = seed;
    let mut next_noise = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        // Box-Muller to get ~N(0,1)
        let u1 = (state as f64) / (u64::MAX as f64);
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let u2 = (state as f64) / (u64::MAX as f64);
        (-2.0 * u1.max(1e-10).ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    };

    (0..n)
        .map(|t| {
            let trend = base + trend_slope * t as f64;
            let seasonal =
                seasonal_amp * (2.0 * std::f64::consts::PI * t as f64 / season_period as f64).sin();
            let noise = next_noise() * noise_scale;
            (trend + seasonal + noise).clamp(0.0, 1.0)
        })
        .collect()
}

// ── Mann-Kendall ──

fn bench_mann_kendall(c: &mut Criterion) {
    for &n in &[10, 100, 1000] {
        let series = make_ndvi_series(n, 0.4, 0.002, 0.1, 12, 0.05, 42);
        let name = format!("mann_kendall_n{}", n);

        let mut group = c.benchmark_group("trend_mk");
        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(&name, |b| {
            b.iter(|| {
                let (tau, p) = mann_kendall(black_box(&series));
                black_box((tau, p))
            })
        });
        group.finish();
    }
}

// ── Linear Trend (OLS + Theil-Sen) ──

fn bench_linear_trend(c: &mut Criterion) {
    for &n in &[10, 100, 1000] {
        let series = make_ndvi_series(n, 0.4, 0.002, 0.1, 12, 0.05, 42);
        let name = format!("linear_trend_n{}", n);

        let mut group = c.benchmark_group("trend_linear");
        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(&name, |b| {
            b.iter(|| {
                let result = linear_trend(black_box(&series));
                black_box(result)
            })
        });
        group.finish();
    }
}

// ── Theil-Sen slope ──

fn bench_sen_slope(c: &mut Criterion) {
    for &n in &[10, 100, 500] {
        let series = make_ndvi_series(n, 0.4, 0.002, 0.1, 12, 0.05, 42);
        let name = format!("sen_slope_n{}", n);

        c.bench_function(&name, |b| {
            b.iter(|| {
                let slope = sen_slope(black_box(&series));
                black_box(slope)
            })
        });
    }
}

// ── Pettitt breakpoint detection ──

fn bench_pettitt(c: &mut Criterion) {
    for &n in &[50, 200, 500] {
        let series = make_ndvi_series(n, 0.4, 0.002, 0.1, 12, 0.05, 42);
        let name = format!("pettitt_test_n{}", n);

        let mut group = c.benchmark_group("trend_pettitt");
        group.throughput(Throughput::Elements(n as u64));
        group.sample_size(if n >= 500 { 20 } else { 50 });
        group.bench_function(&name, |b| {
            b.iter(|| {
                let result = pettitt_test(black_box(&series));
                black_box(result)
            })
        });
        group.finish();
    }
}

// ── Seasonal Mann-Kendall ──

fn bench_seasonal_mk(c: &mut Criterion) {
    for &n in &[36, 120, 600] {
        let series = make_ndvi_series(n, 0.4, 0.002, 0.1, 12, 0.05, 42);
        let name = format!("seasonal_mk_n{}_s12", n);

        let mut group = c.benchmark_group("trend_seasonal_mk");
        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(&name, |b| {
            b.iter(|| {
                let result = seasonal_mann_kendall(black_box(&series), black_box(12));
                black_box(result)
            })
        });
        group.finish();
    }
}

// ── BFAST simplified ──

fn bench_bfast(c: &mut Criterion) {
    for &n in &[100, 500] {
        let series = make_ndvi_series(n, 0.4, 0.002, 0.1, 12, 0.05, 42);
        let name = format!("bfast_simple_n{}_s12", n);

        let mut group = c.benchmark_group("trend_bfast");
        group.throughput(Throughput::Elements(n as u64));
        group.sample_size(if n >= 500 { 10 } else { 30 });
        group.bench_function(&name, |b| {
            b.iter(|| {
                let breaks = bfast_simple(black_box(&series), black_box(12), black_box(3));
                black_box(breaks)
            })
        });
        group.finish();
    }
}

// ── Seasonal decomposition ──

fn bench_seasonal_decompose(c: &mut Criterion) {
    for &n in &[36, 120, 600] {
        let series = make_ndvi_series(n, 0.4, 0.002, 0.15, 12, 0.03, 42);
        let name = format!("seasonal_decompose_n{}_p12", n);

        let mut group = c.benchmark_group("decompose");
        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(&name, |b| {
            b.iter(|| {
                let result = seasonal_decompose(black_box(&series), black_box(12), DecomposeMode::Additive);
                black_box(result)
            })
        });
        group.finish();
    }
}

// ── Warm-up ──

fn bench_temporal_warmup(c: &mut Criterion) {
    let series = make_ndvi_series(30, 0.4, 0.002, 0.1, 12, 0.05, 42);
    c.bench_function("mann_kendall_warmup_n30", |b| {
        b.iter(|| {
            let (tau, p) = mann_kendall(black_box(&series));
            black_box((tau, p))
        })
    });
}

criterion_group!(
    name = temporal_benches;
    config = Criterion::default().configure_from_args();
    targets =
        bench_temporal_warmup,
        bench_mann_kendall,
        bench_linear_trend,
        bench_sen_slope,
        bench_pettitt,
        bench_seasonal_mk,
        bench_bfast,
        bench_seasonal_decompose,
);
criterion_main!(temporal_benches);
