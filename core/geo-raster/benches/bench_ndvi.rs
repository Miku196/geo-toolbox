use criterion::{criterion_group, criterion_main, Criterion};
use geo_raster::ndvi::{compute_ndvi, ndvi_difference};
use geo_raster::RasterBand;

fn make_raster(size: usize) -> (RasterBand, RasterBand) {
    let n = size * size;
    let red: Vec<f64> = (0..n)
        .map(|i| 0.05 + (i as f64 % 255.0) / 255.0 * 0.3)
        .collect();
    let nir: Vec<f64> = (0..n)
        .map(|i| 0.1 + (i as f64 % 255.0) / 255.0 * 0.5)
        .collect();
    (
        RasterBand::new("red", size, size, red, -999.0),
        RasterBand::new("nir", size, size, nir, -999.0),
    )
}

fn bench_ndvi_256x256(c: &mut Criterion) {
    let (red, nir) = make_raster(256);
    c.bench_function("ndvi_256x256", |b| {
        b.iter(|| {
            let _ = compute_ndvi(&red, &nir).unwrap();
        })
    });
}

fn bench_ndvi_1024x1024(c: &mut Criterion) {
    let (red, nir) = make_raster(1024);
    c.bench_function("ndvi_1024x1024", |b| {
        b.iter(|| {
            let _ = compute_ndvi(&red, &nir).unwrap();
        })
    });
}

fn bench_ndvi_difference(c: &mut Criterion) {
    let (red1, nir1) = make_raster(256);
    let (red2, nir2) = make_raster(256);
    let prev = compute_ndvi(&red1, &nir1).unwrap();
    let curr = compute_ndvi(&red2, &nir2).unwrap();
    c.bench_function("ndvi_difference_256x256", |b| {
        b.iter(|| {
            let _ = ndvi_difference(&prev, &curr).unwrap();
        })
    });
}

criterion_group!(
    benches,
    bench_ndvi_256x256,
    bench_ndvi_1024x1024,
    bench_ndvi_difference
);
criterion_main!(benches);
