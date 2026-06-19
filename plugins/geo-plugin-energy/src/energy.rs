//! 新能源选址评估核心逻辑。
//!
//! ## 光伏选址
//!
//! 适宜性 = 坡度因子 × 辐射因子 × 坡向因子
//! - 坡度 < 25° → 因子 = 1 - slope/25
//! - 年辐射 > 1500 kWh/m² → 因子 = radiation/2000
//! - 南向 (135°-225°) → 权重 1.2
//!
//! ## 风电选址
//!
//! - 风速 > 5.5 m/s
//! - 坡度 < 15°
//! - 粗糙度低（草地/裸地）

use crate::config::EnergyConfig;
use geo_core::errors::GeoResult;
use geo_core::types::BBox;
use geo_raster::RasterBand;
use serde::Serialize;

/// 光伏选址评估结果。
#[derive(Debug, Clone, Serialize)]
pub struct SolarAssessment {
    pub aoi_name: String,
    pub aoi_bbox: BBox,
    pub suitable_area_ha: f64,
    pub suitable_ratio: f64,
    pub mean_suitability: f64,
    pub grade: String,
    pub summary: String,
}

/// 风电选址评估结果。
#[derive(Debug, Clone, Serialize)]
pub struct WindAssessment {
    pub aoi_name: String,
    pub aoi_bbox: BBox,
    pub suitable_area_ha: f64,
    pub suitable_ratio: f64,
    pub mean_windspeed: f64,
    pub grade: String,
    pub summary: String,
}

/// 新能源选址插件。
pub struct EnergyPlugin {
    config: EnergyConfig,
}

impl EnergyPlugin {
    pub fn new(config: EnergyConfig) -> Self {
        Self { config }
    }

    pub fn from_file(path: &std::path::Path) -> GeoResult<Self> {
        let s = std::fs::read_to_string(path)?;
        let config: EnergyConfig =
            toml::from_str(&s).map_err(|e| geo_core::GeoError::Validation(e.to_string()))?;
        Ok(Self { config })
    }

    /// 光伏选址评估。
    ///
    /// - `dem`: DEM 栅格（用于计算坡度，假设值为高程m）
    /// - `radiation`: 年太阳辐射栅格 (kWh/m²)
    /// - `aoi_geojson`: AOI FeatureCollection
    pub fn assess_solar(
        &self,
        aoi_name: &str,
        aoi_geojson: &str,
        dem: &RasterBand,
        radiation: &RasterBand,
    ) -> GeoResult<SolarAssessment> {
        let bbox = geo_io::extract_bbox(aoi_geojson)?;
        let cfg = &self.config.solar;

        let mut suitable = 0usize;
        let mut total = 0usize;
        let mut sum_suitability = 0.0;

        for i in 0..dem.data.len().min(radiation.data.len()) {
            let elev = dem.data[i];
            let rad = radiation.data[i];
            if elev == dem.nodata || rad == radiation.nodata {
                continue;
            }
            total += 1;

            // 简化坡度估计：相邻像素高程差/分辨率（假设10m分辨率）
            let slope = if i > 0 {
                ((elev - dem.data[i - 1]).abs() / 10.0).atan().to_degrees()
            } else {
                0.0
            };

            let slope_factor = (1.0 - slope / cfg.slope_max_deg).max(0.0);
            let rad_factor = (rad / 2000.0).min(1.0);
            let suitability = slope_factor * 0.5 + rad_factor * 0.5;

            if slope <= cfg.slope_max_deg && rad >= cfg.radiation_min_kwh {
                suitable += 1;
            }
            sum_suitability += suitability;
        }

        let suitable_ratio = if total > 0 {
            suitable as f64 / total as f64
        } else {
            0.0
        };
        let mean_suitability = if total > 0 {
            sum_suitability / total as f64
        } else {
            0.0
        };

        let (grade, summary) = if suitable_ratio >= 0.6 {
            (
                "🏆 优秀",
                format!(
                    "{aoi_name} 光伏适宜性优秀：{:.0}% 区域达标",
                    suitable_ratio * 100.0
                ),
            )
        } else if suitable_ratio >= 0.3 {
            (
                "✅ 良好",
                format!(
                    "{aoi_name} 光伏适宜性良好：{:.0}% 区域达标",
                    suitable_ratio * 100.0
                ),
            )
        } else {
            ("⚠ 一般", format!("{aoi_name} 光伏适宜性一般，建议另选场址"))
        };

        Ok(SolarAssessment {
            aoi_name: aoi_name.to_string(),
            aoi_bbox: bbox,
            suitable_area_ha: suitable as f64 * 0.01, // 10m像素→ha
            suitable_ratio,
            mean_suitability,
            grade: grade.to_string(),
            summary: summary.to_string(),
        })
    }

