//! NRCS TR-55 Urban Hydrology — Small Watershed Hydrology.
//!
//! Implements:
//! - Expanded CN lookup table (TR-55 Table 2-2a/b/c)
//! - Time of concentration (sheet, shallow, channel flow)
//! - Peak discharge computation (graphical method regression)
//! - Full assessment pipeline

use serde::Serialize;

/// Soil hydrologic groups A/B/C/D
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tr55SoilGroup {
    A,
    B,
    C,
    D,
}

impl Tr55SoilGroup {
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "A" => Tr55SoilGroup::A,
            "B" => Tr55SoilGroup::B,
            "C" => Tr55SoilGroup::C,
            _ => Tr55SoilGroup::D,
        }
    }
}

/// Rainfall distribution type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RainfallType {
    I,
    II,
    III,
}

impl RainfallType {
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "I" => RainfallType::I,
            "III" => RainfallType::III,
            _ => RainfallType::II,
        }
    }
}

/// Look up TR-55 CN for given landuse and soil group.
///
/// Expanded table from NRCS TR-55 (1986), Tables 2-2a, 2-2b, 2-2c.
/// Returns the CN value for AMC II (normal conditions).
pub fn tr55_cn_lookup(landuse: &str, soil_group: &str) -> Option<u8> {
    let sg = Tr55SoilGroup::from_str(soil_group);
    let cn = match landuse.to_lowercase().as_str() {
        "woods_good" | "woods-good" => match sg {
            Tr55SoilGroup::A => 30,
            Tr55SoilGroup::B => 55,
            Tr55SoilGroup::C => 70,
            Tr55SoilGroup::D => 77,
        },
        "woods_poor" | "woods-poor" => match sg {
            Tr55SoilGroup::A => 45,
            Tr55SoilGroup::B => 66,
            Tr55SoilGroup::C => 77,
            Tr55SoilGroup::D => 83,
        },
        "residential_1acre+" | "residential-1acre" => match sg {
            Tr55SoilGroup::A => 51,
            Tr55SoilGroup::B => 68,
            Tr55SoilGroup::C => 79,
            Tr55SoilGroup::D => 84,
        },
        "residential_1_2acre" | "residential-half-acre" => match sg {
            Tr55SoilGroup::A => 54,
            Tr55SoilGroup::B => 70,
            Tr55SoilGroup::C => 80,
            Tr55SoilGroup::D => 85,
        },
        "residential_1_4acre" | "residential-quarter-acre" => match sg {
            Tr55SoilGroup::A => 61,
            Tr55SoilGroup::B => 75,
            Tr55SoilGroup::C => 83,
            Tr55SoilGroup::D => 87,
        },
        "residential_1_8acre" | "residential-eighth-acre" => match sg {
            Tr55SoilGroup::A => 77,
            Tr55SoilGroup::B => 85,
            Tr55SoilGroup::C => 90,
            Tr55SoilGroup::D => 92,
        },
        "commercial" => match sg {
            Tr55SoilGroup::A => 89,
            Tr55SoilGroup::B => 92,
            Tr55SoilGroup::C => 94,
            Tr55SoilGroup::D => 95,
        },
        "industrial" => match sg {
            Tr55SoilGroup::A => 81,
            Tr55SoilGroup::B => 88,
            Tr55SoilGroup::C => 91,
            Tr55SoilGroup::D => 93,
        },
        "parking" | "parking_lot" => match sg {
            Tr55SoilGroup::A => 98,
            Tr55SoilGroup::B => 98,
            Tr55SoilGroup::C => 98,
            Tr55SoilGroup::D => 98,
        },
        "streets_roads" | "streets/roads" => match sg {
            Tr55SoilGroup::A => 98,
            Tr55SoilGroup::B => 98,
            Tr55SoilGroup::C => 98,
            Tr55SoilGroup::D => 98,
        },
        "open_space_good" | "open-space-good" => match sg {
            Tr55SoilGroup::A => 39,
            Tr55SoilGroup::B => 61,
            Tr55SoilGroup::C => 74,
            Tr55SoilGroup::D => 80,
        },
        "open_space_fair" | "open-space-fair" => match sg {
            Tr55SoilGroup::A => 49,
            Tr55SoilGroup::B => 69,
            Tr55SoilGroup::C => 79,
            Tr55SoilGroup::D => 84,
        },
        "farmsteads" => match sg {
            Tr55SoilGroup::A => 59,
            Tr55SoilGroup::B => 74,
            Tr55SoilGroup::C => 82,
            Tr55SoilGroup::D => 86,
        },
        "meadow" => match sg {
            Tr55SoilGroup::A => 30,
            Tr55SoilGroup::B => 58,
            Tr55SoilGroup::C => 71,
            Tr55SoilGroup::D => 78,
        },
        _ => return None,
    };
    Some(cn)
}

