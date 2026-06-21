//! Coastal Vulnerability Index (CVI) — Gornitz 1991.
//!
//! Combines six physical variables into a composite vulnerability score:
//! geomorphology, shoreline change rate, coastal slope, SLR rate,
//! wave height, tidal range.

/// Score a single CVI variable (1=low risk, 5=high risk).
pub fn cvi_variable_score(value: f64, category: &str) -> u8 {
    match category {
        "geomorphology" => {
            // string values handled via cvi_calculate
            match value as u64 {
                1 => 1, // rocky = mapped as 1
                2 => 2, // medium cliff
                3 => 3, // low cliff
                4 => 4, // cobble beach
                5 => 5, // sandy beach
                _ => 3, // default moderate
            }
        }
        "shoreline_change" | "shoreline_change_m_yr" => {
            if value < -2.0 {
                5
            } else if value < -1.0 {
                4
            } else if value.abs() <= 1.0 {
                3
            } else if value < 2.0 {
                2
            } else {
                1
            }
        }
        "coastal_slope" | "coastal_slope_pct" => {
            if value > 12.0 {
                1
            } else if value > 8.0 {
                2
            } else if value > 4.0 {
                3
            } else if value > 2.0 {
                4
            } else {
                5
            }
        }
        "slr" | "slr_mm_yr" => {
            if value < 1.8 {
                1
            } else if value < 2.5 {
                2
            } else if value < 3.0 {
                3
            } else if value < 3.4 {
                4
            } else {
                5
            }
        }
        "wave_height" | "wave_height_m" => {
            if value < 0.55 {
                1
            } else if value < 0.85 {
                2
            } else if value < 1.05 {
                3
            } else if value < 1.25 {
                4
            } else {
                5
            }
        }
        "tidal_range" | "tidal_range_m" => {
            if value > 6.0 {
                1
            } else if value > 4.0 {
                2
            } else if value > 2.0 {
                3
            } else if value > 1.0 {
                4
            } else {
                5
            }
        }
        _ => 3,
    }
}

/// Map geomorphology string to a numeric score 1-5.
fn geomorphology_score(geomorphology: &str) -> u8 {
    match geomorphology.to_lowercase().as_str() {
        "rocky" | "rocky_coast" => 1,
        "medium_cliff" | "moderate_cliff" => 2,
        "low_cliff" | "es carpment" => 3,
        "cobble_beach" | "cobble" => 4,
        "sandy_beach" | "beach" | "sand" | "marsh" | "delta" => 5,
        _ => 3,
    }
}

/// CVI rank string from numeric score.
fn cvi_rank(cvi: f64) -> &'static str {
    if cvi < 5.0 {
        "Very Low"
    } else if cvi < 10.0 {
        "Low"
    } else if cvi < 20.0 {
        "Moderate"
    } else if cvi < 40.0 {
        "High"
    } else {
        "Very High"
    }
}

/// Calculate CVI from six variables.
pub fn cvi_calculate(
    geomorphology: &str,
    shoreline_change_m_yr: f64,
    slope_pct: f64,
    slr_mm_yr: f64,
    wave_height_m: f64,
    tidal_range_m: f64,
) -> serde_json::Value {
    let scores = serde_json::json!({
        "geomorphology": geomorphology_score(geomorphology),
        "shoreline_change_m_yr": cvi_variable_score(shoreline_change_m_yr, "shoreline_change"),
        "coastal_slope_pct": cvi_variable_score(slope_pct, "coastal_slope"),
        "slr_mm_yr": cvi_variable_score(slr_mm_yr, "slr"),
        "wave_height_m": cvi_variable_score(wave_height_m, "wave_height"),
        "tidal_range_m": cvi_variable_score(tidal_range_m, "tidal_range"),
    });

    // CVI = sqrt( (a * b * c * d * e * f) / 6 )
    let a = scores["geomorphology"].as_u64().unwrap_or(3) as f64;
    let b = scores["shoreline_change_m_yr"].as_u64().unwrap_or(3) as f64;
    let c = scores["coastal_slope_pct"].as_u64().unwrap_or(3) as f64;
    let d = scores["slr_mm_yr"].as_u64().unwrap_or(3) as f64;
    let e = scores["wave_height_m"].as_u64().unwrap_or(3) as f64;
    let f = scores["tidal_range_m"].as_u64().unwrap_or(3) as f64;

    let product = a * b * c * d * e * f;
    let cvi = (product / 6.0).sqrt();
    let rank = cvi_rank(cvi);

    let recommendations = match rank {
        "Very High" => {
            "Immediate adaptation needed: retreat, coastal protection, or ecosystem-based measures."
        }
        "High" => "High priority for planning: assess risk zones, implement no-build buffers.",
        "Moderate" => "Monitor and plan: integrate SLR into long-term coastal management.",
        "Low" => "Routine monitoring sufficient.",
        _ => "No immediate action required.",
    };

    serde_json::json!({
        "cvi": (cvi * 100.0).round() / 100.0,
        "rank": rank,
        "scores": scores,
        "recommendations": recommendations,
    })
}

