use criterion::{criterion_group, criterion_main, Criterion};
use geo_carbon_math::{CarbonEngine, CarbonParams, CarbonReport, EmissionFactor, GeoFeature};

fn make_features(count: usize) -> Vec<GeoFeature> {
    (0..count)
        .map(|i| GeoFeature {
            id: Some(i.to_string()),
            geometry: Some(serde_json::json!({
                "type": "Polygon",
                "coordinates": [[[
                    116.0 + i as f64 * 0.01, 39.0,
                    116.0 + i as f64 * 0.01 + 0.005, 39.0,
                    116.0 + i as f64 * 0.01 + 0.005, 39.005,
                    116.0 + i as f64 * 0.01, 39.005,
                    116.0 + i as f64 * 0.01, 39.0
                ]]]
            })),
            properties: Some(serde_json::json!({
                "crop_type": "rice",
                "area_ha": 1.0 + (i % 10) as f64,
            })),
        })
        .collect()
}

fn make_factors() -> Vec<EmissionFactor> {
    vec![EmissionFactor {
        id: Some("ef_rice".into()),
        crop_type: "rice".into(),
        factor_n2o: 1.6,
        factor_ch4: 130.0,
        factor_co2: 0.0,
        region: "china".into(),
        year_start: 2000,
        year_end: 2030,
        ..Default::default()
    }]
}

fn bench_carbon_10_features(c: &mut Criterion) {
    let features = make_features(10);
    let factors = make_factors();
    let engine = CarbonEngine::new(CarbonParams::default());
    c.bench_function("carbon_10_features", |b| {
        b.iter(|| {
            let _: CarbonReport = engine.calculate(&features, &factors, 2024).unwrap();
        })
    });
}

fn bench_carbon_100_features(c: &mut Criterion) {
    let features = make_features(100);
    let factors = make_factors();
    let engine = CarbonEngine::new(CarbonParams::default());
    c.bench_function("carbon_100_features", |b| {
        b.iter(|| {
            let _: CarbonReport = engine.calculate(&features, &factors, 2024).unwrap();
        })
    });
}

criterion_group!(benches, bench_carbon_10_features, bench_carbon_100_features);
criterion_main!(benches);
