use super::*;

fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() < eps
}

#[test]
fn test_erosion_class() {
    assert_eq!(ErosionClass::from_rate(0.0), ErosionClass::Slight);
    assert_eq!(ErosionClass::from_rate(4.9), ErosionClass::Slight);
    assert_eq!(ErosionClass::from_rate(5.0), ErosionClass::Moderate);
    assert_eq!(ErosionClass::from_rate(7.5), ErosionClass::Moderate);
    assert_eq!(ErosionClass::from_rate(10.0), ErosionClass::High);
    assert_eq!(ErosionClass::from_rate(20.0), ErosionClass::Severe);
    assert_eq!(ErosionClass::from_rate(50.0), ErosionClass::VerySevere);
    assert_eq!(ErosionClass::from_rate(100.0), ErosionClass::VerySevere);
    assert_eq!(ErosionClass::Slight.label(), "微度");
    assert_eq!(ErosionClass::VerySevere.label(), "极强烈");
}

#[test]
fn test_r_factor_simple() {
    let r = compute_r_factor_simple(1000.0);
    assert!(r > 3000.0 && r < 4000.0);
}

#[test]
fn test_r_factor_monthly() {
    // 模拟典型南方红壤区月降雨 (mm)
    let monthly: Vec<Vec<f64>> = vec![
        vec![
            50.0, 60.0, 100.0, 150.0, 200.0, 250.0, 200.0, 180.0, 120.0, 80.0, 60.0, 40.0,
        ],
        vec![
            45.0, 55.0, 95.0, 140.0, 190.0, 240.0, 210.0, 170.0, 110.0, 75.0, 55.0, 38.0,
        ],
        vec![
            55.0, 65.0, 105.0, 160.0, 210.0, 260.0, 190.0, 190.0, 130.0, 85.0, 65.0, 42.0,
        ],
    ];
    let refs: Vec<&[f64]> = monthly.iter().map(|v| v.as_slice()).collect();
    let r = compute_r_factor(&refs);
    assert!(r > 0.0);
    assert!(r < 50000.0);
}

#[test]
fn test_r_factor_empty() {
    assert_eq!(compute_r_factor(&[]), 0.0);
    assert_eq!(compute_r_factor_simple(0.0), 0.0);
}

#[test]
fn test_k_factor() {
    // 粉砂壤土: 砂20% 粉65% 粘15%
    let k = compute_k_factor(20.0, 65.0, 15.0, 7.0, 2.0, 2, 3);
    assert!(k > 0.001 && k < 0.7);
}

#[test]
fn test_k_factor_simple() {
    // 粉砂壤土简化计算
    let k = compute_k_factor_simple(20.0, 65.0, 15.0, 2.0);
    assert!(k > 0.01 && k < 0.5);
}

#[test]
fn test_c_factor_from_ndvi() {
    let ndvi = vec![0.0, 0.3, 0.5, 0.7, 1.0, -0.1];
    let c = compute_c_factor_from_ndvi(&ndvi);
    // NDVI=0 → C≈1.0
    assert!(approx_eq(c[0], 1.0, 1e-6));
    // NDVI=1 → C≈0.001
    assert!(c[4] < 0.01);
    // NDVI<0 → C=1.0
    assert!(approx_eq(c[5], 1.0, 1e-6));
    // NDVI 越高，C 越低
    assert!(c[1] > c[2]);
    assert!(c[2] > c[3]);
}

#[test]
fn test_c_factor_landuse() {
    assert!(approx_eq(c_factor_for_landuse("forest"), 0.005, 1e-6));
    assert!(approx_eq(c_factor_for_landuse("bare"), 1.0, 1e-6));
    assert!(approx_eq(c_factor_for_landuse("裸地"), 1.0, 1e-6));
    assert!(approx_eq(c_factor_for_landuse("water"), 0.0, 1e-6));
    assert!(c_factor_for_landuse("unknown") > 0.0);
}

#[test]
fn test_p_factor() {
    let slopes = vec![0.5, 3.0, 10.0, 25.0];
    let p = compute_p_factor(&slopes, PracticeType::Contouring);
    assert!(p.iter().all(|&v| v > 0.0 && v <= 1.0));
    // 梯田 P 因子小于等高耕作
    let p_t = compute_p_factor(&slopes, PracticeType::Terracing);
    for i in 0..slopes.len() {
        assert!(p_t[i] <= p[i]);
    }
}

#[test]
fn test_slope_from_dem() {
    // 平坦 DEM → 坡度 ≈ 0
    let dem = vec![100.0; 9];
    let slope = compute_slope_from_dem(&dem, 10.0, 3, 3);
    assert!(slope.iter().all(|&s| s < 0.001));

    // 斜坡 DEM
    let dem2 = vec![100.0, 100.0, 100.0, 100.0, 95.0, 90.0, 100.0, 95.0, 90.0];
    let slope2 = compute_slope_from_dem(&dem2, 10.0, 3, 3);
    // 中心像元应有正坡度
    assert!(slope2[4] > 0.0);
}

#[test]
fn test_ls_factor() {
    // 平坦 → LS = 0
    let flat = vec![0.0; 4];
    let ls = compute_ls_factor(&flat, 22.13, 2, 2);
    assert!(ls.iter().all(|&v| v == 0.0));

    // 坡度 10° → LS > 0
    let sloped = vec![10.0; 4];
    let ls2 = compute_ls_factor(&sloped, 22.13, 2, 2);
    assert!(ls2.iter().all(|&v| v > 0.0));
}

