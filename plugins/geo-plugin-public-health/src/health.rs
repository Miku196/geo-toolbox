//! Environmental public health: UHI, PM2.5 exposure, disease vectors, risk indices.
use geo_core::errors::GeoResult;
use serde::Serialize;

// ── Urban Heat Island ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct UhiIndex {
    pub temperature_c: f64,
    pub rural_reference_c: f64,
    pub uhi_intensity_c: f64,
    pub risk_level: String,
    pub health_advice: String,
}

/// Urban Heat Island intensity = urban temperature - rural reference.
pub fn uhi_intensity(urban_temp_c: f64, rural_temp_c: f64) -> UhiIndex {
    let intensity = urban_temp_c - rural_temp_c;
    let (risk, advice) = if intensity < 2.0 {
        ("Low", "No special precautions")
    } else if intensity < 5.0 {
        (
            "Moderate",
            "Vulnerable groups should limit outdoor activity",
        )
    } else if intensity < 8.0 {
        ("High", "Heat stress risk — seek cooling")
    } else {
        ("Extreme", "Dangerous heat — avoid outdoor exposure")
    };
    UhiIndex {
        temperature_c: urban_temp_c,
        rural_reference_c: rural_temp_c,
        uhi_intensity_c: intensity,
        risk_level: risk.to_string(),
        health_advice: advice.to_string(),
    }
}

// ── PM2.5 Exposure ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct Pm25Exposure {
    pub pm25_ug_m3: f64,
    pub aqi: u32,
    pub aqi_category: String,
    pub annual_exposure_dose_mg: f64,
}

/// US EPA AQI from PM2.5 (24-hour average in μg/m³).
pub fn pm25_aqi(pm25_ug_m3: f64) -> u32 {
    if pm25_ug_m3 <= 12.0 {
        (pm25_ug_m3 / 12.0 * 50.0) as u32
    } else if pm25_ug_m3 <= 35.4 {
        (50.0 + (pm25_ug_m3 - 12.0) / (35.4 - 12.0) * 50.0) as u32
    } else if pm25_ug_m3 <= 55.4 {
        (100.0 + (pm25_ug_m3 - 35.4) / (55.4 - 35.4) * 50.0) as u32
    } else if pm25_ug_m3 <= 150.4 {
        (150.0 + (pm25_ug_m3 - 55.4) / (150.4 - 55.4) * 100.0) as u32
    } else if pm25_ug_m3 <= 250.4 {
        (200.0 + (pm25_ug_m3 - 150.4) / (250.4 - 150.4) * 100.0) as u32
    } else {
        (300.0 + (pm25_ug_m3 - 250.4) / (500.4 - 250.4) * 200.0).min(500.0) as u32
    }
}

pub fn pm25_exposure(
    pm25_ug_m3: f64,
    breathing_rate_m3_day: f64,
    days_exposed: f64,
) -> Pm25Exposure {
    let aqi = pm25_aqi(pm25_ug_m3);
    let cat = if aqi <= 50 {
        "Good"
    } else if aqi <= 100 {
        "Moderate"
    } else if aqi <= 150 {
        "Unhealthy for Sensitive Groups"
    } else if aqi <= 200 {
        "Unhealthy"
    } else if aqi <= 300 {
        "Very Unhealthy"
    } else {
        "Hazardous"
    };
    let dose_mg = pm25_ug_m3 * breathing_rate_m3_day * days_exposed / 1000.0;
    Pm25Exposure {
        pm25_ug_m3,
        aqi,
        aqi_category: cat.to_string(),
        annual_exposure_dose_mg: (dose_mg * 1000.0).round() / 1000.0,
    }
}

// ── Disease Vector Suitability ───────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct VectorSuitability {
    pub vector_type: String,
    pub temperature_suitability: f64,
    pub precipitation_suitability: f64,
    pub overall_suitability: f64,
    pub risk_period_months: u32,
}