/// TR-55 Eq 3-3: Sheet flow travel time (minutes).
///
/// Tt = 0.007 * (n * L_f) ^ 0.8 / (P2 ^ 0.5 * s ^ 0.4)
pub fn tr55_time_of_concentration_sheet_flow(
    length_m: f64,
    slope_pct: f64,
    manning_n: f64,
    rainfall_2yr_24h_mm: f64,
) -> f64 {
    let slope_abs = slope_pct / 100.0;
    let numerator = (manning_n * length_m).powf(0.8);
    let denominator = rainfall_2yr_24h_mm.powf(0.5) * slope_abs.powf(0.4);
    if denominator < 1e-10 {
        return 999.0;
    }
    0.007 * numerator / denominator
}

/// Shallow concentrated flow travel time (minutes).
///
/// Velocity formulas from TR-55 Fig 3-1.
pub fn tr55_time_of_concentration_shallow_flow(
    length_m: f64,
    slope_pct: f64,
    surface_type: &str,
) -> f64 {
    let slope_abs = slope_pct / 100.0;
    let v = match surface_type.to_lowercase().as_str() {
        "paved" => 6.1961 * slope_abs.sqrt(),
        "unpaved" => 4.9178 * slope_abs.sqrt(),
        _ => 4.9178 * slope_abs.sqrt(),
    };
    if v < 1e-10 {
        return 999.0;
    }
    length_m / (60.0 * v)
}

/// Channel flow travel time (minutes) via Manning's equation.
pub fn tr55_time_of_concentration_channel_flow(
    length_m: f64,
    slope: f64,
    cross_area_m2: f64,
    wetted_perimeter_m: f64,
    manning_n: f64,
) -> f64 {
    if wetted_perimeter_m < 1e-10 || manning_n < 1e-10 {
        return 999.0;
    }
    let r = cross_area_m2 / wetted_perimeter_m;
    let v = (1.0 / manning_n) * r.powf(2.0 / 3.0) * slope.sqrt();
    if v < 1e-10 {
        return 999.0;
    }
    length_m / (60.0 * v)
}

/// Generic travel time from flow length, slope, and velocity.
pub fn tr55_travel_time(flow_length_m: f64, slope_pct: f64, velocity_m_s: f64) -> f64 {
    if velocity_m_s < 1e-10 {
        return 999.0;
    }
    flow_length_m / (60.0 * velocity_m_s)
}

