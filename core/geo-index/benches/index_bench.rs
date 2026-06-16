use criterion::{black_box, criterion_group, criterion_main, Criterion};
use geo_core::types::BBox;
use geo_index::{bbox_to_geohashes, decode, encode, neighbors, Quadtree, RTree};

fn bench_geohash_encode_5(c: &mut Criterion) {
    c.bench_function("geohash_encode_5", |b| {
        b.iter(|| encode(black_box(116.4), black_box(39.9), black_box(5)))
    });
}

fn bench_geohash_encode_8(c: &mut Criterion) {
    c.bench_function("geohash_encode_8", |b| {
        b.iter(|| encode(black_box(116.4), black_box(39.9), black_box(8)))
    });
}

fn bench_geohash_encode_11(c: &mut Criterion) {
    c.bench_function("geohash_encode_11", |b| {
        b.iter(|| encode(black_box(116.4), black_box(39.9), black_box(11)))
    });
}

fn bench_geohash_decode(c: &mut Criterion) {
    let hash = "wx4g0";
    c.bench_function("geohash_decode", |b| b.iter(|| decode(black_box(hash))));
}

fn bench_geohash_neighbors(c: &mut Criterion) {
    let hash = "wx4g0";
    c.bench_function("geohash_neighbors", |b| {
        b.iter(|| neighbors(black_box(hash)))
    });
}

fn bench_geohash_bbox_to_geohashes(c: &mut Criterion) {
    let bbox = BBox::new(116.3, 39.8, 116.5, 40.0);
    c.bench_function("bbox_to_geohashes", |b| {
        b.iter(|| bbox_to_geohashes(black_box(&bbox), black_box(6)))
    });
}

fn bench_quadtree_load_100(c: &mut Criterion) {
    let bboxes: Vec<BBox> = (0..100)
        .map(|i| {
            let lon = 116.0 + (i % 10) as f64 * 0.1;
            let lat = 39.0 + (i / 10) as f64 * 0.1;
            BBox::new(lon, lat, lon + 0.05, lat + 0.05)
        })
        .collect();
    c.bench_function("quadtree_load_100", |b| {
        b.iter(|| {
            let mut qt = Quadtree::new();
            qt.load(black_box(bboxes.clone()));
        })
    });
}

fn bench_quadtree_load_10000(c: &mut Criterion) {
    let bboxes: Vec<BBox> = (0..10000)
        .map(|i| {
            let lon = 116.0 + (i % 100) as f64 * 0.1;
            let lat = 39.0 + (i / 100) as f64 * 0.1;
            BBox::new(lon, lat, lon + 0.05, lat + 0.05)
        })
        .collect();
    c.bench_function("quadtree_load_10000", |b| {
        b.iter(|| {
            let mut qt = Quadtree::new();
            qt.load(black_box(bboxes.clone()));
        })
    });
}

fn bench_quadtree_query(c: &mut Criterion) {
    let bboxes: Vec<BBox> = (0..5000)
        .map(|i| {
            let lon = 116.0 + (i % 100) as f64 * 0.1;
            let lat = 39.0 + (i / 100) as f64 * 0.1;
            BBox::new(lon, lat, lon + 0.05, lat + 0.05)
        })
        .collect();
    let mut qt = Quadtree::new();
    qt.load(bboxes);
    let query_bbox = BBox::new(116.5, 39.5, 117.0, 40.0);
    c.bench_function("quadtree_query", |b| {
        b.iter(|| qt.query(black_box(&query_bbox)))
    });
}

fn bench_rtree_load_and_query(c: &mut Criterion) {
    let bboxes: Vec<BBox> = (0..5000)
        .map(|i| {
            let lon = 116.0 + (i % 100) as f64 * 0.1;
            let lat = 39.0 + (i / 100) as f64 * 0.1;
            BBox::new(lon, lat, lon + 0.05, lat + 0.05)
        })
        .collect();
    c.bench_function("rtree_load_5000", |b| {
        b.iter(|| {
            let mut rt = RTree::new();
            rt.load(black_box(bboxes.clone()));
        })
    });
    let mut rt = RTree::new();
    rt.load(bboxes);
    let query_bbox = BBox::new(116.5, 39.5, 117.0, 40.0);
    c.bench_function("rtree_query_5000", |b| {
        b.iter(|| rt.query(black_box(&query_bbox)))
    });
}

criterion_group!(
    benches,
    bench_geohash_encode_5,
    bench_geohash_encode_8,
    bench_geohash_encode_11,
    bench_geohash_decode,
    bench_geohash_neighbors,
    bench_geohash_bbox_to_geohashes,
    bench_quadtree_load_100,
    bench_quadtree_load_10000,
    bench_quadtree_query,
    bench_rtree_load_and_query,
);
criterion_main!(benches);
