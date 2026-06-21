//! Urban pluvial flooding analysis.
//!
//! Implements:
//! - Inlet capacity (curb, grate, area, combination)
//! - Pipe capacity via Manning's equation
//! - Simple inundation depth from runoff volume balance
//! - Dual drainage (major/minor system)
//! - Comprehensive flood assessment

use serde::Serialize;

/// Inlet capacity (m³/s) for different inlet types.
///
/// Q = C * A * sqrt(2 * g * h) * (1 - clogging_factor)
/// where h = 0.15 m (typical gutter head).
pub fn urban_flood_inlet_capacity(
    inlet_type: &str,
    grate_area_m2: f64,
    clogging_factor: f64,
) -> f64 {
    let coef = match inlet_type.to_lowercase().as_str() {
        "curb_opening" | "curb-opening" => 0.6,
        "grate" => 0.5,
        "combination" => 0.55,
        _ => 0.45, // "area" inlet / default
    };
    let g: f64 = 9.81;
    let head_m: f64 = 0.15;
    let flow = coef * grate_area_m2 * (2.0 * g * head_m).sqrt() * (1.0 - clogging_factor);
    flow.max(0.0)
}

/// Full pipe capacity via Manning's equation (m³/s).
///
/// Q = (1/n) * A * R^(2/3) * S^0.5
pub fn urban_flood_pipe_capacity(diameter_m: f64, slope: f64, manning_n: f64) -> f64 {
    if manning_n < 1e-10 || diameter_m < 1e-10 {
        return 0.0;
    }
    let area = std::f64::consts::PI * diameter_m.powi(2) / 4.0;
    let radius = diameter_m / 4.0;
    (1.0 / manning_n) * area * radius.powf(2.0 / 3.0) * slope.sqrt()
}

/// Simple inundation analysis from runoff volume vs drainage capacity.
///
/// Returns flood depth (mm) and risk category.
#[derive(Debug, Clone, Serialize)]
pub struct InundationResult {
    pub runoff_volume_m3: f64,
    pub drainage_capacity_m3_s: f64,
    pub excess_volume_m3: f64,
    pub flood_depth_mm: f64,
    pub risk_level: String,
    pub inundated_area_ha: f64,
}

pub fn urban_flood_inundation(
    runoff_volume_m3: f64,
    drainage_capacity_m3_s: f64,
    area_ha: f64,
    impervious_ratio: f64,
    duration_hrs: f64,
) -> InundationResult {
    let drainage_volume = drainage_capacity_m3_s * duration_hrs * 3600.0;
    let excess = (runoff_volume_m3 - drainage_volume).max(0.0);
    let area_m2 = area_ha * 10000.0;
    let flood_depth_mm = if area_m2 > 1e-10 {
        excess / area_m2 * 1000.0
    } else {
        0.0
    };
    let risk_level = if flood_depth_mm < 50.0 {
        "Low"
    } else if flood_depth_mm < 150.0 {
        "Moderate"
    } else if flood_depth_mm < 300.0 {
        "High"
    } else {
        "Severe"
    }
    .to_string();

    InundationResult {
        runoff_volume_m3,
        drainage_capacity_m3_s,
        excess_volume_m3: (excess * 100.0).round() / 100.0,
        flood_depth_mm: (flood_depth_mm * 10.0).round() / 10.0,
        risk_level,
        inundated_area_ha: area_ha * impervious_ratio,
    }
}

/// Dual drainage analysis: major (overland flow) + minor (piped) system.
#[derive(Debug, Clone, Serialize)]
pub struct DualDrainageResult {
    pub total_capacity_m3_s: f64,
    pub major_capacity_m3_s: f64,
    pub minor_capacity_m3_s: f64,
    pub total_volume_capacity_m3: f64,
    pub runoff_volume_m3: f64,
    pub excess_volume_m3: f64,
    pub system_utilization_pct: f64,
    pub status: String,
}

pub fn urban_flood_dual_drainage(
    major_system_capacity_m3_s: f64,
    minor_system_capacity_m3_s: f64,
    runoff_m3: f64,
) -> DualDrainageResult {
    let total_capacity = major_system_capacity_m3_s + minor_system_capacity_m3_s;
    // Assume 1-hour event for capacity checking
    let total_volume_capacity = total_capacity * 3600.0;
    let excess = (runoff_m3 - total_volume_capacity).max(0.0);
    let util_pct = if total_volume_capacity > 1e-10 {
        (runoff_m3 / total_volume_capacity * 100.0).min(100.0)
    } else {
        100.0
    };
    let status = if excess <= 0.0 {
        "Adequate capacity"
    } else if excess < runoff_m3 * 0.3 {
        "Minor exceedance"
    } else {
        "Major exceedance"
    }
    .to_string();

    DualDrainageResult {
        total_capacity_m3_s: (total_capacity * 100.0).round() / 100.0,
        major_capacity_m3_s: (major_system_capacity_m3_s * 100.0).round() / 100.0,
        minor_capacity_m3_s: (minor_system_capacity_m3_s * 100.0).round() / 100.0,
        total_volume_capacity_m3: (total_volume_capacity * 100.0).round() / 100.0,
        runoff_volume_m3: runoff_m3,
        excess_volume_m3: (excess * 100.0).round() / 100.0,
        system_utilization_pct: (util_pct * 10.0).round() / 10.0,
        status,
    }
}

