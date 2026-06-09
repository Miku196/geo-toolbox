use crate::config::AgriConfig;
pub struct AgriPlugin { config: AgriConfig }
impl AgriPlugin {
    pub fn new(config: AgriConfig) -> Self { Self { config } }
    /// 作物估产：面积(ha) × NDVI × 系数 × 基准产量。
    pub fn estimate_yield(&self, area_ha: f64, ndvi_mean: f64, baseline_yield_kg_ha: f64) -> f64 {
        let p = &self.config.yield_params;
        area_ha * ndvi_mean * p.ndvi_weight * baseline_yield_kg_ha * p.crop_coefficient
    }
    /// 土壤评级（简化：基于有机质含量和 pH）。
    pub fn soil_rating(&self, organic_matter_pct: f64, ph: f64) -> &'static str {
        let organic_ok = organic_matter_pct >= 2.0;
        let ph_ok = (5.5..=8.0).contains(&ph);
        match (organic_ok, ph_ok) {
            (true, true) => "优", (true, false) | (false, true) => "中", _ => "差",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_yield() {
        let config = toml::from_str("[plugin]\nname=\"agri\"\nversion=\"0.1\"\ndescription=\"\"\n").unwrap();
        let p = AgriPlugin::new(config);
        let y = p.estimate_yield(10.0, 0.7, 6000.0);
        assert!(y > 0.0);
    }
    #[test]
    fn test_soil() {
        let config = toml::from_str("[plugin]\nname=\"agri\"\nversion=\"0.1\"\ndescription=\"\"\n").unwrap();
        let p = AgriPlugin::new(config);
        assert_eq!(p.soil_rating(3.0, 6.5), "优");
        assert_eq!(p.soil_rating(0.5, 4.0), "差");
    }
}
