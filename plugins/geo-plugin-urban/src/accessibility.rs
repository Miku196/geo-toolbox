//! 15-minute city accessibility analysis.
//!
//! Implements:
//! - Walking time/distance calculations
//! - Isochrone radius
//! - Facility accessibility scoring
//! - Composite 15-minute city assessment
//! - Service area gap analysis

use serde::Serialize;
use std::collections::HashMap;

/// Walking time (minutes) from distance and speed.
pub fn walking_time(distance_m: f64, walking_speed_kmh: f64) -> f64 {
    if walking_speed_kmh < 1e-10 {
        return 999.0;
    }
    distance_m / (walking_speed_kmh * 1000.0 / 60.0)
}

/// Isochrone radius (meters) from travel time and speed.
pub fn isochrone_radius(travel_time_min: f64, speed_kmh: f64) -> f64 {
    speed_kmh * 1000.0 / 60.0 * travel_time_min
}

/// Accessibility score based on facility proximity to origin.
///
/// Returns a detailed breakdown of walkable facilities within the isochrone.
pub fn accessibility_score(
    facilities: &[(f64, f64, &str)],
    origin_lat: f64,
    origin_lon: f64,
    max_walk_min: f64,
    walking_speed_kmh: f64,
) -> serde_json::Value {
    let radius_m = isochrone_radius(max_walk_min, walking_speed_kmh);
    let mut within_range = 0usize;
    let mut by_type: HashMap<String, usize> = HashMap::new();

    for (lat, lon, ftype) in facilities {
        let d = haversine_distance_2d(*lat, *lon, origin_lat, origin_lon);
        if d <= radius_m {
            within_range += 1;
            *by_type.entry(ftype.to_string()).or_default() += 1;
        }
    }

    let total = facilities.len();
    let score = if total > 0 {
        (within_range as f64 / total as f64 * 100.0).round() as u64
    } else {
        0
    };

    serde_json::json!({
        "total_facilities": total,
        "within_range": within_range,
        "isochrone_radius_m": (radius_m * 10.0).round() / 10.0,
        "max_walk_min": max_walk_min,
        "walking_speed_kmh": walking_speed_kmh,
        "facility_types": by_type,
        "score": score
    })
}

/// Approximate Haversine distance in meters.
fn haversine_distance_2d(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6_371_000.0;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    r * c
}

/// Composite 15-minute city assessment.
#[derive(Debug, Clone, Serialize)]
pub struct AccessibilityAssessment {
    pub population: u64,
    pub green_space_ha: f64,
    pub green_per_capita_m2: f64,
    pub transit_stops_per_km2: f64,
    pub schools_per_km2: f64,
    pub hospitals_per_km2: f64,
    pub shops_per_km2: f64,
    pub composite_score: u64,
    pub grade: String,
}

/// Compute composite 15-minute city score (0–100).
///
/// Green space threshold: 9 m²/capita
/// Transit: 0.5 stops/km²  
/// School: 0.2 /km²
/// Hospital: 0.02 /km²
/// Shops: 0.5 /km²
pub fn accessibility_assessment(
    population: u64,
    green_space_ha: f64,
    transit_stops: u64,
    schools: u64,
    hospitals: u64,
    shops: u64,
    area_km2: f64,
) -> AccessibilityAssessment {
    let pop = population as f64;
    let green_per_capita = if pop > 0.0 {
        green_space_ha * 10000.0 / pop
    } else {
        0.0
    };
    let transit_density = if area_km2 > 0.0 {
        transit_stops as f64 / area_km2
    } else {
        0.0
    };
    let school_density = if area_km2 > 0.0 {
        schools as f64 / area_km2
    } else {
        0.0
    };
    let hospital_density = if area_km2 > 0.0 {
        hospitals as f64 / area_km2
    } else {
        0.0
    };
    let shop_density = if area_km2 > 0.0 {
        shops as f64 / area_km2
    } else {
        0.0
    };

    // Score each dimension 0–100
    let green_score = (green_per_capita / 9.0 * 100.0).min(100.0);
    let transit_score = (transit_density / 0.5 * 100.0).min(100.0);
    let school_score = (school_density / 0.2 * 100.0).min(100.0);
    let hospital_score = (hospital_density / 0.02 * 100.0).min(100.0);
    let shop_score = (shop_density / 0.5 * 100.0).min(100.0);

    let composite = ((green_score + transit_score + school_score + hospital_score + shop_score)
        / 5.0)
        .round() as u64;

    let grade = if composite >= 80 {
        "A — Excellent 15-minute city"
    } else if composite >= 60 {
        "B — Good"
    } else if composite >= 40 {
        "C — Fair"
    } else if composite >= 20 {
        "D — Poor"
    } else {
        "F — Requires significant improvement"
    }
    .to_string();

    AccessibilityAssessment {
        population,
        green_space_ha,
        green_per_capita_m2: (green_per_capita * 10.0).round() / 10.0,
        transit_stops_per_km2: (transit_density * 100.0).round() / 100.0,
        schools_per_km2: (school_density * 100.0).round() / 100.0,
        hospitals_per_km2: (hospital_density * 1000.0).round() / 1000.0,
        shops_per_km2: (shop_density * 100.0).round() / 100.0,
        composite_score: composite,
        grade,
    }
}