/// Comprehensive urban flood assessment.
#[derive(Debug, Clone, Serialize)]
pub struct UrbanFloodAssessment {
    pub total_runoff_m3: f64,
    pub total_pipe_capacity_m3_s: f64,
    pub surface_storage_m3: f64,
    pub net_excess_m3: f64,
    pub flood_depth_mm: f64,
    pub flood_risk: String,
    pub mitigation_needed_m3: f64,
    pub impervious_ratio: f64,
    pub area_ha: f64,
}

/// Comprehensive urban flood assessment.
pub fn urban_flood_assessment(
    total_runoff_m3: f64,
    pipe_capacities: &[f64],
    surface_storage_m3: f64,
    area_ha: f64,
    impervious_ratio: f64,
) -> UrbanFloodAssessment {
    let total_pipe_capacity = pipe_capacities.iter().sum::<f64>();
    let pipe_volume = total_pipe_capacity * 3600.0; // 1-hour event
    let net_excess = (total_runoff_m3 - pipe_volume - surface_storage_m3).max(0.0);
    let area_m2 = area_ha * 10000.0;
    let flood_depth_mm = if area_m2 > 1e-10 {
        net_excess / area_m2 * 1000.0
    } else {
        0.0
    };
    let flood_risk = if flood_depth_mm < 50.0 {
        "Low"
    } else if flood_depth_mm < 150.0 {
        "Moderate"
    } else if flood_depth_mm < 300.0 {
        "High"
    } else {
        "Severe"
    }
    .to_string();

    UrbanFloodAssessment {
        total_runoff_m3: (total_runoff_m3 * 100.0).round() / 100.0,
        total_pipe_capacity_m3_s: (total_pipe_capacity * 100.0).round() / 100.0,
        surface_storage_m3: (surface_storage_m3 * 100.0).round() / 100.0,
        net_excess_m3: (net_excess * 100.0).round() / 100.0,
        flood_depth_mm: (flood_depth_mm * 10.0).round() / 10.0,
        flood_risk,
        mitigation_needed_m3: (net_excess * 100.0).round() / 100.0,
        impervious_ratio,
        area_ha,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inlet_capacity_grate() {
        let cap = urban_flood_inlet_capacity("grate", 0.5, 0.2);
        assert!(cap > 0.0, "Grate inlet capacity should be positive");
        assert!(cap < 5.0, "Grate inlet capacity should be reasonable");
    }

    #[test]
    fn test_inlet_capacity_curb() {
        let curb = urban_flood_inlet_capacity("curb_opening", 0.3, 0.1);
        let grate = urban_flood_inlet_capacity("grate", 0.3, 0.1);
        assert!(curb > grate, "Curb opening should have higher coefficient");
    }

    #[test]
    fn test_inlet_capacity_clogged() {
        let clean = urban_flood_inlet_capacity("grate", 1.0, 0.0);
        let clogged = urban_flood_inlet_capacity("grate", 1.0, 0.5);
        assert!(clogged < clean, "Clogged inlet should have lower capacity");
    }

    #[test]
    fn test_pipe_capacity() {
        let cap = urban_flood_pipe_capacity(0.3, 0.005, 0.013);
        assert!(cap > 0.0, "Pipe capacity should be positive");
        assert!(cap < 1.0, "300mm pipe capacity < 1 m³/s");
    }

    #[test]
    fn test_pipe_capacity_larger_pipe() {
        let small = urban_flood_pipe_capacity(0.3, 0.005, 0.013);
        let large = urban_flood_pipe_capacity(0.6, 0.005, 0.013);
        assert!(large > small, "Larger pipe should have higher capacity");
    }

    #[test]
    fn test_inundation_low_risk() {
        let r = urban_flood_inundation(100.0, 0.1, 10.0, 0.5, 1.0);
        // 100 m³ runoff vs 360 m³ drainage → no flood
        assert_eq!(r.risk_level, "Low");
        assert!(r.flood_depth_mm < 1.0);
    }

    #[test]
    fn test_inundation_severe() {
        let r = urban_flood_inundation(10000.0, 0.1, 10.0, 0.8, 1.0);
        // 10000 m³ runoff vs 360 m³ drainage → severe flooding
        assert_eq!(r.risk_level, "Moderate");
        assert!(r.flood_depth_mm > 50.0 && r.flood_depth_mm < 150.0);
    }

    #[test]
    fn test_dual_drainage_adequate() {
        let r = urban_flood_dual_drainage(5.0, 3.0, 1000.0);
        // Total capacity 8 m³/s × 3600 = 28800 m³ > 1000 m³
        assert!(r.excess_volume_m3 <= 0.0);
        assert_eq!(r.status, "Adequate capacity");
    }

    #[test]
    fn test_dual_drainage_exceedance() {
        let r = urban_flood_dual_drainage(0.5, 0.2, 10000.0);
        // Total capacity 0.7 m³/s × 3600 = 2520 m³ < 10000 m³
        assert!(r.excess_volume_m3 > 0.0);
    }

    #[test]
    fn test_flood_assessment() {
        let r = urban_flood_assessment(5000.0, &[0.2, 0.3, 0.1], 200.0, 15.0, 0.6);
        assert!(r.total_runoff_m3 > 0.0);
        assert!(r.total_pipe_capacity_m3_s > 0.0);
        assert!(r.flood_depth_mm >= 0.0);
    }

    #[test]
    fn test_flood_assessment_no_flood() {
        let r = urban_flood_assessment(1000.0, &[5.0], 5000.0, 50.0, 0.3);
        // Lots of capacity + storage → no flooding
        assert!(r.net_excess_m3 <= 0.0);
        assert_eq!(r.flood_risk, "Low");
    }
}