/// Disease vector habitat suitability (e.g., mosquitoes, ticks).
/// Based on temperature and precipitation thresholds.
pub fn vector_suitability(temp_c: f64, precip_mm_month: f64, vector: &str) -> VectorSuitability {
    let (t_min, t_opt, t_max, p_min) = match vector {
        "aedes" => (15.0, 28.0, 35.0, 50.0), // Aedes aegypti (dengue/Zika)
        "anopheles" => (18.0, 25.0, 32.0, 80.0), // Anopheles (malaria)
        "culex" => (10.0, 25.0, 35.0, 30.0), // Culex (West Nile)
        "ixodes" => (5.0, 18.0, 30.0, 60.0), // Ixodes tick (Lyme)
        _ => (10.0, 25.0, 35.0, 30.0),
    };
    let t_suit = if temp_c < t_min || temp_c > t_max {
        0.0
    } else if temp_c <= t_opt {
        (temp_c - t_min) / (t_opt - t_min)
    } else {
        (t_max - temp_c) / (t_max - t_opt)
    };
    let p_suit = (precip_mm_month / p_min).min(1.0);
    VectorSuitability {
        vector_type: vector.to_string(),
        temperature_suitability: (t_suit * 100.0).round() / 100.0,
        precipitation_suitability: (p_suit * 100.0).round() / 100.0,
        overall_suitability: ((t_suit * 0.6 + p_suit * 0.4) * 100.0).round() / 100.0,
        risk_period_months: if precip_mm_month >= p_min && t_suit > 0.5 {
            6
        } else if t_suit > 0.3 {
            3
        } else {
            0
        },
    }
}

// ── Environmental Health Risk Index ──────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct HealthRiskIndex {
    pub air_quality_score: f64,
    pub heat_stress_score: f64,
    pub water_quality_score: f64,
    pub overall_index: f64,
    pub risk_category: String,
}

/// Composite Environmental Health Risk Index (0-100, higher = worse).
pub fn health_risk_index(
    pm25_ug_m3: f64,
    max_temp_c: f64,
    water_quality_index: f64,
) -> HealthRiskIndex {
    let air = (pm25_ug_m3 / 100.0 * 100.0).min(100.0);
    let heat = if max_temp_c < 25.0 {
        0.0
    } else if max_temp_c < 35.0 {
        (max_temp_c - 25.0) / 10.0 * 50.0
    } else if max_temp_c < 40.0 {
        50.0 + (max_temp_c - 35.0) / 5.0 * 30.0
    } else {
        80.0 + (max_temp_c - 40.0) / 10.0 * 20.0
    }
    .min(100.0);
    let water = (100.0 - water_quality_index).max(0.0);
    let overall = air * 0.35 + heat * 0.4 + water * 0.25;
    let cat = if overall < 20.0 {
        "Low risk"
    } else if overall < 40.0 {
        "Moderate"
    } else if overall < 60.0 {
        "Elevated"
    } else if overall < 80.0 {
        "High risk"
    } else {
        "Severe"
    };
    HealthRiskIndex {
        air_quality_score: air,
        heat_stress_score: heat,
        water_quality_score: water,
        overall_index: (overall * 10.0).round() / 10.0,
        risk_category: cat.to_string(),
    }
}

// ── Plugin ───────────────────────────────────────────────────────
pub struct HealthPlugin;
impl HealthPlugin {
    pub fn uhi(&self, u: f64, r: f64) -> UhiIndex {
        uhi_intensity(u, r)
    }
    pub fn pm25(&self, p: f64, br: f64, d: f64) -> Pm25Exposure {
        pm25_exposure(p, br, d)
    }
    pub fn vector(&self, t: f64, p: f64, v: &str) -> VectorSuitability {
        vector_suitability(t, p, v)
    }
    pub fn risk(&self, pm: f64, t: f64, wq: f64) -> HealthRiskIndex {
        health_risk_index(pm, t, wq)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_uhi() {
        let r = uhi_intensity(38.0, 30.0);
        assert!(r.uhi_intensity_c > 5.0);
    }
    #[test]
    fn test_pm25() {
        let r = pm25_aqi(35.0);
        assert!(r > 50);
    }
    #[test]
    fn test_vector() {
        let r = vector_suitability(26.0, 60.0, "aedes");
        assert!(r.overall_suitability > 0.5);
    }
    #[test]
    fn test_risk() {
        let r = health_risk_index(50.0, 38.0, 70.0);
        assert!(r.overall_index > 30.0);
    }
}
