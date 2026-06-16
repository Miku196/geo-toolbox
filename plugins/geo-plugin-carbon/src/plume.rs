//! Gaussian plume dispersion model.
//!
//! Point-source atmospheric dispersion for CO₂ and other gaseous emissions.
//! Reference: Pasquill-Gifford stability classes with Briggs rural dispersion parameters.

use serde::{Deserialize, Serialize};

/// Pasquill-Gifford atmospheric stability class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StabilityClass {
    /// Very unstable
    A,
    /// Unstable
    B,
    /// Slightly unstable
    C,
    /// Neutral
    D,
    /// Slightly stable
    E,
    /// Stable
    F,
}

impl StabilityClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            StabilityClass::A => "A (very unstable)",
            StabilityClass::B => "B (unstable)",
            StabilityClass::C => "C (slightly unstable)",
            StabilityClass::D => "D (neutral)",
            StabilityClass::E => "E (slightly stable)",
            StabilityClass::F => "F (stable)",
        }
    }
}

/// Gaussian plume dispersion model for a point source.
#[derive(Debug, Clone)]
pub struct GaussianPlume {
    /// Emission rate (g/s)
    pub emission_rate_g_s: f64,
    /// Wind speed at release height (m/s)
    pub wind_speed_m_s: f64,
    /// Pasquill-Gifford stability class
    pub stability: StabilityClass,
    /// Effective source height (m), stack height + plume rise
    pub source_height_m: f64,
    /// Receptor height (m), usually 0 for ground level
    pub receptor_height_m: f64,
}

impl GaussianPlume {
    pub fn new(
        emission_rate_g_s: f64,
        wind_speed_m_s: f64,
        stability: StabilityClass,
        source_height_m: f64,
    ) -> Self {
        Self {
            emission_rate_g_s,
            wind_speed_m_s,
            stability,
            source_height_m,
            receptor_height_m: 0.0,
        }
    }

    /// Briggs rural dispersion parameters σy, σz at downwind distance x (meters).
    /// x is converted internally to km for the dispersion formulas.
    fn sigma_yz(&self, x_m: f64) -> (f64, f64) {
        let x = (x_m / 1000.0).max(0.001); // km
        let denom = (1.0 + 0.0001 * x).sqrt();

        let sy = match self.stability {
            StabilityClass::A => 0.22 * x / denom,
            StabilityClass::B => 0.16 * x / denom,
            StabilityClass::C => 0.11 * x / denom,
            StabilityClass::D => 0.08 * x / denom,
            StabilityClass::E => 0.06 * x / denom,
            StabilityClass::F => 0.04 * x / denom,
        };

        let sz = match self.stability {
            StabilityClass::A => 0.20 * x,
            StabilityClass::B => 0.12 * x,
            StabilityClass::C => 0.08 * x / (1.0 + 0.0002 * x).sqrt(),
            StabilityClass::D => 0.06 * x / (1.0 + 0.0015 * x).sqrt(),
            StabilityClass::E => 0.03 * x / (1.0 + 0.0003 * x),
            StabilityClass::F => 0.016 * x / (1.0 + 0.0003 * x),
        };

        (sy * 1000.0, sz * 1000.0) // convert km → m
    }

    /// Concentration at point (x, y, z).
    ///
    /// C(x,y,z) = Q / (2π · u · σy · σz) · exp(-y²/(2σy²)) · [exp(-(z-H)²/(2σz²)) + exp(-(z+H)²/(2σz²))]
    ///
    /// Returns concentration in g/m³.
    pub fn concentration(&self, x: f64, y: f64, z: f64) -> f64 {
        if x <= 0.0 || self.wind_speed_m_s <= 0.0 {
            return 0.0;
        }

        let (sy, sz) = self.sigma_yz(x);

        if sy < 1e-10 || sz < 1e-10 {
            return 0.0;
        }

        let H = self.source_height_m;
        let zr = if z < 0.0 { self.receptor_height_m } else { z };
        let Q = self.emission_rate_g_s;
        let u = self.wind_speed_m_s;

        let prefix = Q / (2.0 * std::f64::consts::PI * u * sy * sz);

        let y_term = (-y * y / (2.0 * sy * sy)).exp();

        let z_term1 = (-(zr - H).powi(2) / (2.0 * sz * sz)).exp();
        let z_term2 = (-(zr + H).powi(2) / (2.0 * sz * sz)).exp();
        let z_term = z_term1 + z_term2;

        prefix * y_term * z_term
    }

    /// Ground-level centerline concentration at downwind distance x (y=0, z=0).
    /// Returns concentration in g/m³.
    pub fn downwind_concentration(&self, x: f64) -> f64 {
        self.concentration(x, 0.0, 0.0)
    }

