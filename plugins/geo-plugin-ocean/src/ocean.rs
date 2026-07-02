//! Oceanography: currents, coral bleaching, upwelling, fisheries.

use geo_core::errors::GeoResult;
use serde::Serialize;

use crate::OceanConfig;

// ── Coral Bleaching (Degree Heating Weeks) ───────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct CoralBleachingRisk {
    pub dhw: f64, // Degree Heating Weeks
    pub risk_level: String,
    pub alert_level: u8, // 0-4
    pub sst_anomaly_c: f64,
    pub description: String,
}

/// Calculate Degree Heating Weeks (DHW) from weekly SST data.
/// DHW = sum of SST anomalies > 1°C above max monthly mean over 12 weeks.
pub fn calc_dhw(weekly_sst: &[f64], max_monthly_mean: f64, threshold: f64) -> CoralBleachingRisk {
    let mut dhw = 0.0f64;
    let mut anomaly_sum = 0.0;
    let n = weekly_sst.len();

    for &sst in weekly_sst.iter().rev().take(12) {
        let anomaly = sst - max_monthly_mean;
        if anomaly > 1.0 {
            dhw += anomaly;
            anomaly_sum += anomaly;
        }
    }

    let mean_anomaly = if n > 0 {
        anomaly_sum / n.min(12) as f64
    } else {
        0.0
    };

    let (risk_level, alert_level) = if dhw >= threshold {
        if dhw >= 8.0 {
            (
                "Alert Level 2 — Widespread bleaching likely, significant mortality",
                2,
            )
        } else if dhw >= 4.0 {
            ("Alert Level 1 — Bleaching likely", 1)
        } else {
            ("Warning — Bleaching watch", 0)
        }
    } else {
        ("No stress — Bleaching not expected", 0)
    };

    CoralBleachingRisk {
        dhw: (dhw * 10.0).round() / 10.0,
        risk_level: risk_level.to_string(),
        alert_level,
        sst_anomaly_c: (mean_anomaly * 100.0).round() / 100.0,
        description: format!("DHW={:.1}°C-weeks, SST anomaly={:.2}°C", dhw, mean_anomaly),
    }
}

// ── Upwelling Index (Ekman Transport) ─────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct UpwellingIndex {
    pub ekman_transport_m2_s: f64,
    pub upwelling_index: f64,
    pub direction: String,
    pub productivity_potential: String,
}

/// Calculate upwelling index from wind stress (Ekman transport).
///
/// Ekman transport (m²/s) = τ / (ρ * f)
/// where τ = wind stress (N/m²), ρ = seawater density (1025 kg/m³),
/// f = Coriolis parameter = 2Ω sin(φ)
///
/// Positive = offshore transport (upwelling favorable).
pub fn ekman_upwelling(wind_stress_n_m2: f64, latitude_deg: f64) -> UpwellingIndex {
    let rho = 1025.0; // seawater density
    let omega = 7.2921e-5; // Earth's rotation rate
    let f = 2.0 * omega * (latitude_deg.to_radians().sin());
    if f.abs() < 1e-10 {
        return UpwellingIndex {
            ekman_transport_m2_s: 0.0,
            upwelling_index: 0.0,
            direction: "Equatorial — no Coriolis".into(),
            productivity_potential: "Variable".into(),
        };
    }
    let transport = wind_stress_n_m2 / (rho * f.abs());
    let ui = transport.abs();

    let direction = if wind_stress_n_m2 * f > 0.0 {
        "Offshore (upwelling favorable)"
    } else {
        "Onshore (downwelling)"
    };

    let potential = if ui > 2.0 {
        "High — strong upwelling"
    } else if ui > 1.0 {
        "Moderate"
    } else if ui > 0.3 {
        "Low"
    } else {
        "Negligible"
    };

    UpwellingIndex {
        ekman_transport_m2_s: (transport * 10000.0).round() / 10000.0,
        upwelling_index: (ui * 100.0).round() / 100.0,
        direction: direction.to_string(),
        productivity_potential: potential.to_string(),
    }
}

// ── Geostrophic Current ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct GeostrophicCurrent {
    pub u_m_s: f64, // zonal component (eastward positive)
    pub v_m_s: f64, // meridional component (northward positive)
    pub speed_m_s: f64,
    pub direction_deg: f64, // oceanographic convention (0=North, clockwise)
}

