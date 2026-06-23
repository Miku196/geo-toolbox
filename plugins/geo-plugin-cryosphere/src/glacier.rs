//! 冰川物质平衡与运动
use serde::{Deserialize, Serialize};

/// 冰川物质平衡结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlacierBalance {
    pub accumulation_mwe: f64,
    pub ablation_mwe: f64,
    pub net_balance_mwe: f64,
    pub equilibrium_line_altitude_m: f64,
    pub area_km2: f64,
}

/// 冰川物质平衡: accumulation - ablation
pub fn mass_balance(accumulation_mwe: f64, ablation_mwe: f64, area_km2: f64) -> GlacierBalance {
    GlacierBalance {
        accumulation_mwe,
        ablation_mwe,
        net_balance_mwe: accumulation_mwe - ablation_mwe,
        equilibrium_line_altitude_m: if accumulation_mwe > 0.0 { 3000.0 + (ablation_mwe / accumulation_mwe) * 500.0 } else { 0.0 },
        area_km2,
    }
}

/// 冰川运动速度简化: v = k × τ^n (Glen's flow law)
pub fn glacier_flow_velocity(surface_slope_deg: f64, ice_thickness_m: f64, temp_c: f64) -> f64 {
    let tau = 917.0 * 9.81 * ice_thickness_m * surface_slope_deg.to_radians().sin();
    let n = 3.0;
    let a = (2.4e-24 * (temp_c + 10.0).exp() * 1e9).min(1e-15);
    let strain_rate = a * tau.powi(n as i32);
    let vel = strain_rate * ice_thickness_m * 31536000.0;
    vel.min(500.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mass_balance_negative() {
        let b = mass_balance(0.5, 1.0, 1.0);
        assert!(b.net_balance_mwe < 0.0);
    }

    #[test]
    fn test_mass_balance_positive() {
        let b = mass_balance(1.5, 0.5, 2.0);
        assert!(b.net_balance_mwe > 0.0);
        assert!((b.area_km2 - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_flow_velocity() {
        let v = glacier_flow_velocity(5.0, 100.0, -2.0);
        assert!(v > 0.0 && v < 800.0);
    }
}