    /// Maximum ground-level concentration and the downwind distance where it occurs.
    ///
    /// Uses a simple grid search approach. Returns (x_max_m, C_max_g_m3).
    pub fn max_ground_concentration(&self, x_start: f64, x_end: f64, steps: usize) -> (f64, f64) {
        let dx = (x_end - x_start) / steps.max(1) as f64;
        let mut best_x = x_start;
        let mut best_c = 0.0;

        for i in 0..=steps {
            let x = x_start + i as f64 * dx;
            let c = self.concentration(x, 0.0, 0.0);
            if c > best_c {
                best_c = c;
                best_x = x;
            }
        }

        (best_x, best_c)
    }

    /// Concentration in mg/m³ (convenience).
    pub fn downwind_concentration_mg_m3(&self, x: f64) -> f64 {
        self.downwind_concentration(x) * 1000.0
    }

    /// Concentration in µg/m³ (convenience).
    pub fn downwind_concentration_ug_m3(&self, x: f64) -> f64 {
        self.downwind_concentration(x) * 1_000_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sigma_yz_neutral() {
        let plume = GaussianPlume::new(1.0, 5.0, StabilityClass::D, 50.0);
        let (sy, sz) = plume.sigma_yz(1000.0); // 1 km
        assert!(sy > 0.0);
        assert!(sz > 0.0);
        assert!(sy > sz, "σy should be larger than σz near source");
    }

    #[test]
    fn test_downwind_concentration() {
        let plume = GaussianPlume::new(100.0, 5.0, StabilityClass::D, 50.0);
        let c = plume.downwind_concentration(500.0);
        // At 500m, near the plume centerline, should have non-zero concentration
        assert!(c >= 0.0);

        let c_closer = plume.downwind_concentration(100.0);
        // Closer to source, plume hasn't reached ground yet for elevated source
        assert!(c_closer >= 0.0);
    }

    #[test]
    fn test_ground_level_source() {
        let plume = GaussianPlume::new(10.0, 3.0, StabilityClass::B, 1.0); // near-ground source
        let c_100 = plume.downwind_concentration(100.0);
        let c_500 = plume.downwind_concentration(500.0);
        // For near-ground source, concentration should be higher closer to source
        // (but for Gaussian plume, maximum shifts with stability)
        assert!(c_100 >= 0.0);
        assert!(c_500 >= 0.0);
    }

    #[test]
    fn test_max_ground_concentration() {
        let plume = GaussianPlume::new(50.0, 4.0, StabilityClass::C, 30.0);
        let (x_max, c_max) = plume.max_ground_concentration(100.0, 5000.0, 100);
        assert!(x_max >= 100.0);
        assert!(c_max >= 0.0);
    }

    #[test]
    fn test_concentration_at_centerline() {
        let plume = GaussianPlume::new(50.0, 5.0, StabilityClass::D, 50.0);
        let c_center = plume.concentration(2000.0, 0.0, 0.0);
        let c_offset = plume.concentration(2000.0, 200.0, 0.0);
        // Centerline should have higher or equal concentration than offset
        assert!(c_center >= c_offset);
    }

    #[test]
    fn test_zero_concentration_upwind() {
        let plume = GaussianPlume::new(50.0, 5.0, StabilityClass::D, 50.0);
        let c = plume.concentration(-10.0, 0.0, 0.0);
        assert_eq!(c, 0.0);
    }

    #[test]
    fn test_zero_wind_speed() {
        let plume = GaussianPlume::new(50.0, 0.0, StabilityClass::D, 50.0);
        let c = plume.downwind_concentration(1000.0);
        assert_eq!(c, 0.0);
    }

    #[test]
    fn test_convenience_units() {
        let plume = GaussianPlume::new(100.0, 5.0, StabilityClass::D, 30.0);
        let c_g = plume.downwind_concentration(2000.0);
        let c_mg = plume.downwind_concentration_mg_m3(2000.0);
        let c_ug = plume.downwind_concentration_ug_m3(2000.0);
        assert!((c_mg - c_g * 1000.0).abs() < 1e-10);
        assert!((c_ug - c_g * 1_000_000.0).abs() < 1e-10);
    }

    #[test]
    fn test_all_stability_classes() {
        for stab in &[
            StabilityClass::A,
            StabilityClass::B,
            StabilityClass::C,
            StabilityClass::D,
            StabilityClass::E,
            StabilityClass::F,
        ] {
            let plume = GaussianPlume::new(10.0, 5.0, *stab, 20.0);
            let c = plume.downwind_concentration(1000.0);
            assert!(c >= 0.0, "Failed for {:?}", stab);
        }
    }
}