/// TR-55 peak discharge computation (graphical method).
///
/// Uses simplified regression for Type II rainfall.
pub fn tr55_peak_discharge(
    runoff_mm: f64,
    area_km2: f64,
    tc_hrs: f64,
    rainfall_type: &str,
) -> serde_json::Value {
    let q_cm = runoff_mm / 10.0; // mm → cm
    let area_mi2 = area_km2 * 0.386102; // km² → mi²

    // Ia/P ratio
    let ia_ratio = if q_cm > 0.01 {
        0.2 / (q_cm / runoff_mm)
    } else {
        0.0
    };
    let _ia_p = ia_ratio.min(0.5);

    // Simplified regression for unit peak discharge qu (csm/in)
    // Based on TR-55 Exhibit 4-II for Type II
    let qu = match RainfallType::from_str(rainfall_type) {
        RainfallType::II => {
            if tc_hrs <= 0.1 {
                68.0
            } else {
                let ln_tc = tc_hrs.ln();
                (2.535 - 0.686 * ln_tc - 0.333 * ln_tc * ln_tc).exp()
            }
        }
        RainfallType::I => {
            if tc_hrs <= 0.1 {
                60.0
            } else {
                20.0 / tc_hrs.powf(0.8)
            }
        }
        RainfallType::III => {
            if tc_hrs <= 0.1 {
                72.0
            } else {
                24.0 / tc_hrs.powf(0.75)
            }
        }
    };

    // Pond/swamp factor (default no adjustment)
    let fp = 1.0;
    let qp = qu * area_mi2 * q_cm * fp; // peak discharge in cfs
    let qp_m3s = qp * 0.0283168; // cfs → m³/s

    serde_json::json!({
        "peak_discharge_cfs": (qp * 100.0).round() / 100.0,
        "peak_discharge_m3_s": (qp_m3s * 1000.0).round() / 1000.0,
        "unit_peak_discharge_csm_in": (qu * 100.0).round() / 100.0,
        "time_of_concentration_hrs": tc_hrs,
        "runoff_cm": (q_cm * 100.0).round() / 100.0,
        "area_km2": area_km2,
        "pond_swamp_factor": fp,
        "rainfall_type": rainfall_type
    })
}

/// Full TR-55 assessment: CN lookup → runoff → Tc → peak discharge.
#[derive(Debug, Clone, Serialize)]
pub struct Tr55Assessment {
    pub weighted_cn: f64,
    pub runoff_mm: f64,
    pub time_of_concentration_min: f64,
    pub peak_discharge_m3_s: f64,
    pub peak_discharge_cfs: f64,
    pub area_km2: f64,
    pub rainfall_mm: f64,
    pub rainfall_type: String,
}

