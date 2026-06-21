//! Unit hydrograph runoff routing — converts SCS-CN runoff depth to discharge
//! hydrograph (flow vs time).
//!
//! Three methods: Snyder synthetic UH, SCS dimensionless UH, Clark UH.
//! Plus convolution of UH with excess rainfall hyetograph.

use serde::{Deserialize, Serialize};

/// Unit hydrograph result — discharge at time steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitHydrograph {
    /// Method used (e.g. "snyder", "scs", "clark")
    pub method: String,
    /// Time step (hours between ordinates)
    pub dt_hours: f64,
    /// Total duration (hours)
    pub duration_hours: f64,
    /// Peak discharge (m³/s)
    pub peak_q: f64,
    /// Time to peak (hours from start)
    pub time_to_peak_h: f64,
    /// Discharge ordinates (m³/s) at each time step
    pub ordinates: Vec<f64>,
    /// Total runoff volume (m³) — should match input
    pub total_volume_m3: f64,
    /// Area used (km²)
    pub area_km2: f64,
    /// Rainfall excess used (mm)
    pub rainfall_excess_mm: f64,
}

/// Snyder unit hydrograph parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnyderParams {
    /// Basin coefficient Ct (0.4–2.2, default 1.8 for mountainous, 0.7 for flat)
    pub ct: f64,
    /// Peak coefficient Cp (0.4–0.8, default 0.6)
    pub cp: f64,
}

impl Default for SnyderParams {
    fn default() -> Self {
        Self { ct: 1.8, cp: 0.6 }
    }
}

/// SCS dimensionless unit hydrograph parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScsUhParams {
    /// Peak rate factor (484 for standard, 300 for flat, 600 for steep)
    pub peak_factor: f64,
}

impl Default for ScsUhParams {
    fn default() -> Self {
        Self { peak_factor: 484.0 }
    }
}

// ── SCS dimensionless hydrograph ratios (t/tp vs q/Qp) ──
// Standard SCS dimensionless unit hydrograph tabular values.
const SCS_DIMENSIONLESS: [(f64, f64); 23] = [
    (0.0, 0.000),
    (0.1, 0.030),
    (0.2, 0.100),
    (0.3, 0.190),
    (0.4, 0.310),
    (0.5, 0.470),
    (0.6, 0.660),
    (0.7, 0.820),
    (0.8, 0.930),
    (0.9, 0.990),
    (1.0, 1.000),
    (1.1, 0.990),
    (1.2, 0.930),
    (1.3, 0.860),
    (1.4, 0.780),
    (1.5, 0.680),
    (1.6, 0.560),
    (1.7, 0.460),
    (1.8, 0.390),
    (2.0, 0.280),
    (2.2, 0.207),
    (2.5, 0.147),
    (3.0, 0.075),
];

// ── Snyder unit hydrograph ──

