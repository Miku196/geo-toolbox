use crate::config::HydroConfig;
pub struct HydroPlugin { config: HydroConfig }
impl HydroPlugin {
    pub fn new(config: HydroConfig) -> Self { Self { config } }
    /// 简化的淹没面积估算：汇水面积 × 降雨量 / 安全系数。
    pub fn estimate_inundation_area(&self, catchment_area_ha: f64, rainfall_mm: f64) -> f64 {
        let runoff_volume = catchment_area_ha * 10000.0 * rainfall_mm / 1000.0;
        runoff_volume * self.config.flood.safety_factor / 1000.0
    }
    /// 径流系数（简化）。
    pub fn runoff_coefficient(&self, impervious_ratio: f64) -> f64 {
        0.05 + 0.9 * impervious_ratio.min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_inundation() {
        let config = toml::from_str("[plugin]\nname=\"hydro\"\nversion=\"0.1\"\ndescription=\"\"\n").unwrap();
        let p = HydroPlugin::new(config);
        let area = p.estimate_inundation_area(100.0, 50.0);
        assert!(area > 0.0);
    }
}