/// Complete TR-55 pipeline.
pub fn tr55_full_assessment(
    landuse: &[&str],
    soil_group: &[&str],
    rainfall_mm: f64,
    area_km2: f64,
    flow_lengths: &[f64],
    slopes_pct: &[f64],
    rainfall_type: &str,
) -> Tr55Assessment {
    // Weighted CN
    let mut sum_cn = 0.0;
    let mut count = 0;
    for (lu, sg) in landuse.iter().zip(soil_group.iter()) {
        if let Some(cn) = tr55_cn_lookup(lu, sg) {
            sum_cn += cn as f64;
            count += 1;
        }
    }
    let weighted_cn = if count > 0 {
        sum_cn / count as f64
    } else {
        70.0
    };

    // Runoff via SCS equation
    let s = 25400.0 / weighted_cn - 254.0;
    let ia = 0.2 * s;
    let runoff_mm = if rainfall_mm > ia {
        let num = (rainfall_mm - ia).powi(2);
        let den = rainfall_mm - ia + s;
        if den > 0.0 {
            num / den
        } else {
            0.0
        }
    } else {
        0.0
    };

    // Time of concentration — sum of travel times
    let mut tc_min = 0.0;
    let default_slopes = if slopes_pct.is_empty() {
        &[1.0]
    } else {
        slopes_pct
    };
    for i in 0..flow_lengths.len() {
        let sl = if i < default_slopes.len() {
            default_slopes[i]
        } else {
            default_slopes.last().copied().unwrap_or(1.0)
        };
        // Use shallow concentrated flow as default
        let tt = tr55_time_of_concentration_shallow_flow(flow_lengths[i], sl, "unpaved");
        tc_min += tt;
    }

    let tc_hrs = tc_min / 60.0;
    let peak = tr55_peak_discharge(runoff_mm, area_km2, tc_hrs, rainfall_type);

    Tr55Assessment {
        weighted_cn,
        runoff_mm: (runoff_mm * 10.0).round() / 10.0,
        time_of_concentration_min: (tc_min * 10.0).round() / 10.0,
        peak_discharge_m3_s: peak["peak_discharge_m3_s"].as_f64().unwrap_or(0.0),
        peak_discharge_cfs: peak["peak_discharge_cfs"].as_f64().unwrap_or(0.0),
        area_km2,
        rainfall_mm,
        rainfall_type: rainfall_type.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cn_lookup() {
        assert_eq!(tr55_cn_lookup("woods_good", "A"), Some(30));
        assert_eq!(tr55_cn_lookup("woods_good", "D"), Some(77));
        assert_eq!(tr55_cn_lookup("commercial", "C"), Some(94));
        assert_eq!(tr55_cn_lookup("parking", "A"), Some(98));
        assert_eq!(tr55_cn_lookup("meadow", "B"), Some(58));
        assert_eq!(tr55_cn_lookup("unknown_use", "A"), None);
    }

    #[test]
    fn test_cn_lookup_all() {
        for lu in &[
            "woods_good",
            "woods_poor",
            "residential_1acre+",
            "residential_1_2acre",
            "residential_1_4acre",
            "residential_1_8acre",
            "commercial",
            "industrial",
            "parking",
            "streets_roads",
            "open_space_good",
            "open_space_fair",
            "farmsteads",
            "meadow",
        ] {
            for sg in &["A", "B", "C", "D"] {
                let cn = tr55_cn_lookup(lu, sg);
                assert!(cn.is_some(), "Failed for {} on soil {}", lu, sg);
                let cn_val = cn.unwrap();
                assert!(
                    cn_val >= 30 && cn_val <= 98,
                    "CN {} out of range for {}/{}",
                    cn_val,
                    lu,
                    sg
                );
            }
        }
    }

    #[test]
    fn test_sheet_flow_tc() {
        let tc = tr55_time_of_concentration_sheet_flow(30.0, 2.0, 0.011, 50.0);
        assert!(tc > 0.0);
        assert!(tc < 60.0);
    }

    #[test]
    fn test_shallow_flow_tc() {
        let tc = tr55_time_of_concentration_shallow_flow(100.0, 3.0, "paved");
        assert!(tc > 0.0);
        assert!(tc < 30.0);
    }

    #[test]
    fn test_channel_flow_tc() {
        let tc = tr55_time_of_concentration_channel_flow(500.0, 0.001, 10.0, 8.0, 0.035);
        assert!(tc > 0.0);
        assert!(tc < 30.0);
    }

    #[test]
    fn test_travel_time() {
        let tt = tr55_travel_time(200.0, 2.0, 1.5);
        assert!((tt - 2.22).abs() < 0.1);
    }

    #[test]
    fn test_peak_discharge_type_ii() {
        let peak = tr55_peak_discharge(25.0, 1.0, 0.5, "II");
        assert!(peak["peak_discharge_cfs"].as_f64().unwrap() > 0.0);
        assert!(peak["peak_discharge_m3_s"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn test_full_assessment() {
        let landuse = vec!["woods_good", "woods_good", "meadow", "commercial"];
        let soil = vec!["B", "B", "B", "B"];
        let flows = vec![50.0, 100.0];
        let slopes = vec![3.0, 2.0];
        let result = tr55_full_assessment(&landuse, &soil, 80.0, 0.5, &flows, &slopes, "II");
        assert!(result.weighted_cn > 0.0);
        assert!(result.time_of_concentration_min > 0.0);
        assert!(result.runoff_mm >= 0.0);
        assert!(result.runoff_mm <= 80.0);
    }

    #[test]
    fn test_full_assessment_no_runoff() {
        let landuse = vec!["woods_good", "meadow"];
        let soil = vec!["A", "A"];
        let flows = vec![50.0];
        let slopes = vec![1.0];
        let result = tr55_full_assessment(&landuse, &soil, 5.0, 0.1, &flows, &slopes, "II");
        // Very low rainfall with good infiltration → little or no runoff
        assert!(result.runoff_mm <= 5.0);
    }
}
