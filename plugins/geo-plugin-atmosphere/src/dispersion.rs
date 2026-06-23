use serde::{Deserialize, Serialize};

use crate::boundary_layer::StabilityClass;

/// 高斯烟羽扩散模型。
#[derive(Debug, Clone)]
pub struct GaussianPlume {
    pub emission_rate_g_s: f64,
    pub wind_speed_m_s: f64,
    pub stability: StabilityClass,
    pub source_height_m: f64,
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

    /// Briggs 乡村扩散参数 σy, σz (m) 在距离 x (m) 处。
    fn sigma_yz(&self, x_m: f64) -> (f64, f64) {
        let x = (x_m / 1000.0).max(0.001);
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

        (sy * 1000.0, sz * 1000.0)
    }

    /// 下风向 (x,y,z) 处浓度 (μg/m³)。
    ///
    /// C = Q/(2π·u·σy·σz) · exp(-y²/2σy²) · [exp(-(z-H)²/2σz²) + exp(-(z+H)²/2σz²)]
    pub fn concentration(&self, x_m: f64, y_m: f64, z_m: f64) -> f64 {
        let u = self.wind_speed_m_s;
        let h = self.source_height_m;
        let z = z_m.max(self.receptor_height_m);

        if u <= 0.0 {
            return 0.0;
        }

        let q = self.emission_rate_g_s * 1_000_000.0; // g/s → μg/s
        let (sy, sz) = self.sigma_yz(x_m);

        if sy <= 0.0 || sz <= 0.0 {
            return 0.0;
        }

        let denom = 2.0 * std::f64::consts::PI * u * sy * sz;
        let exp_y = (-y_m.powi(2) / (2.0 * sy.powi(2))).exp();
        let exp_z1 = (-(z - h).powi(2) / (2.0 * sz.powi(2))).exp();
        let exp_z2 = (-(z + h).powi(2) / (2.0 * sz.powi(2))).exp();

        (q / denom) * exp_y * (exp_z1 + exp_z2)
    }

    /// 下风向轴线上最大地面浓度 (μg/m³) 及其距离 (m)。
    pub fn max_ground_concentration(&self) -> (f64, f64) {
        // 对中性/不稳定条件，最大浓度约在 σz ≈ H/√2 处
        let h = self.source_height_m;
        let u = self.wind_speed_m_s;

        if u <= 0.0 || h <= 0.0 {
            return (0.0, 0.0);
        }

        let q = self.emission_rate_g_s * 1_000_000.0;

        // 搜索 x 范围 10m - 50km
        let mut best_c = 0.0;
        let mut best_x = 0.0;
        for x in (10..50000).step_by(10) {
            let x_m = x as f64;
            let (sy, sz) = self.sigma_yz(x_m);
            if sy <= 0.0 || sz <= 0.0 {
                continue;
            }
            let denom = 2.0 * std::f64::consts::PI * u * sy * sz;
            let c = q * (-(h).powi(2) / (2.0 * sz.powi(2))).exp() / denom;
            if c > best_c {
                best_c = c;
                best_x = x_m;
            }
        }

        (best_c, best_x)
    }
}