    /// 风电选址评估（简化版）。
    pub fn assess_wind(
        &self,
        aoi_name: &str,
        aoi_geojson: &str,
        dem: &RasterBand,
        wind_speed: &RasterBand,
    ) -> GeoResult<WindAssessment> {
        let bbox = geo_io::extract_bbox(aoi_geojson)?;
        let cfg = &self.config.wind;

        let mut suitable = 0usize;
        let mut total = 0usize;
        let mut wind_sum = 0.0;

        for i in 0..dem.data.len().min(wind_speed.data.len()) {
            let elev = dem.data[i];
            let wind = wind_speed.data[i];
            if elev == dem.nodata || wind == wind_speed.nodata {
                continue;
            }
            total += 1;
            wind_sum += wind;

            let slope = if i > 0 {
                ((elev - dem.data[i - 1]).abs() / 10.0).atan().to_degrees()
            } else {
                0.0
            };

            if wind >= cfg.wind_speed_min_ms && slope <= cfg.slope_max_deg {
                suitable += 1;
            }
        }

        let suitable_ratio = if total > 0 {
            suitable as f64 / total as f64
        } else {
            0.0
        };
        let mean_ws = if total > 0 {
            wind_sum / total as f64
        } else {
            0.0
        };

        let (grade, summary) = if suitable_ratio >= 0.4 && mean_ws >= 6.0 {
            (
                "🏆 优秀",
                format!("{aoi_name} 风电适宜性优秀，均风速 {:.1} m/s", mean_ws),
            )
        } else if suitable_ratio >= 0.2 {
            ("✅ 良好", format!("{aoi_name} 风电适宜性良好"))
        } else {
            ("⚠ 一般", format!("{aoi_name} 风速偏低，建议另选场址"))
        };

        Ok(WindAssessment {
            aoi_name: aoi_name.to_string(),
            aoi_bbox: bbox,
            suitable_area_ha: suitable as f64 * 0.01,
            suitable_ratio,
            mean_windspeed: mean_ws,
            grade: grade.to_string(),
            summary: summary.to_string(),
        })
    }

    /// Weibull 风速分布拟合。
    ///
    /// 风能密度 WPD = 0.5 * ρ * c³ * Γ(1 + 3/k)，ρ=1.225 kg/m³。
    /// 返回 (k, c, mean_ws, WPD_W_m2)。
    pub fn weibull_fit(wind_speeds: &[f64]) -> (f64, f64, f64, f64) {
        let n = wind_speeds.len();
        if n < 3 {
            return (2.0, 5.0, 5.0, 0.0);
        }

        let mean = wind_speeds.iter().sum::<f64>() / n as f64;
        let variance = wind_speeds.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / n as f64;
        let stddev = variance.sqrt();

        // Method of Moments: k ≈ (σ / v_mean)^(-1.086), c = v_mean / Γ(1 + 1/k)
        let k = (stddev / mean.max(0.1)).powf(-1.086).clamp(1.0, 10.0);
        // Gamma function approximation: Γ(1 + 1/k)
        let g1 = gamma_approx(1.0 + 1.0 / k);
        let c = mean / g1.max(0.001);

        // Wind power density: WPD = 0.5 * ρ * c³ * Γ(1 + 3/k)
        let rho = 1.225;
        let g3 = gamma_approx(1.0 + 3.0 / k);
        let wpd = 0.5 * rho * c.powi(3) * g3;

        (k, c, mean, wpd)
    }
}

