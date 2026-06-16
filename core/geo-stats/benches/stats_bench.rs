use criterion::{black_box, criterion_group, criterion_main, Criterion};
use geo_core::types::BBox;
use geo_stats::{zonal_stats, ZonalStats};

fn bench_zonal_stats_100(c: &mut Criterion) {
    let raster_data: Vec<f64> = (0..10000).map(|i| (i % 256) as f64).collect();
    let rows = 100;
    let cols = 100;
    let nodata = -9999.0;
    let raster_bbox = BBox::new(0.0, 0.0, 1.0, 1.0);
    let zs = ZonalStats {
        data: &raster_data,
        rows,
        cols,
        nodata,
        bbox: raster_bbox,
    };
    let aoi = BBox::new(0.2, 0.2, 0.4, 0.4);
    c.bench_function("zonal_stats_100", |b| {
        b.iter(|| zs.compute(black_box(&aoi), black_box("zone")))
    });
}

fn bench_zonal_stats_representative(c: &mut Criterion) {
    let raster_data: Vec<f64> = (0..10000).map(|i| (i % 256) as f64).collect();
    let rows = 100;
    let cols = 100;
    let nodata = -9999.0;
    let raster_bbox = BBox::new(0.0, 0.0, 1.0, 1.0);
    let aoi = BBox::new(0.3, 0.3, 0.4, 0.4);
    c.bench_function("zonal_stats_representative", |b| {
        b.iter(|| {
            zonal_stats(
                black_box(&raster_data),
                black_box(rows),
                black_box(cols),
                black_box(nodata),
                black_box(raster_bbox),
                black_box(&aoi),
                black_box("zone"),
            )
        })
    });
}

fn bench_zonal_stats_5000(c: &mut Criterion) {
    let raster_data: Vec<f64> = (0..250000).map(|i| (i % 256) as f64).collect();
    let rows = 500;
    let cols = 500;
    let nodata = -9999.0;
    let raster_bbox = BBox::new(0.0, 0.0, 10.0, 10.0);
    let zs = ZonalStats {
        data: &raster_data,
        rows,
        cols,
        nodata,
        bbox: raster_bbox,
    };
    let aoi = BBox::new(2.0, 2.0, 3.0, 3.0);
    c.bench_function("zonal_stats_5000", |b| {
        b.iter(|| zs.compute(black_box(&aoi), black_box("zone")))
    });
}

criterion_group!(
    benches,
    bench_zonal_stats_100,
    bench_zonal_stats_representative,
    bench_zonal_stats_5000
);
criterion_main!(benches);
