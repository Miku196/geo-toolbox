//! Groundwater resources assessment: aquifer properties, recharge, drawdown.

use geo_core::errors::GeoResult;
use serde::Serialize;

use crate::GroundwaterConfig;

// ── Aquifer Properties ───────────────────────────────────────────

/// Result of a pumping test analysis (Cooper-Jacob straight-line method).
#[derive(Debug, Clone, Serialize)]
pub struct AquiferTestResult {
    /// Transmissivity (m²/day)
    pub transmissivity_m2_day: f64,
    /// Storativity (dimensionless)
    pub storativity: f64,
    /// Hydraulic conductivity (m/day) if aquifer thickness is known
    pub hydraulic_conductivity_m_day: Option<f64>,
    /// Quality of fit (R² of the straight-line approximation, 0-1)
    pub fit_r_squared: f64,
}

/// Analyze a pumping test using the Cooper-Jacob straight-line method.
///
/// Parameters:
/// - `time_min`: array of elapsed times (minutes)
/// - `drawdown_m`: array of observed drawdowns (meters)
/// - `pumping_rate_m3_day`: constant pumping rate (m³/day)
/// - `aquifer_thickness_m`: optional, for K calculation
/// - `r_m`: distance from pumping well to observation well (meters)
///
/// Uses the late-time approximation: s = (Q / (4πT)) * ln(2.25Tt / (r²S))
/// One log-cycle drawdown Δs gives T = 0.183 * Q / Δs
pub fn analyze_pumping_test(
    time_min: &[f64],
    drawdown_m: &[f64],
    pumping_rate_m3_day: f64,
    aquifer_thickness_m: Option<f64>,
    r_m: f64,
) -> GeoResult<AquiferTestResult> {
    if time_min.len() < 3 || drawdown_m.len() != time_min.len() {
        return Err(geo_core::errors::GeoError::Validation(
            "Need at least 3 time-drawdown pairs".into(),
        ));
    }
    if pumping_rate_m3_day <= 0.0 {
        return Err(geo_core::errors::GeoError::Validation(
            "Pumping rate must be positive".into(),
        ));
    }

    // Use late-time data (log-scale linear portion)
    // Fit drawdown vs log10(time) to get slope
    let log_t: Vec<f64> = time_min.iter().map(|&t| (t.max(0.001)).log10()).collect();
    // Use last half of data for late-time
    let start = time_min.len() / 2;
    let late_log_t: Vec<f64> = log_t[start..].to_vec();
    let late_dd: Vec<f64> = drawdown_m[start..].to_vec();

    if late_log_t.len() < 2 {
        return Err(geo_core::errors::GeoError::Validation(
            "Insufficient late-time data".into(),
        ));
    }

    // Simple linear regression: drawdown = a + b * log10(t)
    let n = late_log_t.len() as f64;
    let sum_x: f64 = late_log_t.iter().sum();
    let sum_y: f64 = late_dd.iter().sum();
    let sum_xy: f64 = late_log_t
        .iter()
        .zip(late_dd.iter())
        .map(|(x, y)| x * y)
        .sum();
    let sum_x2: f64 = late_log_t.iter().map(|x| x * x).sum();
    let _sum_y2: f64 = late_dd.iter().map(|y| y * y).sum();

    let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
    let intercept = (sum_y - slope * sum_x) / n;

    // R²
    let mean_y = sum_y / n;
    let ss_res: f64 = late_log_t
        .iter()
        .zip(late_dd.iter())
        .map(|(x, y)| (y - (intercept + slope * x)).powi(2))
        .sum();
    let ss_tot: f64 = late_dd.iter().map(|y| (y - mean_y).powi(2)).sum();
    let r_squared = 1.0 - ss_res / ss_tot.max(f64::MIN_POSITIVE);

    // Δs per log cycle = slope (since x is log10)
    let delta_s = slope;

    if delta_s <= 0.0 {
        return Err(geo_core::errors::GeoError::Validation(
            "Drawdown slope must be positive".into(),
        ));
    }

    // T = 0.183 * Q / Δs  (Cooper-Jacob)
    let transmissivity = 0.183 * pumping_rate_m3_day / delta_s;

    // S = 2.25 * T * t0 / r² where t0 is time at zero drawdown (10^(-a/b))
    let t0 = 10.0_f64.powf(-intercept / slope);
    let storativity = 2.25 * transmissivity * t0 / (60.0 * 24.0 * r_m * r_m);
    // t0 was in minutes, convert to days

    Ok(AquiferTestResult {
        transmissivity_m2_day: transmissivity,
        storativity: storativity.clamp(1e-8, 1.0),
        hydraulic_conductivity_m_day: aquifer_thickness_m.map(|b| {
            if b > 0.0 {
                transmissivity / b
            } else {
                0.0
            }
        }),
        fit_r_squared: r_squared.clamp(0.0, 1.0),
    })
}