/// Stirling-based Gamma function approximation.
fn gamma_approx(x: f64) -> f64 {
    if x <= 0.0 {
        return 1.0;
    }
    // Stirling: Γ(x) ≈ sqrt(2π/x) * (x/e)^x
    let n = x;
    #[allow(unused_assignments)]
    let mut g = 1.0;
    let _t = n;
    // Use the recurrence Γ(x+1) = x * Γ(x) with Lanczos-like log
    if n < 1.0 {
        g = std::f64::consts::PI / ((std::f64::consts::PI * n).sin() * gamma_approx(1.0 - n));
        return g;
    }
    // Simple Stirling
    if n > 10.0 {
        g = (2.0 * std::f64::consts::PI / n).sqrt() * (n / std::f64::consts::E).powf(n);
    } else {
        // Compute via recurrence + Lanczos for [1, 10]
        let p = [
            676.5203681218851,
            -1259.1392167224028,
            771.3234287776531,
            -176.6150291621406,
            12.507343278686905,
            -0.13857109526572012,
            9.984369578019572e-6,
            1.5056327351493116e-7,
        ];
        let z = n - 1.0;
        let mut x_l = 0.999_999_999_999_809_9;
        for (i, &pi) in p.iter().enumerate() {
            x_l += pi / (z + i as f64 + 1.0);
        }
        let t = z + p.len() as f64 - 0.5;
        g = (2.0 * std::f64::consts::PI).sqrt() * t.powf(z + 0.5) * (-t).exp() * x_l;
    }
    g.max(1e-10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weibull_fit() {
        let winds = vec![3.0, 4.0, 5.0, 6.0, 7.0, 5.5, 4.5, 3.5, 8.0, 6.5];
        let (k, c, mean, wpd) = EnergyPlugin::weibull_fit(&winds);
        assert!(k >= 1.0 && k <= 10.0, "k={k} out of range");
        assert!(c > 0.0);
        assert!((mean - 5.3).abs() < 1.0, "mean={mean}");
        assert!(wpd > 0.0, "Wind power density should be positive");
    }

    #[test]
    fn test_gamma_approx() {
        // Γ(5) = 24
        let g = gamma_approx(5.0);
        assert!((g - 24.0).abs() < 1.0, "Γ(5) ≈ 24, got {g}");
        // Γ(1) = 1
        assert!((gamma_approx(1.0) - 1.0).abs() < 0.1);
    }
    use geo_raster::RasterBand;

    fn make_band(data: Vec<f64>) -> RasterBand {
        let w = data.len();
        RasterBand::new("test", w, 1, data, -999.0)
    }

    #[test]
    fn test_solar_assessment() {
        let plugin = EnergyPlugin::new(EnergyConfig::default());
        // 平坦地形 (低坡度) + 高辐射
        let dem = make_band(vec![100.0, 100.5, 100.3, 100.1]);
        let rad = make_band(vec![1800.0, 1750.0, 1600.0, 1900.0]);
        let aoi = r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{},"geometry":{"type":"Polygon","coordinates":[[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}}]}"#;

        let result = plugin.assess_solar("测试场", aoi, &dem, &rad).unwrap();
        assert!(result.suitable_ratio > 0.5);
        assert!(result.mean_suitability > 0.0);
    }

    #[test]
    fn test_wind_assessment() {
        let plugin = EnergyPlugin::new(EnergyConfig::default());
        let dem = make_band(vec![100.0, 100.5, 100.3, 100.1]);
        let wind = make_band(vec![6.0, 5.8, 4.0, 7.0]);
        let aoi = r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{},"geometry":{"type":"Polygon","coordinates":[[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}}]}"#;

        let result = plugin.assess_wind("风场", aoi, &dem, &wind).unwrap();
        assert!(result.mean_windspeed > 5.0);
        assert!(!result.grade.is_empty());
    }
}
