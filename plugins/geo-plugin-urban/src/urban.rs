use crate::config::UrbanConfig;
pub struct UrbanPlugin { config: UrbanConfig }
impl UrbanPlugin {
    pub fn new(config: UrbanConfig) -> Self { Self { config } }
    /// 容积率计算：总建筑面积 / 占地面积。
    pub fn far(&self, total_floor_area_m2: f64, site_area_m2: f64) -> f64 {
        if site_area_m2 > 0.0 { total_floor_area_m2 / site_area_m2 } else { 0.0 }
    }
    /// 建筑密度：建筑基底面积 / 占地面积。
    pub fn building_density(&self, building_footprint_m2: f64, site_area_m2: f64) -> f64 {
        if site_area_m2 > 0.0 { building_footprint_m2 / site_area_m2 } else { 0.0 }
    }
    /// 合规检查。
    pub fn check_compliance(&self, far: f64, density: f64) -> (bool, bool) {
        (far <= self.config.density.far_max, density <= 0.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_far() {
        let config = toml::from_str("[plugin]\nname=\"urban\"\nversion=\"0.1\"\ndescription=\"\"\n").unwrap();
        let p = UrbanPlugin::new(config);
        assert!((p.far(3500.0, 1000.0) - 3.5).abs() < 0.01);
    }
}
