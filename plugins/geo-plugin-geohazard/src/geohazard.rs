use crate::config::GeohazardConfig;
pub struct GeohazardPlugin { config: GeohazardConfig }
impl GeohazardPlugin {
    pub fn new(config: GeohazardConfig) -> Self { Self { config } }
    /// 滑坡敏感性指数：slope*0.4 + lithology*0.35 + rainfall*0.25。
    /// 各因子归一化到 [0,1]。
    pub fn landslide_susceptibility(&self, slope_norm: f64, lithology_norm: f64, rainfall_norm: f64) -> f64 {
        let p = &self.config.landslide;
        (slope_norm * p.slope_weight + lithology_norm * p.lithology_weight + rainfall_norm * p.rainfall_weight)
            .clamp(0.0, 1.0)
    }
    /// 风险等级。
    pub fn risk_level(&self, susceptibility: f64) -> &'static str {
        if susceptibility >= 0.7 { "高" } else if susceptibility >= 0.4 { "中" } else { "低" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_susceptibility() {
        let config = toml::from_str("[plugin]\nname=\"geohazard\"\nversion=\"0.1\"\ndescription=\"\"\n").unwrap();
        let p = GeohazardPlugin::new(config);
        let s = p.landslide_susceptibility(0.8, 0.5, 0.9);
        assert!(s > 0.0 && s <= 1.0);
        assert_eq!(p.risk_level(s), if s >= 0.7 { "高" } else { "" });
    }
}
