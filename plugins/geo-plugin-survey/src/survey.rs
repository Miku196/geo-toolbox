//! 测绘核心逻辑。

use crate::config::SurveyConfig;

pub struct SurveyPlugin {
    #[allow(dead_code)]
    config: SurveyConfig,
}

impl SurveyPlugin {
    pub fn new(config: SurveyConfig) -> Self { Self { config } }

    /// 土方量计算（简化为多边形面积 × 平均高程差）。
    pub fn calculate_earthwork(
        &self,
        polygons: &[(f64, f64)], // (area_ha, avg_height_diff_m)
    ) -> f64 {
        polygons.iter()
            .map(|(area, diff)| area * 10000.0 * diff) // ha→m² × height→m³
            .sum::<f64>()
            .abs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_earthwork() {
        let config = toml::from_str("[plugin]\nname=\"survey\"\nversion=\"0.1\"\ndescription=\"\"\n").unwrap();
        let plugin = SurveyPlugin::new(config);
        let volume = plugin.calculate_earthwork(&[(1.0, 2.0), (0.5, -1.0)]);
        assert!(volume > 0.0);
    }
}
