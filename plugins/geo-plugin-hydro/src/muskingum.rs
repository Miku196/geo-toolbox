//! Muskingum flood routing — standard and Muskingum-Cunge.
//!
//! Implements:
//! - Muskingum coefficients (C0, C1, C2)
//! - Muskingum routing for a reach
//! - Muskingum-Cunge with rectangular channel hydraulics
//! - Attenuation analysis

use serde::Serialize;

/// Compute Muskingum routing coefficients.
///
/// # Arguments
/// * `k_hrs` - Travel time of the flood wave through the reach (hours)
/// * `x` - Weighting factor (0–0.5, typically 0.1–0.3)
/// * `dt_hrs` - Time step (hours)
///
/// # Returns
/// `(C0, C1, C2)` such that `C0 + C1 + C2 ≈ 1.0`
pub fn muskingum_coefficients(k_hrs: f64, x: f64, dt_hrs: f64) -> (f64, f64, f64) {
    let denom = 2.0 * k_hrs * (1.0 - x) + dt_hrs;
    if denom.abs() < 1e-15 {
        return (0.0, 1.0, 0.0); // degenerate case
    }
    let c0 = (dt_hrs - 2.0 * k_hrs * x) / denom;
    let c1 = (dt_hrs + 2.0 * k_hrs * x) / denom;
    let c2 = (2.0 * k_hrs * (1.0 - x) - dt_hrs) / denom;
    (c0, c1, c2)
}

/// Muskingum routing: given inflow hydrograph, compute outflow.
///
/// O[0] = I[0], then O[t] = C0 * I[t] + C1 * I[t-1] + C2 * O[t-1]
pub fn muskingum_route(inflow: &[f64], k_hrs: f64, x: f64, dt_hrs: f64) -> Vec<f64> {
    let (c0, c1, c2) = muskingum_coefficients(k_hrs, x, dt_hrs);
    let n = inflow.len();
    if n == 0 {
        return vec![];
    }
    let mut outflow = Vec::with_capacity(n);
    outflow.push(inflow[0]); // O[0] = I[0]
    for t in 1..n {
        let ot = c0 * inflow[t] + c1 * inflow[t - 1] + c2 * outflow[t - 1];
        outflow.push(ot.max(0.0));
    }
    outflow
}

/// Muskingum-Cunge routing for a rectangular channel.
///
/// Computes wave celerity and routing parameters from channel geometry.
pub fn muskingum_cunge_route(
    inflow: &[f64],
    channel_length_m: f64,
    channel_slope: f64,
    channel_width_m: f64,
    manning_n: f64,
    dt_hrs: f64,
) -> Vec<f64> {
    if inflow.is_empty() || inflow[0] < 1e-15 {
        return muskingum_route(inflow, 0.5, 0.2, dt_hrs);
    }

    let q0 = inflow[0];
    let w = channel_width_m;
    let s = channel_slope;
    let n = manning_n;

    // Manning normal depth for wide rectangular: Q = (1/n)*w*y^(5/3)*S^0.5
    let y0 = ((q0 * n) / (w * s.sqrt())).powf(3.0 / 5.0);

    // Velocity and wave celerity
    let v0 = q0 / (w * y0);
    let celerity = (5.0 / 3.0) * v0; // Kleitz-Seddon law for wide rectangular

    // Kinematic wave number
    let k_hrs = channel_length_m / celerity / 3600.0; // seconds to hours

    // X parameter
    let x = if w * s * celerity * channel_length_m > 1e-15 {
        let x_val = 0.5 * (1.0 - q0 / (w * s * celerity * channel_length_m));
        x_val.clamp(0.0, 0.5)
    } else {
        0.2
    };

    muskingum_route(inflow, k_hrs, x, dt_hrs)
}

/// Analyze flood wave attenuation through the reach.
#[derive(Debug, Clone, Serialize)]
pub struct AttenuationResult {
    pub peak_inflow: f64,
    pub peak_outflow: f64,
    pub attenuation_pct: f64,
    pub lag_hrs: f64,
    pub coefficients: Vec<f64>,
}