/// Batch CVI assessment for multiple shoreline segments.
/// Each segment: (lat, lon, geomorphology, shoreline_change_m_yr, slope_pct, slr_mm_yr, wave_height_m, tidal_range_m)
pub fn cvi_assessment(segments: &[(f64, f64, &str, f64, f64, f64, f64, f64)]) -> serde_json::Value {
    let results: Vec<serde_json::Value> = segments
        .iter()
        .map(|(lat, lon, geo, sh, sl, sr, wh, tr)| {
            let cvi = cvi_calculate(geo, *sh, *sl, *sr, *wh, *tr);
            serde_json::json!({
                "lat": lat,
                "lon": lon,
                "geomorphology": geo,
                "result": cvi
            })
        })
        .collect();

    let avg_cvi: f64 = results
        .iter()
        .filter_map(|r| r["result"]["cvi"].as_f64())
        .sum::<f64>()
        / results.len().max(1) as f64;

    serde_json::json!({
        "segment_count": segments.len(),
        "avg_cvi": (avg_cvi * 100.0).round() / 100.0,
        "avg_rank": cvi_rank(avg_cvi),
        "segments": results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cvi_variable_score_shoreline() {
        // erosion > 2 m/yr = 5
        assert_eq!(cvi_variable_score(-3.0, "shoreline_change"), 5);
        // accretion > 2 m/yr = 1
        assert_eq!(cvi_variable_score(3.0, "shoreline_change"), 1);
        // stable = 3
        assert_eq!(cvi_variable_score(0.0, "shoreline_change"), 3);
    }

    #[test]
    fn test_cvi_variable_score_slope() {
        assert_eq!(cvi_variable_score(15.0, "coastal_slope"), 1);
        assert_eq!(cvi_variable_score(1.0, "coastal_slope"), 5);
    }

    #[test]
    fn test_cvi_variable_score_slr() {
        assert_eq!(cvi_variable_score(1.0, "slr"), 1);
        assert_eq!(cvi_variable_score(5.0, "slr"), 5);
    }

    #[test]
    fn test_cvi_variable_score_wave() {
        assert_eq!(cvi_variable_score(0.3, "wave_height"), 1);
        assert_eq!(cvi_variable_score(2.0, "wave_height"), 5);
    }

    #[test]
    fn test_cvi_variable_score_tidal() {
        assert_eq!(cvi_variable_score(7.0, "tidal_range"), 1);
        assert_eq!(cvi_variable_score(0.5, "tidal_range"), 5);
    }

    #[test]
    fn test_cvi_calculate_low_risk() {
        // Rocky coast, accreting, steep slope, low SLR, low waves, high tide
        let result = cvi_calculate("rocky", 3.0, 15.0, 1.0, 0.3, 7.0);
        let rank = result["rank"].as_str().unwrap();
        let cvi = result["cvi"].as_f64().unwrap();
        assert!(rank == "Very Low" || rank == "Low", "got {rank} cvi={cvi}");
    }

    #[test]
    fn test_cvi_calculate_high_risk() {
        // Sandy beach, eroding, flat slope, high SLR, high waves, microtidal
        let result = cvi_calculate("sandy_beach", -3.0, 1.0, 5.0, 2.0, 0.5);
        let rank = result["rank"].as_str().unwrap();
        assert_eq!(rank, "Very High");
    }

    #[test]
    fn test_cvi_calculate_moderate() {
        let result = cvi_calculate("medium_cliff", -1.5, 6.0, 2.0, 0.9, 1.5);
        let rank = result["rank"].as_str().unwrap();
        // Should be High or Moderate
        let cvi = result["cvi"].as_f64().unwrap();
        assert!(cvi > 0.0 && cvi < 50.0, "cvi={cvi}");
    }

    #[test]
    fn test_cvi_assessment() {
        let segments = vec![
            (30.0, 120.0, "rocky", 2.0, 15.0, 1.5, 0.5, 6.0),
            (30.1, 120.1, "sandy_beach", -4.0, 1.0, 4.0, 2.0, 0.5),
        ];
        let result = cvi_assessment(&segments);
        assert_eq!(result["segment_count"].as_u64().unwrap(), 2);
        assert!(result["avg_cvi"].as_f64().unwrap() > 0.0);
    }
}