// ── Theis Drawdown ───────────────────────────────────────────────

/// Theis well function W(u) approximation (Abramowitz & Stegun 1964).
fn well_function(u: f64) -> f64 {
    if u <= 0.0 {
        return f64::INFINITY;
    }
    if u > 50.0 {
        // For large u, W(u) ≈ exp(-u)/u * (1 - 1/u + 2/u² - 6/u³ + ...)
        return (-u).exp() / u * (1.0 - 1.0 / u + 2.0 / (u * u));
    }
    // Series expansion: W(u) = -γ - ln(u) + u - u²/4 + u³/18 - ...
    let _gamma = -0.5772156649015329f64.ln(); // Euler-Mascheroni constant
    let mut w = -0.5772156649015329 - u.ln();
    let mut term = u;
    let mut sign = 1.0f64;
    for n in 1..=20 {
        w += sign * term / (n as f64);
        term *= u;
        sign = -sign;
    }
    w
}

/// Compute drawdown using the Theis equation.
///
/// Parameters:
/// - `t_day`: elapsed time since pumping started (days)
/// - `r_m`: radial distance from pumping well (meters)
/// - `transmissivity_m2_day`: aquifer transmissivity (m²/day)
/// - `storativity`: aquifer storativity (dimensionless)
/// - `pumping_rate_m3_day`: constant pumping rate (m³/day)
///
/// Returns drawdown in meters.
pub fn theis_drawdown(
    t_day: f64,
    r_m: f64,
    transmissivity_m2_day: f64,
    storativity: f64,
    pumping_rate_m3_day: f64,
) -> f64 {
    if r_m <= 0.0 || t_day <= 0.0 || transmissivity_m2_day <= 0.0 || storativity <= 0.0 {
        return 0.0;
    }
    let u = r_m * r_m * storativity / (4.0 * transmissivity_m2_day * t_day);
    pumping_rate_m3_day * well_function(u) / (4.0 * std::f64::consts::PI * transmissivity_m2_day)
}

/// Compute drawdown at multiple distances using the Theis equation.
#[derive(Debug, Clone, Serialize)]
pub struct DrawdownProfile {
    pub distances_m: Vec<f64>,
    pub drawdown_m: Vec<f64>,
    pub t_day: f64,
}

pub fn theis_drawdown_profile(
    t_day: f64,
    distances_m: &[f64],
    transmissivity_m2_day: f64,
    storativity: f64,
    pumping_rate_m3_day: f64,
) -> DrawdownProfile {
    let drawdown_m: Vec<f64> = distances_m
        .iter()
        .map(|&r| {
            theis_drawdown(
                t_day,
                r,
                transmissivity_m2_day,
                storativity,
                pumping_rate_m3_day,
            )
        })
        .collect();
    DrawdownProfile {
        distances_m: distances_m.to_vec(),
        drawdown_m,
        t_day,
    }
}

// ── Recharge Estimation ──────────────────────────────────────────

/// Water balance recharge estimate.
#[derive(Debug, Clone, Serialize)]
pub struct RechargeEstimate {
    /// Annual precipitation (mm)
    pub precipitation_mm: f64,
    /// Annual actual evapotranspiration (mm)
    pub evapotranspiration_mm: f64,
    /// Surface runoff (mm)
    pub runoff_mm: f64,
    /// Estimated recharge (mm/year)
    pub recharge_mm: f64,
    /// Recharge as % of precipitation
    pub recharge_pct: f64,
    /// Recharge volume (m³/year) if area provided
    pub recharge_volume_m3_yr: Option<f64>,
}

