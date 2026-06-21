//! NREL PVWatts v5 solar performance model.
//!
//! Includes: cell temperature (Sandia), DC/AC power, losses,
//! annual energy estimation, performance ratio.

/// Sandia cell temperature model.
/// Tc = Ta + G/(U0 + U1*Ws)
pub fn pvwatts_cell_temperature(
    poa_irradiance_w_m2: f64,
    ambient_temp_c: f64,
    wind_speed_m_s: f64,
    mounting: &str,
) -> f64 {
    let (u0, u1) = match mounting.to_lowercase().as_str() {
        "open_rack" | "open rack" => (25.0, 6.84),
        "roof_mount" | "roof mount" | "flush_mount" => (25.0, 1.0),
        "insulated" | "bipv" | "building_integrated" => (25.0, 0.0),
        _ => (25.0, 6.84),
    };
    ambient_temp_c + poa_irradiance_w_m2 / (u0 + u1 * wind_speed_m_s)
}

/// DC power output from module.
/// Pdc = (G/1000) * Pdc0 * (1 + γ/100 * (Tc - 25))
pub fn pvwatts_dc_power(
    poa_irradiance_w_m2: f64,
    module_capacity_kw: f64,
    temperature_c: f64,
    temp_coefficient_pct_per_c: f64,
) -> f64 {
    if poa_irradiance_w_m2 <= 0.0 {
        return 0.0;
    }
    (poa_irradiance_w_m2 / 1000.0)
        * module_capacity_kw
        * (1.0 + temp_coefficient_pct_per_c / 100.0 * (temperature_c - 25.0))
}

/// AC power from inverter.
pub fn pvwatts_ac_power(dc_power_kw: f64, inverter_efficiency: f64) -> f64 {
    dc_power_kw * inverter_efficiency
}

/// Total system losses (fraction of 1).
pub fn pvwatts_losses(
    soiling_pct: f64,
    shading_pct: f64,
    snow_pct: f64,
    mismatch_pct: f64,
    wiring_pct: f64,
    connections_pct: f64,
    lid_pct: f64,
    nameplate_pct: f64,
    age_pct: f64,
    availability_pct: f64,
) -> f64 {
    let losses = [
        soiling_pct,
        shading_pct,
        snow_pct,
        mismatch_pct,
        wiring_pct,
        connections_pct,
        lid_pct,
        nameplate_pct,
        age_pct,
        availability_pct,
    ];
    let mut total_loss = 1.0;
    for &loss in &losses {
        total_loss *= 1.0 - loss / 100.0;
    }
    1.0 - total_loss
}

/// Monthly days for each month.
const MONTH_DAYS: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// Estimate annual energy production.
pub fn pvwatts_annual_energy(
    monthly_poa: &[f64],
    module_capacity_kw: f64,
    monthly_temp: &[f64],
    monthly_wind: &[f64],
    mounting: &str,
    inverter_eff: f64,
    dc_ac_ratio: f64,
    temp_coeff: f64,
    losses_pct: f64,
) -> serde_json::Value {
    if monthly_poa.len() < 12 {
        return serde_json::json!({"error": "monthly_poa requires 12 months"});
    }

    let loss_factor = 1.0 - losses_pct / 100.0;
    let mut monthly_kwh = Vec::with_capacity(12);
    let mut annual_kwh = 0.0;
    let mut monthly_ac = Vec::with_capacity(12);

    for m in 0..12 {
        let g = monthly_poa[m];
        let ta = if m < monthly_temp.len() {
            monthly_temp[m]
        } else {
            20.0
        };
        let ws = if m < monthly_wind.len() {
            monthly_wind[m]
        } else {
            3.0
        };

        let tc = pvwatts_cell_temperature(g, ta, ws, mounting);
        let pdc = pvwatts_dc_power(g, module_capacity_kw, tc, temp_coeff);
        let mut pac = pvwatts_ac_power(pdc, inverter_eff);

        // DC/AC ratio cap: inverter max = nameplate / ratio
        let inverter_max_kw = module_capacity_kw / dc_ac_ratio;
        if pac > inverter_max_kw {
            pac = inverter_max_kw;
        }

        let days = MONTH_DAYS[m] as f64;
        // Effective sun hours = POA_kW/m² * loss_factor
        // Energy = Pac_kW * (POA/1000) * hours_of_daylight
        // Simplified: Energy = Pac * days * (G/1000) * 24 * loss_factor
        let sun_hours = days * 24.0; // total hours in the month
        let energy = pac * sun_hours * loss_factor;
        monthly_kwh.push((energy * 100.0).round() / 100.0);
        monthly_ac.push(pac);
        annual_kwh += energy;
    }

    // Capacity factor = annual_kWh / (nameplate_kW * 8760)
    let capacity_factor = if module_capacity_kw > 0.0 {
        annual_kwh / (module_capacity_kw * 8760.0)
    } else {
        0.0
    };

    serde_json::json!({
        "annual_kwh": (annual_kwh * 100.0).round() / 100.0,
        "monthly_kwh": monthly_kwh,
        "capacity_factor": (capacity_factor * 10000.0).round() / 10000.0,
        "module_capacity_kw": module_capacity_kw,
        "inverter_efficiency": inverter_eff,
        "dc_ac_ratio": dc_ac_ratio,
        "total_losses_pct": (losses_pct * 100.0).round() / 100.0,
    })
}

