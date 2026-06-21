/// 泥沙输移比 (Sediment Delivery Ratio) 与 MUSLE 事件土壤流失模型。
///
/// 将 RUSLE 地块边缘土壤流失量转化为入河泥沙量：
///   入河泥沙 = RUSLE 流失量 × SDR
///
/// SDR 随流域面积增大而减小，因为泥沙沿途沉积。
/// MUSLE (Modified USLE) 替代 R 因子为径流项，用于单场暴雨评估。
use serde::{Deserialize, Serialize};

/// SDR 计算方法。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SdrMethod {
    /// USDA: SDR = A^(-0.125)，A = 流域面积 (km²)
    Usda,
    /// Renfro (1975): log₁₀(SDR) = 1.7935 - 0.1419·log₁₀(A)
    Renfro,
    /// Vanoni (1975): SDR = 0.42 · A^(-0.125)
    Vanoni,
    /// Boyce (1975): SDR = 0.41 · A^(-0.3)
    Boyce,
}

/// SDR 计算结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdrResult {
    /// 流域面积 (km²)
    pub drainage_area_km2: f64,
    /// 泥沙输移比 (0~1)
    pub sdr: f64,
    /// 使用的计算方法
    pub method: SdrMethod,
    /// 入河泥沙量 (t/yr)
    pub sediment_yield_t_yr: f64,
    /// SDR 前土壤流失量 (t/yr)
    pub source_soil_loss_t_yr: f64,
}

/// 根据流域面积和指定方法计算 SDR。
///
/// 若 `drainage_area_km2 <= 0`，返回 `None`。
pub fn compute_sdr(drainage_area_km2: f64, method: &SdrMethod) -> Option<f64> {
    if drainage_area_km2 <= 0.0 {
        return None;
    }
    let sdr = match method {
        SdrMethod::Usda => drainage_area_km2.powf(-0.125),
        SdrMethod::Renfro => 10.0_f64.powf(1.7935 - 0.1419 * drainage_area_km2.log10()),
        SdrMethod::Vanoni => 0.42 * drainage_area_km2.powf(-0.125),
        SdrMethod::Boyce => 0.41 * drainage_area_km2.powf(-0.3),
    };
    Some(sdr.clamp(0.0, 1.0))
}

/// 将 SDR 应用于 RUSLE 土壤流失结果，计算入河泥沙量。
///
/// - `soil_loss_t_ha_yr`: RUSLE 年均单位面积流失量 (t/ha/yr)
/// - `area_ha`: 地块或流域面积 (ha)
/// - `drainage_area_km2`: 上游集水面积 (km²)
pub fn apply_sdr_to_rusle(
    soil_loss_t_ha_yr: f64,
    area_ha: f64,
    drainage_area_km2: f64,
    method: SdrMethod,
) -> Option<SdrResult> {
    if drainage_area_km2 <= 0.0 || soil_loss_t_ha_yr < 0.0 || area_ha <= 0.0 {
        return None;
    }
    let source_soil_loss_t_yr = soil_loss_t_ha_yr * area_ha;
    let sdr = compute_sdr(drainage_area_km2, &method)?;
    let sediment_yield_t_yr = source_soil_loss_t_yr * sdr;

    Some(SdrResult {
        drainage_area_km2,
        sdr,
        method,
        sediment_yield_t_yr,
        source_soil_loss_t_yr,
    })
}

/// MUSLE (Modified Universal Soil Loss Equation) — 单场暴雨事件土壤流失。
///
/// 公式：A = 11.8 × (Q × qp)^0.56 × K × LS × C × P
///
/// - Q: 暴雨径流量 (m³)
/// - qp: 洪峰流量 (m³/s)
/// - K, LS, C, P: 同 RUSLE
///
/// 返回土壤流失量（吨）。
pub fn musle_event(
    runoff_volume_m3: f64,
    peak_flow_m3s: f64,
    k_factor: f64,
    ls_factor: f64,
    c_factor: f64,
    p_factor: f64,
) -> f64 {
    let runoff_product = (runoff_volume_m3 * peak_flow_m3s).powf(0.56);
    11.8 * runoff_product * k_factor * ls_factor * c_factor * p_factor
}