/// Estimate groundwater recharge using a simplified water balance.
///
/// Recharge = Precipitation - Evapotranspiration - Runoff
pub fn water_balance_recharge(
    precipitation_mm: f64,
    evapotranspiration_mm: f64,
    runoff_coefficient: f64,
    area_km2: Option<f64>,
) -> RechargeEstimate {
    let runoff_mm = precipitation_mm * runoff_coefficient;
    let recharge_mm = (precipitation_mm - evapotranspiration_mm - runoff_mm).max(0.0);
    let recharge_pct = if precipitation_mm > 0.0 {
        recharge_mm / precipitation_mm * 100.0
    } else {
        0.0
    };
    RechargeEstimate {
        precipitation_mm,
        evapotranspiration_mm,
        runoff_mm,
        recharge_mm,
        recharge_pct,
        recharge_volume_m3_yr: area_km2.map(|a| recharge_mm * a * 1000.0),
    }
}

/// Estimate actual evapotranspiration using the Turc formula.
/// PET = 0.013 * (T / (T+15)) * (Rs + 50) * factor
/// where T = mean annual temperature (°C), Rs = solar radiation (cal/cm²/day)
pub fn turc_et(t_mean_c: f64, solar_radiation_cal_cm2_day: f64) -> f64 {
    let t_factor = t_mean_c / (t_mean_c + 15.0);
    0.013 * t_factor * (solar_radiation_cal_cm2_day + 50.0) * 30.0 // approximate monthly → annual mm
}

/// Estimate recharge using the chloride mass balance method.
///
/// R = P * (Cl_p / Cl_gw)
/// where Cl_p = chloride in precipitation, Cl_gw = chloride in groundwater
pub fn chloride_mass_balance_recharge(
    precipitation_mm: f64,
    cl_precipitation_mg_l: f64,
    cl_groundwater_mg_l: f64,
) -> f64 {
    if cl_groundwater_mg_l <= 0.0 {
        return 0.0;
    }
    precipitation_mm * cl_precipitation_mg_l / cl_groundwater_mg_l
}

// ── Water Table Trend ────────────────────────────────────────────

/// Water level trend analysis result.
#[derive(Debug, Clone, Serialize)]
pub struct WaterTableTrend {
    /// Slope (m/year, negative = declining)
    pub slope_m_yr: f64,
    /// Intercept (water level at time zero, meters)
    pub intercept_m: f64,
    /// R² of the trend line
    pub r_squared: f64,
    /// Number of observations
    pub n: usize,
    /// Trend description
    pub trend: String,
    /// Projected water level after n years
    pub projected_level_m_5yr: f64,
    pub projected_level_m_10yr: f64,
}

/// Analyze water table trend from time series data.
///
/// Parameters:
/// - `years`: fractional years (e.g., 2020.5 for mid-2020)
/// - `water_level_m`: observed water level depths (meters below surface)
pub fn water_table_trend(years: &[f64], water_level_m: &[f64]) -> GeoResult<WaterTableTrend> {
    if years.len() < 3 || water_level_m.len() != years.len() {
        return Err(geo_core::errors::GeoError::Validation(
            "Need at least 3 year-level pairs".into(),
        ));
    }

    let n = years.len() as f64;
    let sum_x: f64 = years.iter().sum();
    let sum_y: f64 = water_level_m.iter().sum();
    let sum_xy: f64 = years
        .iter()
        .zip(water_level_m.iter())
        .map(|(x, y)| x * y)
        .sum();
    let sum_x2: f64 = years.iter().map(|x| x * x).sum();
    let _sum_y2: f64 = water_level_m.iter().map(|y| y * y).sum();

    let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
    let intercept = (sum_y - slope * sum_x) / n;

    let mean_y = sum_y / n;
    let ss_res: f64 = years
        .iter()
        .zip(water_level_m.iter())
        .map(|(x, y)| (y - (intercept + slope * x)).powi(2))
        .sum();
    let ss_tot: f64 = water_level_m.iter().map(|y| (y - mean_y).powi(2)).sum();
    let r_squared = 1.0 - ss_res / ss_tot.max(f64::MIN_POSITIVE);

    let last_year = years.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let trend = if slope.abs() < 0.01 {
        "Stable".into()
    } else if slope > 0.0 {
        "Rising (water table declining)".into()
    } else {
        "Declining (water table recovering)".into()
    };

    Ok(WaterTableTrend {
        slope_m_yr: slope,
        intercept_m: intercept,
        r_squared,
        n: years.len(),
        trend,
        projected_level_m_5yr: intercept + slope * (last_year + 5.0),
        projected_level_m_10yr: intercept + slope * (last_year + 10.0),
    })
}

// ── Specific Capacity ────────────────────────────────────────────

