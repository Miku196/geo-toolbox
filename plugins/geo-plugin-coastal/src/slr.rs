//! Sea Level Rise (SLR) bathtub model — IPCC AR6 scenarios.
//!
//! Includes: scenario-based SLR projection, bathtub inundation mapping,
//! Bruun Rule erosion impact.

use serde::{Deserialize, Serialize};

/// IPCC AR6 SLR data points (meters above 2000 baseline).
struct SlrDataPoint {
    year: u16,
    level_m: f64,
}

/// Return SLR values for known SSP scenarios.
fn scenario_data(scenario: &str) -> &[SlrDataPoint] {
    match scenario {
        "SSP1-1.9" | "ssp1-1.9" | "ssp119" => &[
            SlrDataPoint {
                year: 2020,
                level_m: 0.06,
            },
            SlrDataPoint {
                year: 2050,
                level_m: 0.28,
            },
            SlrDataPoint {
                year: 2100,
                level_m: 0.41,
            },
        ],
        "SSP1-2.6" | "ssp1-2.6" | "ssp126" => &[
            SlrDataPoint {
                year: 2020,
                level_m: 0.06,
            },
            SlrDataPoint {
                year: 2050,
                level_m: 0.32,
            },
            SlrDataPoint {
                year: 2100,
                level_m: 0.55,
            },
        ],
        "SSP2-4.5" | "ssp2-4.5" | "ssp245" => &[
            SlrDataPoint {
                year: 2020,
                level_m: 0.06,
            },
            SlrDataPoint {
                year: 2050,
                level_m: 0.32,
            },
            SlrDataPoint {
                year: 2100,
                level_m: 0.62,
            },
        ],
        "SSP3-7.0" | "ssp3-7.0" | "ssp370" => &[
            SlrDataPoint {
                year: 2020,
                level_m: 0.06,
            },
            SlrDataPoint {
                year: 2050,
                level_m: 0.34,
            },
            SlrDataPoint {
                year: 2100,
                level_m: 0.72,
            },
        ],
        "SSP5-8.5" | "ssp5-8.5" | "ssp585" => &[
            SlrDataPoint {
                year: 2020,
                level_m: 0.06,
            },
            SlrDataPoint {
                year: 2050,
                level_m: 0.37,
            },
            SlrDataPoint {
                year: 2100,
                level_m: 0.88,
            },
        ],
        _ => &[
            SlrDataPoint {
                year: 2020,
                level_m: 0.06,
            },
            SlrDataPoint {
                year: 2100,
                level_m: 0.62,
            },
        ],
    }
}

/// Linear interpolation between two points.
fn lerp(x: f64, x0: f64, x1: f64, y0: f64, y1: f64) -> f64 {
    if (x1 - x0).abs() < 1e-12 {
        return (y0 + y1) / 2.0;
    }
    y0 + (x - x0) * (y1 - y0) / (x1 - x0)
}

/// SLR level for a given scenario and year (meters above 2000 baseline).
pub fn slr_scenario_level(scenario: &str, year: u16) -> f64 {
    let data = scenario_data(scenario);
    let year_f = year as f64;

    // Before first data point
    if year <= data[0].year {
        return data[0].level_m;
    }

    // After last data point — extrapolate using last segment
    if year >= data[data.len() - 1].year {
        let last = &data[data.len() - 1];
        let prev = &data[data.len() - 2];
        return lerp(
            year_f,
            prev.year as f64,
            last.year as f64,
            prev.level_m,
            last.level_m,
        );
    }

    // Between two points
    for i in 0..data.len() - 1 {
        if year >= data[i].year && year <= data[i + 1].year {
            return lerp(
                year_f,
                data[i].year as f64,
                data[i + 1].year as f64,
                data[i].level_m,
                data[i + 1].level_m,
            );
        }
    }

    data[data.len() - 1].level_m
}

/// Bathtub inundation area from DEM.
pub fn slr_inundation_area(
    dem: &[f64],
    cols: usize,
    rows: usize,
    cell_size_m: f64,
    slr_m: f64,
    tidal_range_m: f64,
) -> serde_json::Value {
    let total_cells = cols * rows;
    if dem.len() < total_cells || total_cells == 0 {
        return serde_json::json!({
            "error": "DEM dimensions mismatch or empty"
        });
    }

    let effective_water_level = slr_m + tidal_range_m / 2.0;
    let mut inundated = 0u64;
    let mut total_depth = 0.0f64;

    for &z in dem.iter().take(total_cells) {
        if z < effective_water_level {
            inundated += 1;
            total_depth += effective_water_level - z;
        }
    }

    let cell_area_ha = cell_size_m * cell_size_m / 10_000.0;
    let inundated_area_ha = inundated as f64 * cell_area_ha;
    let inundation_pct = if total_cells > 0 {
        inundated as f64 / total_cells as f64 * 100.0
    } else {
        0.0
    };
    let mean_depth_m = if inundated > 0 {
        total_depth / inundated as f64
    } else {
        0.0
    };

    serde_json::json!({
        "inundated_cells": inundated,
        "total_cells": total_cells,
        "inundated_area_ha": (inundated_area_ha * 100.0).round() / 100.0,
        "inundation_pct": (inundation_pct * 100.0).round() / 100.0,
        "mean_depth_m": (mean_depth_m * 100.0).round() / 100.0,
        "effective_water_level_m": (effective_water_level * 100.0).round() / 100.0,
        "slr_m": (slr_m * 100.0).round() / 100.0,
    })
}