/// 多重现期 MUSLE 分析。
///
/// `events` 为 `[(Q_m3, qp_m3s), ...]` 列表。
/// 返回各事件的土壤流失量 (吨)。
pub fn musle_return_periods(
    events: &[(f64, f64)],
    k_factor: f64,
    ls_factor: f64,
    c_factor: f64,
    p_factor: f64,
) -> Vec<f64> {
    events
        .iter()
        .map(|(q, qp)| musle_event(*q, *qp, k_factor, ls_factor, c_factor, p_factor))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── compute_sdr ──

    #[test]
    fn test_compute_sdr_all_methods_1km2() {
        for m in &[
            SdrMethod::Usda,
            SdrMethod::Renfro,
            SdrMethod::Vanoni,
            SdrMethod::Boyce,
        ] {
            let sdr = compute_sdr(1.0, m);
            assert!(sdr.is_some());
            let v = sdr.unwrap();
            assert!(v > 0.0 && v <= 1.0, "method {:?} gave SDR={}", m, v);
        }
    }

    #[test]
    fn test_compute_sdr_large_area_smaller() {
        let small = compute_sdr(1.0, &SdrMethod::Usda).unwrap();
        let large = compute_sdr(1000.0, &SdrMethod::Usda).unwrap();
        assert!(
            large < small,
            "SDR at 1000km² should be smaller than at 1km²: {} vs {}",
            large,
            small
        );
        // Renfro 在任何实际面积下均 clamp 到 1.0，不参与此测试
        for (m, area_large) in &[
            (&SdrMethod::Vanoni, 1000.0_f64),
            (&SdrMethod::Boyce, 1000.0_f64),
        ] {
            let s = compute_sdr(1.0, m).unwrap();
            let l = compute_sdr(*area_large, m).unwrap();
            assert!(
                l < s,
                "method {:?}: SDR at {} ({}) should be < at 1km² ({})",
                m,
                area_large,
                l,
                s
            );
        }
    }

    #[test]
    fn test_compute_sdr_zero_area() {
        assert!(compute_sdr(0.0, &SdrMethod::Usda).is_none());
    }

    #[test]
    fn test_compute_sdr_negative_area() {
        assert!(compute_sdr(-5.0, &SdrMethod::Renfro).is_none());
    }

    // ── apply_sdr_to_rusle ──

    #[test]
    fn test_apply_sdr_produces_less_sediment() {
        let result = apply_sdr_to_rusle(100.0, 10.0, 10.0, SdrMethod::Usda).unwrap();
        assert!(
            result.sediment_yield_t_yr < result.source_soil_loss_t_yr,
            "sediment yield must be less than source loss"
        );
        assert_eq!(result.source_soil_loss_t_yr, 1000.0);
        assert!(result.sediment_yield_t_yr > 0.0);
        assert_eq!(result.drainage_area_km2, 10.0);
    }

    #[test]
    fn test_apply_sdr_zero_drainage() {
        assert!(apply_sdr_to_rusle(100.0, 10.0, 0.0, SdrMethod::Usda).is_none());
    }

    #[test]
    fn test_apply_sdr_negative_soil_loss() {
        assert!(apply_sdr_to_rusle(-1.0, 10.0, 10.0, SdrMethod::Usda).is_none());
    }

    #[test]
    fn test_apply_sdr_zero_area_ha() {
        assert!(apply_sdr_to_rusle(100.0, 0.0, 10.0, SdrMethod::Usda).is_none());
    }

    // ── musle_event ──

    #[test]
    fn test_musle_event_typical() {
        let loss = musle_event(1000.0, 10.0, 0.03, 2.0, 0.5, 1.0);
        let expected = 11.8 * (10000.0_f64.powf(0.56)) * 0.03 * 2.0 * 0.5 * 1.0;
        assert!(
            (loss - expected).abs() < 1e-6,
            "MUSLE: got {loss}, expected {expected}"
        );
    }

    #[test]
    fn test_musle_event_zero_runoff() {
        let loss = musle_event(0.0, 10.0, 0.03, 2.0, 0.5, 1.0);
        assert_eq!(loss, 0.0);
    }

    // ── musle_return_periods ──

    #[test]
    fn test_musle_return_periods_three_events() {
        let events = vec![(500.0, 5.0), (1000.0, 10.0), (2000.0, 20.0)];
        let results = musle_return_periods(&events, 0.03, 2.0, 0.5, 1.0);
        assert_eq!(results.len(), 3);
        for r in &results {
            assert!(*r > 0.0);
        }
        assert!(results[1] > results[0]);
        assert!(results[2] > results[1]);
    }

    #[test]
    fn test_musle_return_periods_empty() {
        let results = musle_return_periods(&[], 0.03, 2.0, 0.5, 1.0);
        assert!(results.is_empty());
    }

    // ── SdrMethod serde roundtrip ──

    #[test]
    fn test_sdr_method_serde_roundtrip() {
        let methods = [
            SdrMethod::Usda,
            SdrMethod::Renfro,
            SdrMethod::Vanoni,
            SdrMethod::Boyce,
        ];
        for m in &methods {
            let json = serde_json::to_string(m).unwrap();
            let de: SdrMethod = serde_json::from_str(&json).unwrap();
            let json2 = serde_json::to_string(&de).unwrap();
            assert_eq!(json, json2);
        }
    }

    // ── SdrResult fields ──

    #[test]
    fn test_sdr_result_all_fields() {
        let result = apply_sdr_to_rusle(50.0, 20.0, 5.0, SdrMethod::Vanoni).unwrap();
        assert_eq!(result.drainage_area_km2, 5.0);
        assert!(result.sdr > 0.0 && result.sdr <= 1.0);
        assert_eq!(result.source_soil_loss_t_yr, 1000.0);
        assert!(result.sediment_yield_t_yr < result.source_soil_loss_t_yr);
        match result.method {
            SdrMethod::Vanoni => {}
            _ => panic!("method mismatch"),
        }
    }
}