/// Estimate transmissivity from specific capacity using the empirical Driscoll method.
/// T ≈ specific_capacity * factor (factor typically 1.0–2.0 for confined aquifers)
pub fn specific_capacity_to_t(
    specific_capacity_m2_day: f64, // m³/day per meter of drawdown
    factor: f64,
) -> f64 {
    specific_capacity_m2_day * factor
}

// ── Plugin ───────────────────────────────────────────────────────

pub struct GroundwaterPlugin {
    pub config: GroundwaterConfig,
}

impl GroundwaterPlugin {
    pub fn new(config: GroundwaterConfig) -> Self {
        Self { config }
    }

    pub fn analyze_pumping_test(
        &self,
        time_min: &[f64],
        drawdown_m: &[f64],
        pumping_rate_m3_day: f64,
        aquifer_thickness_m: Option<f64>,
        r_m: f64,
    ) -> GeoResult<AquiferTestResult> {
        analyze_pumping_test(
            time_min,
            drawdown_m,
            pumping_rate_m3_day,
            aquifer_thickness_m,
            r_m,
        )
    }

    pub fn theis_drawdown(
        &self,
        t_day: f64,
        r_m: f64,
        transmissivity_m2_day: f64,
        storativity: f64,
        pumping_rate_m3_day: f64,
    ) -> f64 {
        theis_drawdown(
            t_day,
            r_m,
            transmissivity_m2_day,
            storativity,
            pumping_rate_m3_day,
        )
    }

    pub fn recharge(
        &self,
        precipitation_mm: f64,
        evapotranspiration_mm: f64,
        runoff_coefficient: f64,
        area_km2: Option<f64>,
    ) -> RechargeEstimate {
        water_balance_recharge(
            precipitation_mm,
            evapotranspiration_mm,
            runoff_coefficient,
            area_km2,
        )
    }

    pub fn water_table_trend(
        &self,
        years: &[f64],
        water_level_m: &[f64],
    ) -> GeoResult<WaterTableTrend> {
        water_table_trend(years, water_level_m)
    }

    pub fn chloride_recharge(
        &self,
        precipitation_mm: f64,
        cl_precip_mg_l: f64,
        cl_gw_mg_l: f64,
    ) -> f64 {
        chloride_mass_balance_recharge(precipitation_mm, cl_precip_mg_l, cl_gw_mg_l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pumping_test() {
        let time_min = vec![1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0];
        // Simulated drawdown for T=50 m²/day, S=0.001, Q=100 m³/day, r=10m
        let drawdown = vec![0.05, 0.12, 0.25, 0.35, 0.48, 0.65, 0.78, 0.90];
        let result = analyze_pumping_test(&time_min, &drawdown, 100.0, Some(10.0), 10.0).unwrap();
        assert!(result.transmissivity_m2_day > 0.0);
        assert!(result.storativity > 0.0);
        assert!(result.fit_r_squared > 0.5);
    }

    #[test]
    fn test_theis_drawdown() {
        let dd = theis_drawdown(1.0, 10.0, 100.0, 0.001, 200.0);
        assert!(dd > 0.0);
        // Drawdown should decrease with distance
        let dd_far = theis_drawdown(1.0, 100.0, 100.0, 0.001, 200.0);
        assert!(dd_far < dd);
    }

    #[test]
    fn test_recharge() {
        let r = water_balance_recharge(800.0, 450.0, 0.1, Some(10.0));
        assert!(r.recharge_mm > 0.0);
        assert!(r.recharge_volume_m3_yr.unwrap() > 0.0);
    }

    #[test]
    fn test_water_table_trend() {
        let years = vec![2015.0, 2017.0, 2019.0, 2021.0, 2023.0];
        let levels = vec![10.0, 10.5, 11.2, 11.8, 12.0]; // declining
        let trend = water_table_trend(&years, &levels).unwrap();
        assert!(trend.slope_m_yr > 0.0); // water level rising = declining table
        assert!(trend.r_squared > 0.5);
        assert_eq!(trend.n, 5);
    }

    #[test]
    fn test_chloride_recharge() {
        let r = chloride_mass_balance_recharge(800.0, 2.0, 50.0);
        assert!(r > 0.0 && r < 800.0);
    }

    #[test]
    fn test_turc_et() {
        let et = turc_et(15.0, 400.0);
        assert!(et > 0.0);
    }
}
