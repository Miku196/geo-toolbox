//! Soil erosion (USLE/RUSLE) and soil organic carbon dynamics.
use geo_core::errors::GeoResult;
use serde::Serialize;

// ── USLE Soil Erosion ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct UsleResult {
    /// Rainfall erosivity factor R (MJ·mm/ha·hr·yr)
    pub r_factor: f64,
    /// Soil erodibility factor K (t·ha·hr/ha/MJ/mm)
    pub k_factor: f64,
    /// Slope length and steepness factor LS
    pub ls_factor: f64,
    /// Cover management factor C
    pub c_factor: f64,
    /// Support practice factor P
    pub p_factor: f64,
    /// Annual soil loss A (t/ha/yr)
    pub soil_loss_t_ha_yr: f64,
    /// Erosion risk class
    pub risk_class: String,
    /// Annual soil loss in mm/yr (assuming bulk density 1.3 t/m³)
    pub soil_loss_mm_yr: f64,
}

/// Compute soil loss using the Universal Soil Loss Equation (USLE).
///
/// A = R × K × LS × C × P
pub fn usle_erosion(
    r_factor: f64,
    k_factor: f64,
    ls_factor: f64,
    c_factor: f64,
    p_factor: f64,
) -> UsleResult {
    let a = r_factor * k_factor * ls_factor * c_factor * p_factor;
    let mm_yr = a / 13.0; // assume bulk density ~1.3 t/m³ → 1mm soil ≈ 13 t/ha

    let risk = if a < 2.0 {
        "Very low"
    } else if a < 5.0 {
        "Low"
    } else if a < 10.0 {
        "Moderate"
    } else if a < 20.0 {
        "High"
    } else if a < 50.0 {
        "Very high"
    } else {
        "Severe"
    };

    UsleResult {
        r_factor,
        k_factor,
        ls_factor,
        c_factor,
        p_factor,
        soil_loss_t_ha_yr: (a * 100.0).round() / 100.0,
        risk_class: risk.to_string(),
        soil_loss_mm_yr: (mm_yr * 1000.0).round() / 1000.0,
    }
}

/// Estimate LS factor from slope length and gradient.
/// LS = (λ/22.13)^m × (65.41 sin²θ + 4.56 sinθ + 0.065)
/// where m = 0.5 for slope ≥5%, 0.4 for 3-5%, 0.3 for 1-3%, 0.2 for <1%
pub fn ls_factor(slope_length_m: f64, slope_pct: f64) -> f64 {
    let theta = (slope_pct / 100.0).atan();
    let sin_theta = theta.sin();
    let m = if slope_pct >= 5.0 {
        0.5
    } else if slope_pct >= 3.0 {
        0.4
    } else if slope_pct >= 1.0 {
        0.3
    } else {
        0.2
    };
    (slope_length_m / 22.13).powf(m) * (65.41 * sin_theta * sin_theta + 4.56 * sin_theta + 0.065)
}

/// Estimate R factor (rainfall erosivity) from annual precipitation.
/// Simplified Wischmeier formula for temperate climates.
pub fn r_factor_annual(precip_mm: f64, max_30min_intensity_mm_hr: f64) -> f64 {
    // EI₃₀ = kinetic energy × max 30-min intensity
    // Simplified: R ≈ 0.5 × P × I₃₀ / 100
    precip_mm * max_30min_intensity_mm_hr * 0.005
}

/// Estimate K factor (soil erodibility) from soil texture.
/// Based on Wischmeier nomograph approximation.
pub fn k_factor_texture(
    silt_pct: f64,
    sand_pct: f64,
    clay_pct: f64,
    organic_matter_pct: f64,
) -> f64 {
    // Simplified K factor estimation
    let m = (silt_pct + sand_pct * (100.0 - clay_pct) / 100.0) / 100.0;
    let k1 = 2.1e-4 * (12.0 - organic_matter_pct) * m.powf(1.14);
    let k2 = 3.25 * (2.0 - 2.0); // structure code (simplified)
    let k3 = 2.5 * (3.0 - 3.0); // permeability code (simplified)
    (k1 + k2 + k3).max(0.01).min(0.7)
}

// ── Soil Organic Carbon Dynamics ──────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SoilCarbonResult {
    /// Initial SOC stock (tC/ha)
    pub initial_soc_tc_ha: f64,
    /// Annual carbon input (tC/ha/yr)
    pub c_input_tc_ha_yr: f64,
    /// Decomposition rate constant k (1/yr)
    pub k_decay: f64,
    /// Equilibrium SOC (tC/ha)
    pub equilibrium_soc_tc_ha: f64,
    /// SOC after 20 years (tC/ha)
    pub soc_20yr_tc_ha: f64,
    /// Annual sequestration rate (tCO₂/ha/yr)
    pub seq_rate_tco2_ha_yr: f64,
}

/// Model soil organic carbon dynamics using first-order kinetics (Hénin-Dupuis / RothC simplified).
///
/// dC/dt = I - k*C
/// C(t) = Ceq + (C₀ - Ceq)*exp(-k*t),  where Ceq = I/k
pub fn soil_carbon_dynamics(
    initial_soc_tc_ha: f64,
    c_input_tc_ha_yr: f64,
    k_decay: f64,
) -> SoilCarbonResult {
    let ceq = if k_decay > 0.0 {
        c_input_tc_ha_yr / k_decay
    } else {
        initial_soc_tc_ha
    };
    let t = 20.0;
    let soc_20yr = ceq + (initial_soc_tc_ha - ceq) * (-k_decay * t).exp();
    let seq_rate = ((soc_20yr - initial_soc_tc_ha) / t * 44.0 / 12.0).max(0.0);

    SoilCarbonResult {
        initial_soc_tc_ha,
        c_input_tc_ha_yr,
        k_decay,
        equilibrium_soc_tc_ha: (ceq * 100.0).round() / 100.0,
        soc_20yr_tc_ha: (soc_20yr * 100.0).round() / 100.0,
        seq_rate_tco2_ha_yr: (seq_rate * 100.0).round() / 100.0,
    }
}

/// Estimate decomposition rate k from climate (mean annual temperature).
/// k ≈ k₀ * Q₁₀^(T/10), where k₀ = 0.01, Q₁₀ = 2
pub fn k_from_temperature(mean_annual_temp_c: f64) -> f64 {
    0.01 * 2.0_f64.powf(mean_annual_temp_c / 10.0)
}

/// Estimate carbon input from crop residue.
/// C_input = yield * harvest_index * residue_c_fraction
pub fn crop_residue_c_input(yield_kg_ha: f64, harvest_index: f64, residue_c_fraction: f64) -> f64 {
    let residue_kg_ha = yield_kg_ha / harvest_index.max(0.01) - yield_kg_ha;
    residue_kg_ha * residue_c_fraction / 1000.0 // kg → tC/ha
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usle() {
        let r = usle_erosion(100.0, 0.3, 2.0, 0.2, 1.0);
        assert!(r.soil_loss_t_ha_yr > 0.0);
    }

    #[test]
    fn test_ls_factor() {
        let ls = ls_factor(100.0, 10.0);
        assert!(ls > 1.0);
    }

    #[test]
    fn test_soc_dynamics() {
        let r = soil_carbon_dynamics(50.0, 2.0, 0.05);
        assert!(r.soc_20yr_tc_ha > 0.0);
    }

    #[test]
    fn test_k_from_temp() {
        let k = k_from_temperature(15.0);
        assert!(k > 0.01 && k < 0.1);
    }
}