/// Estimate surface geostrophic current from sea surface height gradient.
///
/// u = -(g/f) * ∂η/∂y,  v = (g/f) * ∂η/∂x
/// where g = gravity, f = Coriolis, η = sea surface height anomaly
pub fn geostrophic_current(
    dssh_dx: f64, // SSH gradient in x (m/m)
    dssh_dy: f64, // SSH gradient in y (m/m)
    latitude_deg: f64,
) -> GeostrophicCurrent {
    let g = 9.81;
    let omega = 7.2921e-5;
    let f = 2.0 * omega * (latitude_deg.to_radians().sin());
    if f.abs() < 1e-10 {
        return GeostrophicCurrent {
            u_m_s: 0.0,
            v_m_s: 0.0,
            speed_m_s: 0.0,
            direction_deg: 0.0,
        };
    }
    let u = -g / f * dssh_dy;
    let v = g / f * dssh_dx;
    let speed = (u * u + v * v).sqrt();
    let dir = (u.atan2(v).to_degrees() + 360.0) % 360.0; // 0=North, clockwise

    GeostrophicCurrent {
        u_m_s: (u * 1000.0).round() / 1000.0,
        v_m_s: (v * 1000.0).round() / 1000.0,
        speed_m_s: (speed * 1000.0).round() / 1000.0,
        direction_deg: (dir * 10.0).round() / 10.0,
    }
}

// ── Sea Level Rise Projection ────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SlrProjection {
    pub current_rate_mm_yr: f64,
    pub acceleration_mm_yr2: f64,
    pub projected_2050_m: f64,
    pub projected_2100_m: f64,
    pub confidence: String,
}

/// Project sea level from historical rate and acceleration (quadratic model).
/// h(t) = h₀ + a*t + 0.5*b*t², where a=rate, b=acceleration, t=years from now.
pub fn project_slr(
    current_rate_mm_yr: f64,
    acceleration_mm_yr2: f64,
    base_year: u16,
) -> SlrProjection {
    let now = 2026u16;
    let t_2050 = (2050u16.saturating_sub(base_year)) as f64;
    let t_2100 = (2100u16.saturating_sub(base_year)) as f64;

    let h_2050 = current_rate_mm_yr * t_2050 + 0.5 * acceleration_mm_yr2 * t_2050 * t_2050;
    let h_2100 = current_rate_mm_yr * t_2100 + 0.5 * acceleration_mm_yr2 * t_2100 * t_2100;

    SlrProjection {
        current_rate_mm_yr,
        acceleration_mm_yr2,
        projected_2050_m: (h_2050 / 1000.0 * 100.0).round() / 100.0,
        projected_2100_m: (h_2100 / 1000.0 * 100.0).round() / 100.0,
        confidence: if acceleration_mm_yr2.abs() < 0.01 {
            "Low confidence (uncertain acceleration)".into()
        } else {
            "Moderate confidence".into()
        },
    }
}

// ── Fisheries Potential Index ────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct FisheriesPotential {
    pub chlorophyll_mg_m3: f64,
    pub sst_c: f64,
    pub potential_index: f64,
    pub fishery_type: String,
    pub description: String,
}

/// Estimate fisheries potential from chlorophyll-a (proxy for primary production) and SST.
///
/// Combines chlorophyll concentration and optimal temperature range for target species.
pub fn fisheries_potential(
    chlorophyll_mg_m3: f64,
    sst_c: f64,
    target_type: &str, // "pelagic", "demersal", "reef"
) -> FisheriesPotential {
    let chl_score = (chlorophyll_mg_m3 / 1.0).min(1.0); // normalize to 0-1

    let temp_score = match target_type {
        "pelagic" => {
            // Tuna-like: optimal 20-28°C
            if sst_c >= 20.0 && sst_c <= 28.0 {
                1.0
            } else if sst_c > 28.0 {
                (30.0 - sst_c) / 2.0
            } else {
                sst_c / 20.0
            }
        }
        "demersal" => {
            if sst_c >= 10.0 && sst_c <= 20.0 {
                1.0
            } else if sst_c > 20.0 {
                (25.0 - sst_c) / 5.0
            } else {
                sst_c / 10.0
            }
        }
        "reef" => {
            if sst_c >= 24.0 && sst_c <= 29.0 {
                1.0
            } else if sst_c > 29.0 {
                (32.0 - sst_c) / 3.0
            } else {
                sst_c / 24.0
            }
        }
        _ => {
            if sst_c >= 15.0 && sst_c <= 25.0 {
                1.0
            } else {
                0.5
            }
        }
    }
    .clamp(0.0, 1.0);

    let potential = (chl_score * 0.6 + temp_score * 0.4).clamp(0.0, 1.0);

    let fishery = match target_type {
        "pelagic" => "Pelagic (tuna, mackerel)",
        "demersal" => "Demersal (cod, flatfish)",
        "reef" => "Reef-associated (grouper, snapper)",
        _ => "General",
    };

    let desc = if potential > 0.7 {
        "High potential — favorable conditions"
    } else if potential > 0.4 {
        "Moderate potential"
    } else {
        "Low potential — suboptimal conditions"
    };

    FisheriesPotential {
        chlorophyll_mg_m3,
        sst_c,
        potential_index: (potential * 100.0).round() / 100.0,
        fishery_type: fishery.to_string(),
        description: desc.to_string(),
    }
}

