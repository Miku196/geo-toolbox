use criterion::{black_box, criterion_group, criterion_main, Criterion};
use geo_types::{Coord, LineString, Point, Polygon};
use geo_vector::{
    buffer, intersect, kernel_density, line_density, simplify, union_all, BufferMode,
};

fn make_polygon(pts_per_ring: usize) -> Polygon<f64> {
    let exterior = LineString::new(
        (0..pts_per_ring)
            .map(|i| {
                let angle = 2.0 * std::f64::consts::PI * i as f64 / pts_per_ring as f64;
                Coord {
                    x: angle.cos() * 0.01,
                    y: angle.sin() * 0.01,
                }
            })
            .collect(),
    );
    Polygon::new(exterior, vec![])
}

fn bench_buffer_10(c: &mut Criterion) {
    let poly = make_polygon(10);
    c.bench_function("buffer_10", |b| {
        b.iter(|| {
            buffer(
                black_box(&poly),
                black_box(100.0),
                BufferMode::Precise {
                    quadrant_segments: 4,
                },
            )
        })
    });
}

fn bench_buffer_100(c: &mut Criterion) {
    let poly = make_polygon(100);
    c.bench_function("buffer_100", |b| {
        b.iter(|| {
            buffer(
                black_box(&poly),
                black_box(100.0),
                BufferMode::Precise {
                    quadrant_segments: 4,
                },
            )
        })
    });
}

fn bench_buffer_1000(c: &mut Criterion) {
    let poly = make_polygon(1000);
    c.bench_function("buffer_1000", |b| {
        b.iter(|| {
            buffer(
                black_box(&poly),
                black_box(100.0),
                BufferMode::Precise {
                    quadrant_segments: 4,
                },
            )
        })
    });
}

fn bench_intersect_100(c: &mut Criterion) {
    let poly_a = make_polygon(64);
    let poly_b = {
        let exterior = LineString::new(vec![
            Coord {
                x: -0.005,
                y: -0.005,
            },
            Coord {
                x: 0.015,
                y: -0.005,
            },
            Coord { x: 0.015, y: 0.015 },
            Coord {
                x: -0.005,
                y: 0.015,
            },
            Coord {
                x: -0.005,
                y: -0.005,
            },
        ]);
        Polygon::new(exterior, vec![])
    };
    c.bench_function("intersect_100", |b| {
        b.iter(|| intersect(black_box(&poly_a), black_box(&poly_b)))
    });
}

fn bench_union_all_10(c: &mut Criterion) {
    let polys: Vec<Polygon<f64>> = (0..10)
        .map(|i| {
            let offset = i as f64 * 0.1;
            let exterior = LineString::new(vec![
                Coord { x: offset, y: 0.0 },
                Coord {
                    x: offset + 0.05,
                    y: 0.0,
                },
                Coord {
                    x: offset + 0.05,
                    y: 0.05,
                },
                Coord { x: offset, y: 0.05 },
                Coord { x: offset, y: 0.0 },
            ]);
            Polygon::new(exterior, vec![])
        })
        .collect();
    c.bench_function("union_all_10", |b| b.iter(|| union_all(black_box(&polys))));
}

fn bench_union_all_100(c: &mut Criterion) {
    let polys: Vec<Polygon<f64>> = (0..100)
        .map(|i| {
            let offset = i as f64 * 0.01;
            let exterior = LineString::new(vec![
                Coord { x: offset, y: 0.0 },
                Coord {
                    x: offset + 0.005,
                    y: 0.0,
                },
                Coord {
                    x: offset + 0.005,
                    y: 0.005,
                },
                Coord {
                    x: offset,
                    y: 0.005,
                },
                Coord { x: offset, y: 0.0 },
            ]);
            Polygon::new(exterior, vec![])
        })
        .collect();
    c.bench_function("union_all_100", |b| b.iter(|| union_all(black_box(&polys))));
}

fn bench_simplify_100(c: &mut Criterion) {
    let poly = make_polygon(1000);
    c.bench_function("simplify_100", |b| {
        b.iter(|| simplify(black_box(&poly), black_box(0.001)))
    });
}

fn bench_kernel_density_100(c: &mut Criterion) {
    let points: Vec<(f64, f64)> = (0..100)
        .map(|i| (i as f64 * 0.01, i as f64 * 0.01))
        .collect();
    c.bench_function("kernel_density_100", |b| {
        b.iter(|| {
            kernel_density(
                black_box(&points),
                black_box(100),
                black_box(100),
                black_box((0.0, 0.0, 1.0, 1.0)),
                black_box(0.1),
            )
        })
    });
}

fn bench_kernel_density_1000(c: &mut Criterion) {
    let points: Vec<(f64, f64)> = (0..1000)
        .map(|i| (i as f64 * 0.001, (i as f64 * 0.001).sin()))
        .collect();
    c.bench_function("kernel_density_1000", |b| {
        b.iter(|| {
            kernel_density(
                black_box(&points),
                black_box(100),
                black_box(100),
                black_box((0.0, -1.0, 1.0, 1.0)),
                black_box(0.05),
            )
        })
    });
}

fn bench_line_density_100(c: &mut Criterion) {
    let lines: Vec<(f64, f64, f64, f64)> = (0..100)
        .map(|i| {
            let offset_x = i as f64 * 0.1;
            (offset_x, 0.0, offset_x + 0.1, 0.1)
        })
        .collect();
    c.bench_function("line_density_100", |b| {
        b.iter(|| {
            line_density(
                black_box(&lines),
                black_box(100),
                black_box(100),
                black_box((0.0, 0.0, 10.0, 1.0)),
            )
        })
    });
}

criterion_group!(
    benches,
    bench_buffer_10,
    bench_buffer_100,
    bench_buffer_1000,
    bench_intersect_100,
    bench_union_all_10,
    bench_union_all_100,
    bench_simplify_100,
    bench_kernel_density_100,
    bench_kernel_density_1000,
    bench_line_density_100,
);
criterion_main!(benches);