#[test]
fn test_compute_soil_loss() {
    let n = 6;
    let r = vec![5000.0; n];
    let k = vec![0.04; n];
    let ls = vec![1.0; n];
    let c = vec![0.2; n];
    let p = vec![1.0; n];
    let loss = compute_soil_loss(&r, &k, &ls, &c, &p, n);
    // 5000 × 0.04 × 1.0 × 0.2 × 1.0 = 40
    assert!(approx_eq(loss[0], 40.0, 1e-6));
    // 长度不足的元素填 0
    let r2 = vec![5000.0; 3];
    let loss2 = compute_soil_loss(&r2, &k, &ls, &c, &p, n);
    assert!(approx_eq(loss2[0], 40.0, 1e-6));
    assert_eq!(loss2[5], 0.0);
}

#[test]
fn test_musle_sediment() {
    let q = vec![30.0, 0.0, 50.0];
    let qp = vec![10.0, 20.0, 0.0];
    let k = vec![0.04, 0.04, 0.04];
    let ls = vec![1.0, 1.0, 1.0];
    let c = vec![0.2, 0.2, 0.2];
    let p = vec![1.0, 1.0, 1.0];
    let sed = compute_musle_sediment(&q, &qp, &k, &ls, &c, &p, 3);
    // cell 0: 11.8 × (30×10)^0.56 × 0.04 × 1.0 × 0.2 × 1.0 > 0
    assert!(sed[0] > 0.0, "cell0 should have sediment, got {}", sed[0]);
    // cell 1: Q=0 → sediment=0
    assert_eq!(sed[1], 0.0);
    // cell 2: qp=0 → sediment=0
    assert_eq!(sed[2], 0.0);
    // Running 11.8 × (30×10)^0.56 × 0.04 × 0.2
    // = 11.8 × (300)^0.56 × 0.008
    let expected = 11.8 * (300.0_f64).powf(0.56) * 0.04 * 0.2;
    assert!(
        (sed[0] - expected).abs() < 1e-6,
        "sed[0]={} expected={}",
        sed[0],
        expected
    );
}

#[test]
fn test_assess_soil_loss_flat() {
    let dem = vec![100.0; 9];
    let ndvi = vec![0.5; 9];
    let result = assess_soil_loss(
        &dem,
        None,
        30.0,
        3,
        3,
        4000.0,
        None,
        &ndvi,
        PracticeType::None,
    );

    // 平坦 → LS ≈ 0 → loss ≈ 0
    assert!(result.soil_loss_mean < 1.0);
    assert!(approx_eq(result.area_ha, 0.81, 1e-4)); // 0.81 ha
                                                    // 100% 微度
    let slight_pct = result
        .class_distribution
        .iter()
        .find(|(c, _)| *c == ErosionClass::Slight)
        .map(|(_, p)| *p)
        .unwrap_or(0.0);
    assert!(approx_eq(slight_pct, 100.0, 1e-3));
}

#[test]
fn test_assess_soil_loss_steep() {
    // 陡坡 DEM（八字形：中间低，四周高）
    let dem = vec![
        120.0, 110.0, 120.0, 110.0, 100.0, 110.0, 120.0, 110.0, 120.0,
    ];
    let ndvi = vec![0.3; 9];
    let result = assess_soil_loss(
        &dem,
        None,
        10.0,
        3,
        3,
        5000.0,
        None,
        &ndvi,
        PracticeType::None,
    );

    // 有坡度 → 应有土壤流失
    assert!(result.soil_loss_mean > 0.0);
    assert!(result.ls_factor_mean > 0.0);
    assert!(result.soil_loss_total > 0.0);
}

#[test]
fn test_assess_with_terracing() {
    let dem = vec![
        120.0, 110.0, 120.0, 110.0, 100.0, 110.0, 120.0, 110.0, 120.0,
    ];
    let ndvi = vec![0.3; 9];

    let result_none = assess_soil_loss(
        &dem,
        None,
        10.0,
        3,
        3,
        5000.0,
        None,
        &ndvi,
        PracticeType::None,
    );
    let result_terrace = assess_soil_loss(
        &dem,
        None,
        10.0,
        3,
        3,
        5000.0,
        None,
        &ndvi,
        PracticeType::Terracing,
    );

    // 梯田 P 因子低 → 土壤流失应更少
    assert!(result_terrace.soil_loss_mean < result_none.soil_loss_mean);
    assert!(result_terrace.p_factor_mean < result_none.p_factor_mean);
}

#[test]
fn test_k_factor_simple_typical() {
    // 典型中国南方红壤
    let k = compute_k_factor_simple(35.0, 40.0, 25.0, 1.5);
    assert!(k > 0.01 && k < 0.4);

    // 砂土 — K 值较低
    let k_sand = compute_k_factor_simple(80.0, 10.0, 10.0, 0.5);
    assert!(k_sand < 0.2);

    // 粘土 — K 值中等
    let k_clay = compute_k_factor_simple(20.0, 20.0, 60.0, 3.0);
    assert!(k_clay > 0.01 && k_clay < 0.5);
}