/// Analyze attenuation between inflow and outflow hydrographs.
pub fn attenuation_analysis(inflow: &[f64], k_hrs: f64, x: f64, dt_hrs: f64) -> AttenuationResult {
    let outflow = muskingum_route(inflow, k_hrs, x, dt_hrs);
    let peak_in = inflow.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let peak_out = outflow.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let attenuation_pct = if peak_in > 1e-10 {
        ((peak_in - peak_out) / peak_in * 100.0).max(0.0)
    } else {
        0.0
    };

    // Estimate lag: time from peak inflow to peak outflow
    let peak_in_idx = inflow
        .iter()
        .position(|&v| (v - peak_in).abs() < 1e-10)
        .unwrap_or(0);
    let peak_out_idx = outflow
        .iter()
        .position(|&v| (v - peak_out).abs() < 1e-10)
        .unwrap_or(0);
    let lag_hrs = (peak_out_idx as f64 - peak_in_idx as f64) * dt_hrs;

    let (c0, c1, c2) = muskingum_coefficients(k_hrs, x, dt_hrs);

    AttenuationResult {
        peak_inflow: (peak_in * 100.0).round() / 100.0,
        peak_outflow: (peak_out * 100.0).round() / 100.0,
        attenuation_pct: (attenuation_pct * 100.0).round() / 100.0,
        lag_hrs: (lag_hrs * 10.0).round() / 10.0,
        coefficients: vec![
            (c0 * 1000.0).round() / 1000.0,
            (c1 * 1000.0).round() / 1000.0,
            (c2 * 1000.0).round() / 1000.0,
        ],
    }
}

/// Flood wave celerity for a wide rectangular channel.
///
/// c = (5/3) * V = (5/3) * Q / A
pub fn flood_wave_velocity(
    channel_slope: f64,
    channel_width_m: f64,
    manning_n: f64,
    discharge_m3_s: f64,
) -> f64 {
    if discharge_m3_s < 1e-15 || channel_width_m < 1e-15 || manning_n < 1e-15 {
        return 0.0;
    }
    let y0 =
        ((discharge_m3_s * manning_n) / (channel_width_m * channel_slope.sqrt())).powf(3.0 / 5.0);
    let v0 = discharge_m3_s / (channel_width_m * y0);
    (5.0 / 3.0) * v0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coefficients_sum_to_one() {
        let (c0, c1, c2) = muskingum_coefficients(2.0, 0.2, 1.0);
        let sum = c0 + c1 + c2;
        assert!((sum - 1.0).abs() < 1e-6, "C0+C1+C2 = {}", sum);
    }

    #[test]
    fn test_routing_no_attenuation() {
        let inflow = vec![10.0, 20.0, 30.0, 25.0, 15.0, 5.0];
        let outflow = muskingum_route(&inflow, 0.0, 0.2, 1.0);
        assert_eq!(outflow.len(), inflow.len());
        assert!(outflow[0] >= 0.0);
    }

    #[test]
    fn test_routing_attenuation() {
        let inflow = vec![0.0, 10.0, 30.0, 50.0, 60.0, 55.0, 40.0, 25.0, 10.0, 0.0];
        let outflow = muskingum_route(&inflow, 2.0, 0.2, 1.0);
        assert_eq!(outflow.len(), inflow.len());
        // Outflow peak should be ≤ inflow peak
        let in_peak = inflow.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let out_peak = outflow.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(out_peak <= in_peak + 0.1);
    }

    #[test]
    fn test_coefficients_degenerate() {
        let (c0, c1, c2) = muskingum_coefficients(0.0, 0.2, 1.0);
        assert!((c0 + c1 + c2 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_attenuation_analysis() {
        let inflow = vec![0.0, 10.0, 30.0, 50.0, 60.0, 55.0, 40.0, 25.0, 10.0, 0.0];
        let result = attenuation_analysis(&inflow, 2.0, 0.2, 1.0);
        assert!(result.peak_inflow > 0.0);
        assert!(result.peak_outflow > 0.0);
        assert!(result.attenuation_pct >= 0.0);
    }

    #[test]
    fn test_flood_wave_velocity() {
        let v = flood_wave_velocity(0.001, 50.0, 0.035, 100.0);
        assert!(v > 0.0);
    }

    #[test]
    fn test_cunge_routing() {
        let inflow = vec![1.0, 5.0, 15.0, 30.0, 40.0, 35.0, 25.0, 15.0, 5.0, 1.0];
        let outflow = muskingum_cunge_route(&inflow, 5000.0, 0.001, 50.0, 0.035, 1.0);
        assert_eq!(outflow.len(), inflow.len());
        assert!(outflow.iter().all(|&v| v >= 0.0));
    }

    #[test]
    fn test_cunge_empty() {
        let outflow = muskingum_cunge_route(&[], 5000.0, 0.001, 50.0, 0.035, 1.0);
        assert!(outflow.is_empty());
    }

    #[test]
    fn test_attenuation_symmetric_inflow() {
        let inflow = vec![0.0, 10.0, 20.0, 20.0, 10.0, 0.0];
        let result = attenuation_analysis(&inflow, 1.0, 0.1, 1.0);
        assert_eq!(result.peak_inflow, 20.0);
        assert!(result.peak_outflow > 0.0);
        assert!(result.peak_outflow <= 20.0);
    }
}