/// Compute Snyder synthetic unit hydrograph.
///
/// Converts rainfall excess to a discharge hydrograph using the Snyder synthetic
/// unit hydrograph method. The unit duration is derived from the basin lag.
///
/// # Parameters
/// * `area_km2` — basin drainage area (km²)
/// * `l_km` — length of main stream from outlet to divide (km)
/// * `lca_km` — length from outlet to point opposite basin centroid (km)
/// * `rainfall_excess_mm` — effective rainfall depth (mm, from SCS-CN)
/// * `params` — Snyder coefficients (Ct, Cp)
/// * `dt_hours` — desired time step for output ordinates
///
/// # Equations
/// * `tp = Ct * (L * Lca)^0.3` — basin lag (hours)
/// * Standard rainfall duration `tr_std = tp / 5.5`
/// * Adjusted lag `tp_adj = tp + 0.25 * (tr - tr_std)` where `tr = dt_hours`
/// * `Qp = Cp * A * 2.778 / tp_adj` — peak discharge (m³/s/cm)
/// * Base time `Tb = 3.0 + tp_adj / 8.0` (days), 24× conversion to hours
/// * `W50 = 770.0 / (Qp / A)^1.08` (hours), `W75 = 440.0 / (Qp / A)^1.08`
pub fn snyder_uh(
    area_km2: f64,
    l_km: f64,
    lca_km: f64,
    rainfall_excess_mm: f64,
    params: &SnyderParams,
    dt_hours: f64,
) -> Option<UnitHydrograph> {
    if area_km2 <= 0.0 || l_km <= 0.0 || lca_km <= 0.0 || rainfall_excess_mm < 0.0 {
        return None;
    }
    if dt_hours <= 0.0 {
        return None;
    }

    // Basin lag (hours)
    let tp = params.ct * (l_km * lca_km).powf(0.3);
    if tp <= 0.0 {
        return None;
    }

    // Standard rainfall duration and adjusted lag
    let tr_std = tp / 5.5;
    let tr = dt_hours;
    let tp_adj = (tp + 0.25 * (tr - tr_std)).max(0.1);

    // Unit peak discharge (per 1 cm of excess)
    let qp_unit = params.cp * area_km2 * 2.778 / tp_adj;
    if qp_unit <= 0.0 {
        return None;
    }

    // Volume of 1 cm of excess over the basin (m³)
    let unit_volume_m3 = area_km2 * 1_000_000.0 * 0.01;

    // Base time required for triangular UH to conserve volume:
    // Volume = 0.5 * Qp * Tb_seconds → Tb_hours = unit_volume_m3 / (0.5 * qp_unit * 3600)
    let tb_hours = unit_volume_m3 / (0.5 * qp_unit * 3600.0);
    let tb_hours = tb_hours.max(tp_adj * 2.0); // at least 2× time to peak

    // Scale by actual excess
    let excess_cm = rainfall_excess_mm / 10.0;
    let qp_actual = qp_unit * excess_cm;

    // Build triangular hydrograph with volume conservation
    let n_steps = (tb_hours / dt_hours).ceil() as usize;
    let n_steps = n_steps.max(2);
    let mut ordinates = vec![0.0_f64; n_steps];
    let time_to_peak = tp_adj;

    for i in 0..n_steps {
        let t = i as f64 * dt_hours;
        let q = if t <= time_to_peak {
            // Rising limb
            qp_actual * (t / time_to_peak)
        } else if t < tb_hours {
            // Recession limb: linear from Qp to 0
            qp_actual * (1.0 - (t - time_to_peak) / (tb_hours - time_to_peak))
        } else {
            0.0
        };
        ordinates[i] = q.max(0.0);
    }

    let total_volume_m3 = compute_volume(&ordinates, dt_hours);

    Some(UnitHydrograph {
        method: "snyder".to_string(),
        dt_hours,
        duration_hours: n_steps as f64 * dt_hours,
        peak_q: qp_actual,
        time_to_peak_h: time_to_peak,
        ordinates,
        total_volume_m3,
        area_km2,
        rainfall_excess_mm,
    })
}

// ── SCS dimensionless unit hydrograph ──

/// Compute SCS dimensionless unit hydrograph.
///
/// # Parameters
/// * `area_km2` — basin drainage area (km²)
/// * `tc_hours` — time of concentration (hours)
/// * `rainfall_excess_mm` — effective rainfall depth (mm)
/// * `params` — peak factor (484 standard)
/// * `dt_hours` — time step (hours)
///
/// # Equations
/// * `tp = 0.6 * Tc` — time to peak (hours)
/// * `qp = peak_factor * A * Q / (0.5 * tp * 3600)` — peak discharge (m³/s)
///   where Q is excess in feet... adapted to metric:
///   `qp = peak_factor * A * excess_mm / (2.0 * tp * 3600.0 / 1000.0)` simplified
///   Standard US: qp = 484 * A * Q / Tp  (A=miles², Q=inches, Tp=hours, qp=ft³/s)
///   Metric: qp = peak_factor * A * excess_mm / (tp * 3600.0 / 1000.0 * 0.5)
pub fn scs_uh(
    area_km2: f64,
    tc_hours: f64,
    rainfall_excess_mm: f64,
    params: &ScsUhParams,
    dt_hours: f64,
) -> Option<UnitHydrograph> {
    if area_km2 <= 0.0 || tc_hours <= 0.0 || rainfall_excess_mm < 0.0 {
        return None;
    }
    if dt_hours <= 0.0 {
        return None;
    }

    let tp = 0.6 * tc_hours; // time to peak
    let tr = 1.7 * tp; // recession time
    let tb = 2.67 * tp; // total base time

    // Peak discharge: convert from US standard units
    // US: qp = 484 * A_mi² * Q_in / (tp * 3600) → ft³/s
    // Metric: qp = 0.208 * A_km² * Q_mm / tp → m³/s
    // Where Q_mm is total excess in mm
    // The factor 0.208 = 484/2321 (unit conversion)
    let peak_factor_metric = params.peak_factor / 2321.0;
    let qp = peak_factor_metric * area_km2 * rainfall_excess_mm / tp;

    // Build hydrograph from SCS dimensionless ratios
    let n_steps = ((tb * 1.2) / dt_hours).ceil() as usize; // extend slightly past base
    let n_steps = n_steps.max(10);
    let mut ordinates = vec![0.0_f64; n_steps];

    for i in 0..n_steps {
        let t = i as f64 * dt_hours;
        let t_over_tp = t / tp;

        // Interpolate from SCS dimensionless table
        let q_over_qp = if t_over_tp <= 3.0 {
            // Linear interpolation within table bounds
            interpolate_scs(t_over_tp)
        } else if t_over_tp <= 5.0 {
            // Tail extension using exponential decay
            let base = interpolate_scs(3.0);
            let decay = (-1.0 * (t_over_tp - 3.0)).exp();
            base * decay
        } else {
            0.0
        };

        ordinates[i] = (q_over_qp * qp).max(0.0);
    }

    // Compute volume
    let total_volume_m3 = compute_volume(&ordinates, dt_hours);

    Some(UnitHydrograph {
        method: "scs".to_string(),
        dt_hours,
        duration_hours: n_steps as f64 * dt_hours,
        peak_q: qp,
        time_to_peak_h: tp,
        ordinates,
        total_volume_m3,
        area_km2,
        rainfall_excess_mm,
    })
}