/// 扩散结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispersionResult {
    pub plume: PlumeSummary,
    pub centerline: Vec<ConcentrationPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlumeSummary {
    pub stability: String,
    pub wind_speed_m_s: f64,
    pub source_height_m: f64,
    pub max_ground_conc_ug_m3: f64,
    pub max_conc_distance_m: f64,
    pub max_conc_classification: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcentrationPoint {
    pub distance_m: f64,
    pub concentration_ug_m3: f64,
}

/// 生成中心线浓度剖面。
pub fn centerline_profile(plume: &GaussianPlume, distances: &[f64]) -> Vec<ConcentrationPoint> {
    distances
        .iter()
        .map(|&d| {
            let c = plume.concentration(d, 0.0, 0.0);
            ConcentrationPoint {
                distance_m: d,
                concentration_ug_m3: c,
            }
        })
        .collect()
}

/// 完整扩散评估。
pub fn dispersion_assessment(
    emission_rate_g_s: f64,
    wind_speed_m_s: f64,
    stability: StabilityClass,
    source_height_m: f64,
) -> DispersionResult {
    let plume = GaussianPlume::new(
        emission_rate_g_s,
        wind_speed_m_s,
        stability,
        source_height_m,
    );

    let (max_c, max_x) = plume.max_ground_concentration();

    let distances: Vec<f64> = (1..=50).map(|i| i as f64 * 100.0).collect();
    let centerline = centerline_profile(&plume, &distances);

    let classification = if max_c < 10.0 {
        "negligible"
    } else if max_c < 50.0 {
        "low"
    } else if max_c < 150.0 {
        "moderate"
    } else if max_c < 500.0 {
        "high"
    } else {
        "severe"
    };

    DispersionResult {
        plume: PlumeSummary {
            stability: stability.as_str().to_string(),
            wind_speed_m_s,
            source_height_m,
            max_ground_conc_ug_m3: max_c,
            max_conc_distance_m: max_x,
            max_conc_classification: classification.to_string(),
        },
        centerline,
    }
}

// ═══════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn neutral_plume() -> GaussianPlume {
        GaussianPlume::new(10.0, 5.0, StabilityClass::D, 50.0)
    }

    #[test]
    fn test_sigma_yz_neutral() {
        let plume = neutral_plume();
        let (sy, sz) = plume.sigma_yz(500.0);
        assert!(sy > 0.0 && sy < 100.0, "sy={}", sy);
        assert!(sz > 0.0 && sz < 50.0, "sz={}", sz);
    }

    #[test]
    fn test_downwind_concentration() {
        let plume = neutral_plume();
        let c = plume.concentration(500.0, 0.0, 0.0);
        assert!(c > 0.0, "downwind conc should be >0, got {}", c);
    }

    #[test]
    fn test_concentration_at_centerline() {
        let plume = neutral_plume();
        let c_center = plume.concentration(500.0, 0.0, 0.0);
        let c_off = plume.concentration(500.0, 50.0, 0.0);
        assert!(c_center > c_off, "center > off-axis");
    }

    #[test]
    fn test_max_ground_concentration() {
        let plume = neutral_plume();
        let (c, x) = plume.max_ground_concentration();
        assert!(c > 0.0, "max conc>0, got {}", c);
        assert!(x > 0.0 && x < 50000.0, "max dist {}, expect 10-50000", x);
    }

    #[test]
    fn test_zero_wind_speed() {
        let plume = GaussianPlume::new(10.0, 0.0, StabilityClass::D, 50.0);
        let c = plume.concentration(500.0, 0.0, 0.0);
        assert_eq!(c, 0.0, "no wind -> 0 conc");
    }

    #[test]
    fn test_ground_level_source() {
        let plume = GaussianPlume::new(10.0, 3.0, StabilityClass::D, 0.0);
        let c = plume.concentration(100.0, 0.0, 0.0);
        assert!(c > 0.0, "ground source -> conc>0, got {}", c);
    }

    #[test]
    fn test_centerline_profile_length() {
        let plume = neutral_plume();
        let distances: Vec<f64> = vec![100.0, 500.0, 1000.0, 5000.0];
        let profile = centerline_profile(&plume, &distances);
        assert_eq!(profile.len(), 4);
        for pt in &profile {
            assert!(pt.concentration_ug_m3 >= 0.0);
        }
    }

    #[test]
    fn test_dispersion_assessment() {
        let result = dispersion_assessment(10.0, 5.0, StabilityClass::D, 50.0);
        assert!(result.plume.max_ground_conc_ug_m3 > 0.0);
        assert!(result.plume.max_conc_distance_m > 0.0);
        assert_eq!(result.centerline.len(), 50);
    }

    #[test]
    fn test_all_stability_classes() {
        for &stab in &[
            StabilityClass::A,
            StabilityClass::B,
            StabilityClass::C,
            StabilityClass::D,
            StabilityClass::E,
            StabilityClass::F,
        ] {
            let plume = GaussianPlume::new(10.0, 3.0, stab, 30.0);
            let (c, x) = plume.max_ground_concentration();
            assert!(c > 0.0, "{:?} max conc={}", stab, c);
            assert!(x > 0.0, "{:?} max dist={}", stab, x);
        }
    }
}