/// Performance ratio: Yf / Yr.
/// Yf = annual_kWh / module_capacity_kW (yield factor, kWh/kWp)
/// Yr = annual_poa_kWh_m2 (reference yield, kWh/m²)
pub fn pvwatts_performance_ratio(
    annual_kwh: f64,
    module_capacity_kw: f64,
    annual_poa_kwh_m2: f64,
) -> f64 {
    if module_capacity_kw <= 0.0 || annual_poa_kwh_m2 <= 0.0 {
        return 0.0;
    }
    let yf = annual_kwh / module_capacity_kw;
    yf / annual_poa_kwh_m2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_temperature_open_rack() {
        // 1000 W/m², 25°C ambient, 0 m/s wind
        let tc = pvwatts_cell_temperature(1000.0, 25.0, 0.0, "open_rack");
        // Tc = 25 + 1000/25 = 65
        assert!((tc - 65.0).abs() < 0.1, "got {tc}");
    }

    #[test]
    fn test_cell_temperature_roof_mount() {
        let tc = pvwatts_cell_temperature(800.0, 30.0, 2.0, "roof_mount");
        // Tc = 30 + 800/(25 + 1*2) = 30 + 800/27 = 59.63
        assert!((tc - 59.63).abs() < 0.1, "got {tc}");
    }

    #[test]
    fn test_dc_power() {
        // STC: 1000 W/m², 1 kW module, 25°C, -0.35%/°C
        let pdc = pvwatts_dc_power(1000.0, 1.0, 25.0, -0.35);
        assert!((pdc - 1.0).abs() < 0.01, "got {pdc}");
    }

    #[test]
    fn test_dc_power_hot() {
        // 1000 W/m², 1 kW, 45°C (hot)
        let pdc = pvwatts_dc_power(1000.0, 1.0, 45.0, -0.35);
        // 1.0 * (1 + (-0.35/100)*(45-25)) = 1.0 * (1 - 0.0035*20) = 0.93
        assert!((pdc - 0.93).abs() < 0.01, "got {pdc}");
    }

    #[test]
    fn test_ac_power() {
        let pac = pvwatts_ac_power(1.0, 0.96);
        assert!((pac - 0.96).abs() < 0.01);
    }

    #[test]
    fn test_losses() {
        // 14% typical total loss
        let l = pvwatts_losses(2.0, 3.0, 0.0, 2.0, 0.5, 0.5, 0.5, 1.0, 0.0, 1.0);
        // 1 - (1-0.02)*(1-0.03)*(1-0)*(1-0.02)*(1-0.005)^3*(1-0.01)^2
        assert!(l > 0.08 && l < 0.15, "got {l}");
    }

    #[test]
    fn test_annual_energy() {
        let poa = vec![120.0; 12]; // 120 kWh/m²/month
        let temp = vec![20.0; 12];
        let wind = vec![3.0; 12];
        let r = pvwatts_annual_energy(&poa, 1.0, &temp, &wind, "open_rack", 0.96, 1.1, -0.35, 14.0);
        let annual_kwh = r["annual_kwh"].as_f64().unwrap();
        assert!(
            annual_kwh > 400.0 && annual_kwh < 1500.0,
            "got {annual_kwh}"
        );
    }

    #[test]
    fn test_performance_ratio() {
        let pr = pvwatts_performance_ratio(1500.0, 1.0, 1800.0);
        assert!((pr - 1500.0 / 1800.0).abs() < 0.01);
    }
}
