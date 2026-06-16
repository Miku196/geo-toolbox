//! geo-tile benchmarks.
//!
//! Measures MVT encoding, PMTiles reads, and tile-index computations.
//! Run: `cargo bench -p geo-tile`

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use geo_tile::{latlon_to_tile, tile_bounds, tile_to_latlon, MvtEncoder, MvtFeature, MvtLayer, MvtValue};

fn bench_latlon_to_tile(c: &mut Criterion) {
    c.bench_function("latlon_to_tile", |b| {
        b.iter(|| {
            let (x, y, z) = latlon_to_tile(black_box(104.06), black_box(30.57), black_box(12));
            black_box((x, y, z));
        })
    });
}

fn bench_tile_bounds(c: &mut Criterion) {
    c.bench_function("tile_bounds", |b| {
        b.iter(|| {
            let bbox = tile_bounds(black_box(3270), black_box(1671), black_box(12));
            black_box(bbox);
        })
    });
}

fn bench_tile_to_latlon(c: &mut Criterion) {
    c.bench_function("tile_to_latlon", |b| {
        b.iter(|| {
            let (lon, lat) = tile_to_latlon(black_box(3270), black_box(1671), black_box(12));
            black_box((lon, lat));
        })
    });
}

fn bench_mvt_encode_empty_layer(c: &mut Criterion) {
    let encoder = MvtEncoder::new(4096);
    let layers = vec![MvtLayer {
        name: "empty".into(),
        extent: 4096,
        features: vec![],
    }];

    c.bench_function("mvt_encode_empty_layer", |b| {
        b.iter(|| {
            let bytes = encoder.encode(black_box(&layers)).unwrap();
            black_box(bytes);
        })
    });
}

fn bench_mvt_encode_points(c: &mut Criterion) {
    let encoder = MvtEncoder::new(4096);
    let features: Vec<MvtFeature> = (0..1000)
        .map(|i| MvtFeature {
            id: Some(i),
            tags: vec![
                ("name".into(), MvtValue::String(format!("point-{i}"))),
                ("value".into(), MvtValue::Double(i as f64)),
            ],
            geom_type: geo_tile::GeomType::Point,
            geometry: vec![9, 128 + (i as u32 % 256), 0], // MoveTo to varint x, y
        })
        .collect();
    let layers = vec![MvtLayer {
        name: "points".into(),
        extent: 4096,
        features,
    }];

    c.bench_function("mvt_encode_1000_points", |b| {
        b.iter(|| {
            let bytes = encoder.encode(black_box(&layers)).unwrap();
            black_box(bytes);
        })
    });
}

fn bench_mvt_encode_polygon(c: &mut Criterion) {
    let encoder = MvtEncoder::new(4096);
    // A simple square polygon (5 commands: MoveTo + 3×LineTo + ClosePath)
    let geometry = vec![9, 0, 0, 10, 4095, 0, 10, 0, 4095, 10, 4095, 0, 15]; // MoveTo(0,0) + 3 LineTo + ClosePath
    let features = vec![MvtFeature {
        id: None,
        tags: vec![],
        geom_type: geo_tile::GeomType::Polygon,
        geometry,
    }];
    let layers = vec![MvtLayer {
        name: "square".into(),
        extent: 4096,
        features,
    }];

    c.bench_function("mvt_encode_square", |b| {
        b.iter(|| {
            let bytes = encoder.encode(black_box(&layers)).unwrap();
            black_box(bytes);
        })
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(100);
    targets =
        bench_latlon_to_tile,
        bench_tile_bounds,
        bench_tile_to_latlon,
        bench_mvt_encode_empty_layer,
        bench_mvt_encode_points,
        bench_mvt_encode_polygon,
);

criterion_main!(benches);
