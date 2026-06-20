use criterion::{criterion_group, criterion_main, Criterion};
use geo_carbon_math::{CarbonEngine, CarbonReport, EmissionFactor, GeoFeature};

fn make_features(count: usize) -> Vec<GeoFeature> {
    (0..count)
        .map(|i| {
            let geojson = format!(
                r#"{{
                "type": "Polygon",
                "coordinates": [[[
                    {}, 39.0,
                    {}, 39.0,
                    {}, 39.005,
                    {}, 39.005,
                    {}, 39.0
                ]]]
            }}"#,
                116.0 + i as f64 * 0.01,
                116.0 + i as f64 * 0.01 + 0.005,
                116.0 + i as f64 * 0.01 + 0.005,
                116.0 + i as f64 * 0.01,
                116.0 + i as f64 * 0.01,
            );
            GeoFeature::new("rice", &geojson).unwrap()
        })
        .collect()
}

fn make_factors() -> Vec<EmissionFactor> {
    vec![EmissionFactor {
        category: "rice".into(),
        subcategory: None,
        source: "bench".into(),
        region: Some("china".into()),
        factor_value: 131.6,
        unit: "tCO₂e/ha/yr".into(),
        valid_from_year: 2000,
        valid_to_year: Some(2030),
        gas_factors: vec![],
        uncertainty_pct: None,
        scope: None,
        activity_type: None,
        fuel_type: None,
        ncv_override: None,
        cc_override: None,
        ox_override: None,
        grid_ef: None,
    }]
}

fn bench_carbon_10_features(c: &mut Criterion) {
    let features = make_features(10);
    let factors = make_factors();
    let engine = CarbonEngine::new();
    c.bench_function("carbon_10_features", |b| {
        b.iter(|| {
            let _: CarbonReport = engine.calculate(&features, &factors, 2024).unwrap();
        })
    });
}

fn bench_carbon_100_features(c: &mut Criterion) {
    let features = make_features(100);
    let factors = make_factors();
    let engine = CarbonEngine::new();
    c.bench_function("carbon_100_features", |b| {
        b.iter(|| {
            let _: CarbonReport = engine.calculate(&features, &factors, 2024).unwrap();
        })
    });
}

criterion_group!(benches, bench_carbon_10_features, bench_carbon_100_features);
criterion_main!(benches);