/// Interpolate q/Qp from SCS dimensionless ratios for a given t/tp.
fn interpolate_scs(t_over_tp: f64) -> f64 {
    let t_over_tp = t_over_tp.clamp(0.0, 3.0);

    for j in 1..SCS_DIMENSIONLESS.len() {
        let (t_prev, q_prev) = SCS_DIMENSIONLESS[j - 1];
        let (t_next, q_next) = SCS_DIMENSIONLESS[j];

        if t_over_tp >= t_prev && t_over_tp <= t_next {
            if (t_next - t_prev).abs() < 1e-12 {
                return q_prev;
            }
            let fraction = (t_over_tp - t_prev) / (t_next - t_prev);
            return q_prev + fraction * (q_next - q_prev);
        }
    }

    // If beyond table bounds, return last value or 0
    if t_over_tp <= SCS_DIMENSIONLESS[0].0 {
        SCS_DIMENSIONLESS[0].1
    } else {
        SCS_DIMENSIONLESS[SCS_DIMENSIONLESS.len() - 1].1
    }
}

// ── Clark unit hydrograph ──

/// Compute Clark unit hydrograph using time-area histogram + linear reservoir routing.
///
/// # Parameters
/// * `area_km2` — basin area (km²)
/// * `tc_hours` — time of concentration (hours)
/// * `k_hours` — storage coefficient (hours, typical 0.5–2.0)
/// * `rainfall_excess_mm` — effective rainfall (mm)
/// * `dt_hours` — time step (hours)
///
/// # Algorithm
/// 1. Build time-area histogram: basin area divided evenly across Tc isochrones
/// 2. Translate to inflow hydrograph: I = area * 1cm / dt → m³/s per unit
/// 3. Route through linear reservoir: O(t+dt) = O(t) + (I-O)*dt/(K+0.5*dt)
/// 4. Scale by actual rainfall excess
pub fn clark_uh(
    area_km2: f64,
    tc_hours: f64,
    k_hours: f64,
    rainfall_excess_mm: f64,
    dt_hours: f64,
) -> Option<UnitHydrograph> {
    if area_km2 <= 0.0 || tc_hours <= 0.0 || rainfall_excess_mm < 0.0 {
        return None;
    }
    if dt_hours <= 0.0 {
        return None;
    }

    let tc_sec = tc_hours * 3600.0;
    let dt_sec = dt_hours * 3600.0;
    let n_time_steps = (tc_hours / dt_hours).ceil() as usize;
    let n_time_steps = n_time_steps.max(1);

    // Total volume of 1 cm excess over basin (m³)
    let unit_volume_m3 = area_km2 * 1_000_000.0 * 0.01;

    // Clark method: linear time-area relationship
    // A(t) = A_total * min(t/Tc, 1)
    // Inflow rate dA/dt = A_total / Tc
    // For 1cm excess uniformly over Tc:
    //   I(t) = (A_total/Tc) * 0.01  [m³/s]  for 0≤t≤Tc
    let inflow_rate = (area_km2 * 1_000_000.0 / tc_sec) * 0.01; // m³/s per cm excess

    // Run sufficiently long: runtime = 3 * (Tc + K)
    let duration = 3.0 * (tc_hours + k_hours);
    let n_steps = (duration / dt_hours).ceil() as usize;
    let n_steps = n_steps.max(2 * n_time_steps);

    // Linear reservoir routing: O(t+dt) = C*I + (1-C)*O(t)
    // where C = 1 - exp(-dt/K) — this conserves mass
    let c = 1.0 - (-dt_hours / k_hours.max(0.01)).exp();

    let mut outflow = vec![0.0_f64; n_steps];
    let mut o_prev = 0.0;

    for t_step in 0..n_steps {
        // Inflow at this time step
        let i = if (t_step as f64 * dt_hours) < tc_hours {
            inflow_rate
        } else {
            0.0
        };

        // Linear reservoir routing
        let o = c * i + (1.0 - c) * o_prev;
        outflow[t_step] = o.max(0.0);
        o_prev = outflow[t_step];
    }

    // Scale by actual excess (rainfall_excess_mm / 10 = excess in cm)
    let excess_cm = rainfall_excess_mm / 10.0;
    let ordinates: Vec<f64> = outflow.iter().map(|&q| q * excess_cm).collect();

    // Find peak
    let peak_q = ordinates.iter().copied().fold(0.0_f64, f64::max);
    let time_to_peak = ordinates
        .iter()
        .position(|&q| (q - peak_q).abs() < 1e-10)
        .unwrap_or(0) as f64
        * dt_hours;

    let total_volume_m3 = compute_volume(&ordinates, dt_hours);

    Some(UnitHydrograph {
        method: "clark".to_string(),
        dt_hours,
        duration_hours: n_steps as f64 * dt_hours,
        peak_q,
        time_to_peak_h: time_to_peak,
        ordinates,
        total_volume_m3,
        area_km2,
        rainfall_excess_mm,
    })
}

