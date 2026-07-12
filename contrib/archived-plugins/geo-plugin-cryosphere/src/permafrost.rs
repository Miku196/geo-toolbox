//! 冻土 — 活动层厚度、冻融指数
use serde::{Deserialize, Serialize};

/// 冻融指数结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrostIndex {
    pub freezing_index: f64,
    pub thawing_index: f64,
    pub active_layer_thickness_cm: f64,
}

/// Stefan 解: 活动层厚度 = K × √(I)
pub fn active_layer_thickness_stefan(
    thawing_degree_days: f64,
    thermal_conductivity: f64,
    ice_content: f64,
) -> f64 {
    if thawing_degree_days <= 0.0 {
        return 0.0;
    }
    let k = thermal_conductivity;
    let i = thawing_degree_days * 86400.0;
    let l = 334000.0 * 917.0;
    let z = (2.0 * k * i / (l * ice_content.max(0.01))).sqrt();
    z * 100.0
}

/// 冻融指数 (freezing/thawing degree days)
pub fn freeze_thaw_index(daily_temp_c: &[f64]) -> FrostIndex {
    let fi: f64 = daily_temp_c
        .iter()
        .map(|&t| if t < 0.0 { -t } else { 0.0 })
        .sum();
    let ti: f64 = daily_temp_c
        .iter()
        .map(|&t| if t > 0.0 { t } else { 0.0 })
        .sum();
    FrostIndex {
        freezing_index: fi,
        thawing_index: ti,
        active_layer_thickness_cm: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stefan_thickness() {
        let alt = active_layer_thickness_stefan(800.0, 1.5, 0.3);
        assert!(alt > 0.0 && alt < 500.0);
    }

    #[test]
    fn test_zero_thawing() {
        assert_eq!(active_layer_thickness_stefan(0.0, 1.5, 0.3), 0.0);
    }

    #[test]
    fn test_freeze_thaw_index() {
        let temps = vec![-10.0, -5.0, 0.0, 5.0, 10.0];
        let fi = freeze_thaw_index(&temps);
        assert!((fi.freezing_index - 15.0).abs() < 1e-6);
        assert!((fi.thawing_index - 15.0).abs() < 1e-6);
    }
}