/// Comprehensive coastal SLR impact assessment.
pub fn slr_coastal_impact(
    dem: &[f64],
    cols: usize,
    rows: usize,
    cell_size_m: f64,
    scenario: &str,
    year: u16,
) -> serde_json::Value {
    let sea_level_m = slr_scenario_level(scenario, year);
    let inundation = slr_inundation_area(dem, cols, rows, cell_size_m, sea_level_m, 0.0);

    serde_json::json!({
        "scenario": scenario,
        "year": year,
        "sea_level_rise_m": (sea_level_m * 100.0).round() / 100.0,
        "inundation": inundation
    })
}

/// Bruun Rule erosion impact from SLR.
pub fn slr_erosion_impact(
    slr_m: f64,
    shoreline_length_km: f64,
    beach_slope: f64,
    closure_depth_m: f64,
) -> serde_json::Value {
    if beach_slope <= 0.0 || shoreline_length_km <= 0.0 {
        return serde_json::json!({
            "error": "beach_slope and shoreline_length_km must be positive"
        });
    }
    // Bruun Rule: R = S / tan(β)
    let slope_rad = (beach_slope / 100.0).atan();
    let recession_m = if slope_rad > 0.0 {
        slr_m / slope_rad.tan()
    } else {
        0.0
    };
    let erosion_volume_m3 = recession_m * shoreline_length_km * 1000.0 * closure_depth_m;

    serde_json::json!({
        "slr_m": (slr_m * 100.0).round() / 100.0,
        "shoreline_length_km": shoreline_length_km,
        "beach_slope_pct": beach_slope,
        "closure_depth_m": closure_depth_m,
        "recession_m": (recession_m * 100.0).round() / 100.0,
        "erosion_volume_m3": (erosion_volume_m3 * 100.0).round() / 100.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slr_scenario_level_known() {
        let l = slr_scenario_level("SSP2-4.5", 2020);
        assert!((l - 0.06).abs() < 0.01, "got {l}");
        let l = slr_scenario_level("SSP2-4.5", 2100);
        assert!((l - 0.62).abs() < 0.01, "got {l}");
    }

    #[test]
    fn test_slr_scenario_level_interp() {
        // 2050 is exactly mid: SSP1-1.9 0.28
        let l = slr_scenario_level("SSP1-1.9", 2050);
        assert!((l - 0.28).abs() < 0.01, "got {l}");
        // 2035 interp between 2020(0.06) and 2050(0.28) for SSP2-4.5
        let l = slr_scenario_level("SSP2-4.5", 2035);
        let expected = 0.06 + (15.0 / 30.0) * (0.32 - 0.06);
        assert!((l - expected).abs() < 0.01, "got {l} expected {expected}");
    }

    #[test]
    fn test_slr_scenario_level_extrapolate() {
        // 2150 beyond last point, extrapolate from 2050-2100
        let l = slr_scenario_level("SSP1-1.9", 2150);
        // 2100 is 0.41, slope = (0.41-0.28)/50 = 0.0026, so 2150 = 0.41+50*0.0026 = 0.54
        assert!(l > 0.41, "got {l} — should extrapolate above 2100");
    }

    #[test]
    fn test_slr_scenario_level_default_scenario() {
        let l = slr_scenario_level("unknown_scenario", 2100);
        assert!((l - 0.62).abs() < 0.01, "got {l}"); // falls through to default
    }

    #[test]
    fn test_slr_inundation_area_simple() {
        let dem = vec![5.0, 6.0, 7.0, 4.0, 5.0, 6.0];
        let result = slr_inundation_area(&dem, 3, 2, 30.0, 0.5, 1.0);
        // Effective water = 0.5 + 0.5 = 1.0. Cells below 1.0: none (min is 4.0)
        assert_eq!(result["inundated_cells"].as_u64().unwrap(), 0);
        assert_eq!(result["total_cells"].as_u64().unwrap(), 6);
    }

    #[test]
    fn test_slr_inundation_area_with_flooding() {
        let dem = vec![-2.0, 0.0, 2.0, 4.0];
        let result = slr_inundation_area(&dem, 2, 2, 30.0, 1.0, 2.0);
        // Effective water = 1.0 + 1.0 = 2.0. Cells below 2.0: index 0 (-2), 1 (0)
        assert_eq!(result["inundated_cells"].as_u64().unwrap(), 2);
        let area_ha = result["inundated_area_ha"].as_f64().unwrap();
        assert!((area_ha - 2.0 * 900.0 / 10_000.0).abs() < 0.01);
    }

    #[test]
    fn test_slr_coastal_impact() {
        let dem = vec![1.0, 2.0, 3.0, 4.0];
        let result = slr_coastal_impact(&dem, 2, 2, 30.0, "SSP2-4.5", 2100);
        assert_eq!(result["scenario"].as_str().unwrap(), "SSP2-4.5");
        assert_eq!(result["year"].as_u64().unwrap(), 2100);
        assert!(result["inundation"]["inundated_cells"].as_u64().is_some());
    }

    #[test]
    fn test_slr_erosion_impact() {
        let result = slr_erosion_impact(0.5, 10.0, 3.0, 10.0);
        // recession = 0.5 / tan(atan(3/100)) = 0.5 / 0.03 = 16.67 m
        let recession = result["recession_m"].as_f64().unwrap();
        assert!((recession - 0.5 / 0.03).abs() < 1.0, "got {recession}");
        let vol = result["erosion_volume_m3"].as_f64().unwrap();
        assert!((vol - recession * 10_000.0 * 10.0).abs() < 10000.0);
    }
}