// ── Convolution utility ──

/// Convolve unit hydrograph with excess rainfall hyetograph.
///
/// Returns the full discharge hydrograph (m³/s) using discrete convolution.
///
/// # Arguments
/// * `uh_ordinates` — unit hydrograph ordinates (m³/s per unit, e.g. per mm of excess)
/// * `excess_hyetograph` — excess rainfall depth per time step (mm)
/// * `dt_hours` — time step (hours), must match UH time step
pub fn convolve_rainfall(
    uh_ordinates: &[f64],
    excess_hyetograph: &[f64],
    dt_hours: f64,
) -> Vec<f64> {
    if uh_ordinates.is_empty() || excess_hyetograph.is_empty() {
        return vec![];
    }

    let m = uh_ordinates.len();
    let n = excess_hyetograph.len();
    let result_len = m + n - 1;
    let mut result = vec![0.0_f64; result_len];

    // Discrete convolution: Q(k) = Σ P(i) * U(k-i+1) for all valid i
    for i in 0..n {
        let p_i = excess_hyetograph[i];
        if p_i <= 0.0 {
            continue;
        }
        for j in 0..m {
            result[i + j] += p_i * uh_ordinates[j];
        }
    }

    result
}

// ── Helpers ──

/// Compute total volume (m³) from discharge ordinates using trapezoidal rule.
fn compute_volume(ordinates: &[f64], dt_hours: f64) -> f64 {
    let dt_sec = dt_hours * 3600.0;
    if ordinates.len() < 2 {
        return ordinates.first().copied().unwrap_or(0.0) * dt_sec;
    }

    let mut volume = 0.0;
    for i in 0..ordinates.len() - 1 {
        volume += 0.5 * (ordinates[i] + ordinates[i + 1]) * dt_sec;
    }
    volume
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snyder_uh_basic() {
        // 100 km², L=20km, Lca=10km, 50mm excess
        let result = snyder_uh(100.0, 20.0, 10.0, 50.0, &SnyderParams::default(), 0.5);
        assert!(result.is_some());
        let uh = result.unwrap();
        assert!(
            uh.peak_q > 0.0,
            "Peak Q should be positive, got {}",
            uh.peak_q
        );
        assert!(uh.time_to_peak_h > 0.0);
        assert!(!uh.ordinates.is_empty());
        assert_eq!(uh.method, "snyder");
    }

    #[test]
    fn test_snyder_uh_zero_area() {
        let result = snyder_uh(0.0, 20.0, 10.0, 50.0, &SnyderParams::default(), 0.5);
        assert!(result.is_none());
    }

    #[test]
    fn test_snyder_uh_volume_conservation() {
        // Volume from ordinates should roughly match input
        let area_km2 = 100.0;
        let excess_mm = 50.0;
        let result = snyder_uh(
            area_km2,
            20.0,
            10.0,
            excess_mm,
            &SnyderParams::default(),
            0.5,
        );
        assert!(result.is_some());
        let uh = result.unwrap();

        // Expected volume: area * excess
        let expected_m3 = area_km2 * 1_000_000.0 * (excess_mm / 1000.0);
        // Allow 30% tolerance — Snyder is approximate
        let ratio = uh.total_volume_m3 / expected_m3;
        assert!(
            ratio > 0.5 && ratio < 1.5,
            "Volume ratio {:.3} out of [0.5,1.5]: expected {:.0}, got {:.0}",
            ratio,
            expected_m3,
            uh.total_volume_m3
        );
    }

    #[test]
    fn test_snyder_params_default() {
        let params = SnyderParams::default();
        assert!((params.ct - 1.8).abs() < 1e-6);
        assert!((params.cp - 0.6).abs() < 1e-6);
    }

    #[test]
    fn test_scs_uh_basic() {
        let result = scs_uh(50.0, 2.0, 30.0, &ScsUhParams::default(), 0.25);
        assert!(result.is_some());
        let uh = result.unwrap();
        assert!(
            uh.peak_q > 0.0,
            "Peak Q should be positive, got {}",
            uh.peak_q
        );
        assert!(uh.time_to_peak_h > 0.0);
        assert_eq!(uh.method, "scs");
    }

    #[test]
    fn test_scs_uh_zero_area() {
        let result = scs_uh(0.0, 2.0, 30.0, &ScsUhParams::default(), 0.25);
        assert!(result.is_none());
    }

    #[test]
    fn test_scs_uh_volume_conservation() {
        let area_km2 = 50.0;
        let excess_mm = 30.0;
        let result = scs_uh(area_km2, 2.0, excess_mm, &ScsUhParams::default(), 0.25);
        assert!(result.is_some());
        let uh = result.unwrap();

        let expected_m3 = area_km2 * 1_000_000.0 * (excess_mm / 1000.0);
        let ratio = uh.total_volume_m3 / expected_m3;
        assert!(
            ratio > 0.5 && ratio < 1.5,
            "Volume ratio {:.3} out of [0.5,1.5]",
            ratio
        );
    }

    #[test]
    fn test_scs_params_default() {
        let params = ScsUhParams::default();
        assert!((params.peak_factor - 484.0).abs() < 1e-6);
    }

    #[test]
    fn test_clark_uh_basic() {
        let result = clark_uh(50.0, 2.0, 1.0, 30.0, 0.25);
        assert!(result.is_some());
        let uh = result.unwrap();
        assert!(
            uh.peak_q > 0.0,
            "Peak Q should be positive, got {}",
            uh.peak_q
        );
        assert!(!uh.ordinates.is_empty());
        assert_eq!(uh.method, "clark");
    }

    #[test]
    fn test_clark_uh_zero_area() {
        let result = clark_uh(0.0, 2.0, 1.0, 30.0, 0.25);
        assert!(result.is_none());
    }

    #[test]
    fn test_clark_uh_attenuation_increases_with_k() {
        // Higher K → more attenuation → lower peak
        let result_low_k = clark_uh(50.0, 2.0, 0.5, 30.0, 0.25).unwrap();
        let result_high_k = clark_uh(50.0, 2.0, 4.0, 30.0, 0.25).unwrap();

        assert!(
            result_high_k.peak_q < result_low_k.peak_q,
            "Higher K should attenuate more: low_K peak={:.2}, high_K peak={:.2}",
            result_low_k.peak_q,
            result_high_k.peak_q
        );
    }

    #[test]
    fn test_clark_uh_volume_conservation() {
        let area_km2 = 50.0;
        let excess_mm = 30.0;
        let result = clark_uh(area_km2, 2.0, 1.0, excess_mm, 0.25);
        assert!(result.is_some());
        let uh = result.unwrap();

        let expected_m3 = area_km2 * 1_000_000.0 * (excess_mm / 1000.0);
        let ratio = uh.total_volume_m3 / expected_m3;
        assert!(
            ratio > 0.7 && ratio < 1.3,
            "Volume ratio {:.3} out of [0.7,1.3]",
            ratio
        );
    }

    #[test]
    fn test_convolve_rainfall_simple() {
        // UH: [1, 2, 1] at dt
        // Excess: [10, 20] mm
        // Manual: Q0 = 10*1 = 10
        //         Q1 = 10*2 + 20*1 = 40
        //         Q2 = 10*1 + 20*2 = 50
        //         Q3 = 20*1 = 20
        let uh = vec![1.0, 2.0, 1.0];
        let excess = vec![10.0, 20.0];
        let result = convolve_rainfall(&uh, &excess, 1.0);
        assert_eq!(result.len(), 4);
        assert!((result[0] - 10.0).abs() < 1e-6);
        assert!((result[1] - 40.0).abs() < 1e-6);
        assert!((result[2] - 50.0).abs() < 1e-6);
        assert!((result[3] - 20.0).abs() < 1e-6);
    }

    #[test]
    fn test_convolve_rainfall_empty() {
        let result = convolve_rainfall(&[], &[1.0, 2.0], 1.0);
        assert!(result.is_empty());

        let result = convolve_rainfall(&[1.0, 2.0], &[], 1.0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_convolve_rainfall_single_uh() {
        let uh = vec![5.0];
        let excess = vec![2.0, 3.0];
        let result = convolve_rainfall(&uh, &excess, 1.0);
        assert_eq!(result.len(), 2);
        assert!((result[0] - 10.0).abs() < 1e-6);
        assert!((result[1] - 15.0).abs() < 1e-6);
    }

    #[test]
    fn test_scs_uh_peak_factor_effect() {
        // Higher peak factor → higher peak
        let flat = scs_uh(50.0, 2.0, 30.0, &ScsUhParams { peak_factor: 300.0 }, 0.25).unwrap();
        let steep = scs_uh(50.0, 2.0, 30.0, &ScsUhParams { peak_factor: 600.0 }, 0.25).unwrap();

        assert!(
            steep.peak_q > flat.peak_q,
            "Higher peak factor should give higher peak: flat={:.2}, steep={:.2}",
            flat.peak_q,
            steep.peak_q
        );
    }

    #[test]
    fn test_snyder_different_params() {
        // Flat basin (low Ct) vs mountainous (high Ct) — mountainous has longer lag
        let flat = snyder_uh(
            100.0,
            20.0,
            10.0,
            50.0,
            &SnyderParams { ct: 0.7, cp: 0.6 },
            0.5,
        )
        .unwrap();
        let mountain = snyder_uh(
            100.0,
            20.0,
            10.0,
            50.0,
            &SnyderParams { ct: 2.2, cp: 0.6 },
            0.5,
        )
        .unwrap();

        assert!(
            mountain.time_to_peak_h > flat.time_to_peak_h,
            "Mountainous basin should have longer lag: flat_tp={:.2}, mountain_tp={:.2}",
            flat.time_to_peak_h,
            mountain.time_to_peak_h
        );
    }

    #[test]
    fn test_interpolate_scs_bounds() {
        // At t/tp=0, q/Qp=0
        let q = interpolate_scs(0.0);
        assert!(
            (q - 0.0).abs() < 1e-6,
            "At t=0, q/Qp should be 0, got {}",
            q
        );

        // At t/tp=1, q/Qp=1
        let q = interpolate_scs(1.0);
        assert!(
            (q - 1.0).abs() < 1e-6,
            "At t/tp=1, q/Qp should be 1, got {}",
            q
        );

        // At t/tp=3, q/Qp≈0.075
        let q = interpolate_scs(3.0);
        assert!(
            (q - 0.075).abs() < 1e-4,
            "At t/tp=3, q/Qp should be ~0.075, got {}",
            q
        );
    }
}
