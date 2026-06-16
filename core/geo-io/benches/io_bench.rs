use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_parse_feature_collection_10(c: &mut Criterion) {
    let geojson = r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"id":1},"geometry":{"type":"Point","coordinates":[116.4,39.9]}},{"type":"Feature","properties":{"id":2},"geometry":{"type":"Point","coordinates":[116.5,39.8]}},{"type":"Feature","properties":{"id":3},"geometry":{"type":"Point","coordinates":[116.6,39.7]}},{"type":"Feature","properties":{"id":4},"geometry":{"type":"Point","coordinates":[116.7,39.9]}},{"type":"Feature","properties":{"id":5},"geometry":{"type":"Point","coordinates":[116.8,39.8]}},{"type":"Feature","properties":{"id":6},"geometry":{"type":"Point","coordinates":[116.9,39.7]}},{"type":"Feature","properties":{"id":7},"geometry":{"type":"Point","coordinates":[117.0,39.9]}},{"type":"Feature","properties":{"id":8},"geometry":{"type":"Point","coordinates":[117.1,39.8]}},{"type":"Feature","properties":{"id":9},"geometry":{"type":"Point","coordinates":[117.2,39.7]}},{"type":"Feature","properties":{"id":10},"geometry":{"type":"Point","coordinates":[117.3,39.9]}}]}"#;
    c.bench_function("parse_feature_collection_10", |b| {
        b.iter(|| geo_io::parse_feature_collection(black_box(geojson)))
    });
}

fn bench_extract_bbox_1000(c: &mut Criterion) {
    let mut features = Vec::with_capacity(1000);
    for i in 0..1000 {
        let lon = 116.0 + (i % 100) as f64 * 0.1;
        let lat = 39.0 + (i / 100) as f64 * 0.1;
        features.push(format!(
            r#"{{"type":"Feature","properties":{{"id":{}}},"geometry":{{"type":"Point","coordinates":[{},{}]}}}}"#,
            i, lon, lat
        ));
    }
    let geojson = format!(
        r#"{{"type":"FeatureCollection","features":[{}]}}"#,
        features.join(",")
    );
    c.bench_function("extract_bbox_1000", |b| {
        b.iter(|| geo_io::extract_bbox(black_box(&geojson)))
    });
}

fn bench_parse_nmea_lines(c: &mut Criterion) {
    let nmea = "$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\n$GPGGA,123520,4807.039,N,01131.001,E,1,08,0.9,545.4,M,46.9,M,,*48\n$GPGGA,123521,4807.040,N,01131.002,E,1,08,0.9,545.4,M,46.9,M,,*49";
    c.bench_function("parse_nmea_lines", |b| {
        b.iter(|| {
            for line in black_box(nmea).lines() {
                let _ = geo_io::nmea::parse_nmea_line(line);
            }
        })
    });
}

fn bench_parse_gga(c: &mut Criterion) {
    let gga = "$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47";
    c.bench_function("parse_gga", |b| {
        b.iter(|| geo_io::nmea::parse_gga(black_box(gga)))
    });
}

fn bench_parse_rmc(c: &mut Criterion) {
    let rmc = "$GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W*6A";
    c.bench_function("parse_rmc", |b| {
        b.iter(|| geo_io::nmea::parse_rmc(black_box(rmc)))
    });
}

criterion_group!(
    benches,
    bench_parse_feature_collection_10,
    bench_extract_bbox_1000,
    bench_parse_nmea_lines,
    bench_parse_gga,
    bench_parse_rmc,
);
criterion_main!(benches);
