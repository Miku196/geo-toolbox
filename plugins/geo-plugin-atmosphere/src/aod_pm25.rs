use serde::{Deserialize, Serialize};

/// AOD→PM2.5 反演结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AodPm25Result {
    pub pm25_ug_m3: Vec<f64>,
    pub pm25_mean: f64,
    pub pm25_max: f64,
    pub pm25_min: f64,
    pub aqi_values: Vec<u32>,
    pub aqi_mean: u32,
    pub aqi_classification: String,
}

/// AQI 等级。
fn aqi_from_pm25(pm25: f64) -> (u32, &'static str) {
    if pm25 <= 12.0 {
        ((pm25 / 12.0 * 50.0) as u32, "Good")
    } else if pm25 <= 35.4 {
        (
            (50.0 + (pm25 - 12.0) / (35.4 - 12.0) * 50.0) as u32,
            "Moderate",
        )
    } else if pm25 <= 55.4 {
        (
            (100.0 + (pm25 - 35.4) / (55.4 - 35.4) * 50.0) as u32,
            "Unhealthy for Sensitive Groups",
        )
    } else if pm25 <= 150.4 {
        (
            (150.0 + (pm25 - 55.4) / (150.4 - 55.4) * 100.0) as u32,
            "Unhealthy",
        )
    } else if pm25 <= 250.4 {
        (
            (200.0 + (pm25 - 150.4) / (250.4 - 150.4) * 100.0) as u32,
            "Very Unhealthy",
        )
    } else {
        (
            (300.0 + ((pm25 - 250.4) / (500.4 - 250.4) * 200.0).min(200.0)) as u32,
            "Hazardous",
        )
    }
}

/// AOD550 → PM2.5 浓度 (μg/m³)。
///
/// PM2.5 = AOD550 × ratio × RH_correction
pub fn aod_to_pm25(aod_values: &[f64], aod550_pm25_ratio: f64, rh_correction: f64) -> Vec<f64> {
    aod_values
        .iter()
        .map(|&aod| aod * aod550_pm25_ratio * rh_correction)
        .collect()
}

/// PM2.5 → AQI 转换。
pub fn pm25_to_aqi(pm25_values: &[f64]) -> (Vec<u32>, f64, String) {
    let aqis: Vec<u32> = pm25_values.iter().map(|&p| aqi_from_pm25(p).0).collect();
    let mean_aqi = if aqis.is_empty() {
        0
    } else {
        aqis.iter().sum::<u32>() / aqis.len() as u32
    };
    let mean_pm25 = if pm25_values.is_empty() {
        0.0
    } else {
        pm25_values.iter().sum::<f64>() / pm25_values.len() as f64
    };
    let (_, classification) = aqi_from_pm25(mean_pm25);
    (aqis, mean_aqi as f64, classification.to_string())
}

/// 季节校正系数 (冬季高/夏季低)。
pub fn seasonal_correction(season: &str) -> f64 {
    match season.to_lowercase().as_str() {
        "winter" | "djf" => 1.3,
        "spring" | "mam" => 1.1,
        "summer" | "jja" => 0.8,
        "autumn" | "son" => 1.0,
        _ => 1.0,
    }
}

/// 完整 AOD→PM2.5→AQI 管线。
pub fn aod_pm25_pipeline(
    aod_values: &[f64],
    aod550_pm25_ratio: f64,
    rh_correction: f64,
    season: &str,
) -> AodPm25Result {
    let season_corr = seasonal_correction(season);
    let pm25 = aod_to_pm25(aod_values, aod550_pm25_ratio * season_corr, rh_correction);
    let (aqis, aqi_mean, classification) = pm25_to_aqi(&pm25);

    let (pm25_min, pm25_max) = if pm25.is_empty() {
        (0.0, 0.0)
    } else {
        let min = pm25.iter().copied().fold(f64::INFINITY, f64::min);
        let max = pm25.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        (min, max)
    };
    let pm25_mean = if pm25.is_empty() {
        0.0
    } else {
        pm25.iter().sum::<f64>() / pm25.len() as f64
    };

    AodPm25Result {
        pm25_ug_m3: pm25,
        pm25_mean,
        pm25_max,
        pm25_min,
        aqi_values: aqis,
        aqi_mean: aqi_mean as u32,
        aqi_classification: classification,
    }
}

// ═══════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aod_to_pm25_basic() {
        let aods = vec![0.2, 0.5, 1.0];
        let pm25 = aod_to_pm25(&aods, 0.55, 0.85);
        assert_eq!(pm25.len(), 3);
        assert!((pm25[0] - 0.0935).abs() < 1e-4);
    }

    #[test]
    fn test_pm25_to_aqi_good() {
        let (_, mean, _) = pm25_to_aqi(&[5.0, 10.0]);
        assert!(mean < 50.0, "aqi mean={}", mean);
    }

    #[test]
    fn test_pm25_to_aqi_hazardous() {
        let (aqis, _, class) = pm25_to_aqi(&[300.0]);
        assert_eq!(class, "Hazardous");
    }

    #[test]
    fn test_aqi_from_pm25() {
        let (aqi, label) = aqi_from_pm25(5.0);
        assert_eq!(label, "Good");
        assert!(aqi <= 50);
    }

    #[test]
    fn test_seasonal_correction_summer() {
        let corr = seasonal_correction("summer");
        assert!((corr - 0.8).abs() < 1e-4);
    }

    #[test]
    fn test_seasonal_correction_winter() {
        let corr = seasonal_correction("winter");
        assert!((corr - 1.3).abs() < 1e-4);
    }

    #[test]
    fn test_full_pipeline() {
        let aods = vec![0.1, 0.3, 0.6, 1.2, 2.0];
        let result = aod_pm25_pipeline(&aods, 0.55, 0.85, "summer");
        assert_eq!(result.pm25_ug_m3.len(), 5);
        assert!(result.pm25_mean > 0.0);
        assert!(result.aqi_mean > 0);
        assert!(!result.aqi_classification.is_empty());
    }

    #[test]
    fn test_empty_input() {
        let result = aod_pm25_pipeline(&[], 0.55, 0.85, "annual");
        assert_eq!(result.pm25_ug_m3.len(), 0);
        assert_eq!(result.pm25_mean, 0.0);
        assert_eq!(result.aqi_mean, 0);
    }

    #[test]
    fn test_single_value() {
        let result = aod_pm25_pipeline(&[0.5], 0.55, 0.85, "spring");
        assert_eq!(result.pm25_ug_m3.len(), 1);
        assert!(result.pm25_ug_m3[0] > 0.0);
    }
}