// ── Ocean Acidification Index ─────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct AcidificationIndex {
    pub ph: f64,
    pub aragonite_saturation: f64,
    pub risk: String,
}

/// Estimate aragonite saturation state (Ω_arag) from pH and SST.
/// Simplified empirical relationship.
pub fn aragonite_saturation(ph: f64, sst_c: f64) -> AcidificationIndex {
    // Ω_arag ≈ exp(1.5 * pH - 0.02 * SST - 10)
    let omega = (1.5 * ph - 0.02 * sst_c - 10.0).exp();
    let risk = if omega < 1.0 {
        "Critical — dissolution likely"
    } else if omega < 2.0 {
        "Marginal — stress for calcifiers"
    } else if omega < 3.0 {
        "Adequate"
    } else {
        "Optimal"
    };

    AcidificationIndex {
        ph,
        aragonite_saturation: (omega * 100.0).round() / 100.0,
        risk: risk.to_string(),
    }
}

// ── Plugin ───────────────────────────────────────────────────────

pub struct OceanPlugin {
    pub config: OceanConfig,
}

impl OceanPlugin {
    pub fn new(config: OceanConfig) -> Self {
        Self { config }
    }
    pub fn coral_bleaching(&self, weekly_sst: &[f64], max_monthly_mean: f64) -> CoralBleachingRisk {
        calc_dhw(
            weekly_sst,
            max_monthly_mean,
            self.config.coral.dhw_bleaching_threshold,
        )
    }
    pub fn upwelling(&self, wind_stress: f64, lat: f64) -> UpwellingIndex {
        ekman_upwelling(wind_stress, lat)
    }
    pub fn geostrophic(&self, dx: f64, dy: f64, lat: f64) -> GeostrophicCurrent {
        geostrophic_current(dx, dy, lat)
    }
    pub fn slr(&self, rate: f64, accel: f64, base: u16) -> SlrProjection {
        project_slr(rate, accel, base)
    }
    pub fn fisheries(&self, chl: f64, sst: f64, t: &str) -> FisheriesPotential {
        fisheries_potential(chl, sst, t)
    }
    pub fn acidification(&self, ph: f64, sst: f64) -> AcidificationIndex {
        aragonite_saturation(ph, sst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dhw() {
        let sst = vec![30.0f64; 12];
        let r = calc_dhw(&sst, 28.0, 4.0);
        assert!(r.dhw > 0.0);
        assert_eq!(r.alert_level, 2);
    }

    #[test]
    fn test_ekman() {
        let r = ekman_upwelling(0.1, 30.0);
        assert!(r.ekman_transport_m2_s > 0.0);
    }

    #[test]
    fn test_geostrophic() {
        let r = geostrophic_current(1e-6, -2e-6, 30.0);
        assert!(r.speed_m_s >= 0.0);
    }

    #[test]
    fn test_slr() {
        let r = project_slr(3.5, 0.1, 2020);
        assert!(r.projected_2100_m > 0.0);
    }

    #[test]
    fn test_fisheries() {
        let r = fisheries_potential(0.8, 25.0, "pelagic");
        assert!(r.potential_index > 0.5);
    }

    #[test]
    fn test_acidification() {
        let r = aragonite_saturation(8.0, 25.0);
        assert!(r.aragonite_saturation > 0.0);
    }
}