/// Service area gap analysis: compute deficiency vs. standards.
#[derive(Debug, Clone, Serialize)]
pub struct ServiceAreaGap {
    pub facility_type: String,
    pub existing_count: u64,
    pub recommended_count: u64,
    pub gap: i64,
    pub gap_pct: f64,
    pub recommendation: String,
}

/// Analyze service area gaps against planning standards.
///
/// Standards:
/// - school: 1 per 5,000 pop
/// - hospital: 1 per 50,000 pop
/// - grocery: 1 per 10,000 pop
/// - park: 1 per 5,000 pop
pub fn service_area_gap(
    population_density_per_km2: f64,
    facility_type: &str,
    existing_count: u64,
    area_km2: f64,
) -> ServiceAreaGap {
    let pop = population_density_per_km2 * area_km2;
    let denominator = match facility_type.to_lowercase().as_str() {
        "school" | "schools" => 5000.0,
        "hospital" | "hospitals" => 50000.0,
        "grocery" | "supermarket" | "shop" => 10000.0,
        "park" | "parks" | "green_space" => 5000.0,
        _ => 10000.0,
    };

    let recommended = (pop / denominator).ceil() as u64;
    let gap = if recommended >= existing_count {
        (recommended - existing_count) as i64
    } else {
        0
    };
    let gap_pct = if recommended > 0 {
        (gap as f64 / recommended as f64 * 100.0).round() / 10.0
    } else {
        0.0
    };

    let recommendation = if gap == 0 {
        format!("Adequate {} provision for population", facility_type)
    } else {
        format!(
            "Need {} additional {} facility(-ies) to meet standard of 1 per {:.0} population",
            gap, facility_type, denominator
        )
    };

    ServiceAreaGap {
        facility_type: facility_type.to_string(),
        existing_count,
        recommended_count: recommended,
        gap,
        gap_pct,
        recommendation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_walking_time() {
        let wt = walking_time(800.0, 4.0);
        assert!(
            (wt - 12.0).abs() < 0.1,
            "Walking time 800m at 4 km/h = {} min",
            wt
        );
    }

    #[test]
    fn test_isochrone_radius() {
        let r = isochrone_radius(15.0, 5.0);
        assert!((r - 1250.0).abs() < 1.0, "15 min at 5 km/h = {} m", r);
    }

    #[test]
    fn test_accessibility_score_no_facilities() {
        let r = accessibility_score(&[], 30.0, 104.0, 15.0, 5.0);
        assert_eq!(r["score"].as_u64().unwrap(), 0);
        assert_eq!(r["total_facilities"].as_u64().unwrap(), 0);
    }

    #[test]
    fn test_accessibility_score_nearby() {
        let facilities = vec![
            (30.67, 104.06, "park"),     // ~0m
            (30.671, 104.061, "school"), // ~150m
            (30.68, 104.07, "hospital"), // ~1.5km
            (30.70, 104.10, "grocery"),  // ~5km
        ];
        let r = accessibility_score(&facilities, 30.67, 104.06, 15.0, 5.0);
        // Within 1250m radius → first 2 should be within, rest outside
        assert_eq!(r["total_facilities"].as_u64().unwrap(), 4);
        assert!(r["within_range"].as_u64().unwrap() >= 2);
    }

    #[test]
    fn test_haversine_distance_2d() {
        let d = haversine_distance_2d(30.67, 104.06, 30.67, 104.06);
        assert!(d < 1.0, "Same point distance should be ~0");
    }

    #[test]
    fn test_assessment_a_grade() {
        let r = accessibility_assessment(10000, 100.0, 100, 50, 10, 100, 5.0);
        // 100 ha green = 100000 m² / 10000 pop = 10 m²/capita (≥9 ✓)
        // transit 100/5 = 20 stops/km² (≥0.5 ✓)
        // school 50/5 = 10/km² (≥0.2 ✓)
        // hospital 10/5 = 2/km² (≥0.02 ✓)
        // shop 100/5 = 20/km² (≥0.5 ✓)
        assert!(r.composite_score >= 80);
        assert!(r.grade.starts_with("A"));
    }

    #[test]
    fn test_assessment_f_grade() {
        let r = accessibility_assessment(1000000, 0.0, 0, 0, 0, 0, 50.0);
        assert!(r.composite_score < 40);
    }

    #[test]
    fn test_service_area_gap_sufficient() {
        let r = service_area_gap(1000.0, "school", 10, 5.0);
        // pop = 5000, school need = 1, existing = 10 → no gap
        assert_eq!(r.gap, 0);
    }

    #[test]
    fn test_service_area_gap_deficit() {
        let r = service_area_gap(20000.0, "hospital", 1, 10.0);
        // pop = 200000, hospital need = 4, existing = 1 → gap = 3
        assert_eq!(r.gap, 3);
    }

    #[test]
    fn test_service_area_gap_edge() {
        let r = service_area_gap(0.0, "park", 0, 0.0);
        // No population, no area → no gap
        assert_eq!(r.recommended_count, 0);
        assert_eq!(r.gap, 0);
    }

    #[test]
    fn test_green_per_capita() {
        let r = accessibility_assessment(50000, 45.0, 10, 5, 2, 30, 10.0);
        // 45 ha * 10000 = 450000 m² / 50000 pop = 9 m²/capita
        assert!((r.green_per_capita_m2 - 9.0).abs() < 0.1);
    }
}
