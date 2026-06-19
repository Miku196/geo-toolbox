use crate::config::UrbanConfig;
use serde::{Deserialize, Serialize};

/// NLCD 用地分类。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LandUseClass {
    Water,
    GreenSpace,
    BareSoil,
    LowIntensityUrban,
    MediumIntensityUrban,
    HighIntensityUrban,
}

/// 日照分析结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolarResult {
    /// 冬至日阴影长度(m)。
    pub winter_shadow_m: f64,
    /// 夏至日阴影长度(m)。
    pub summer_shadow_m: f64,
    /// 冬至日阴影方位角(度)。
    pub shadow_azimuth_deg: f64,
    /// 日照合规（冬至日阴影不遮挡相邻建筑）。
    pub compliant: bool,
}

/// UHI 评估。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UhiResult {
    pub uhi_index: f64,
    pub risk_level: String,
}

/// 通风走廊分析。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VentilationResult {
    /// 粗糙度长度(m)。
    pub roughness_length_m: f64,
    /// 通风指数[0,1]。
    pub ventilation_index: f64,
    pub quality: String,
}

/// 综合城乡规划评估。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrbanAssessment {
    pub far: f64,
    pub building_density: f64,
    pub estimated_avg_height_m: f64,
    pub far_compliant: bool,
    pub density_compliant: bool,
    pub green_ratio: f64,
    pub green_per_capita_m2: f64,
    pub land_use: Vec<(String, f64)>,
    pub uhi: UhiResult,
    pub solar: SolarResult,
    pub ventilation: VentilationResult,
}

// ════════════════════════════════════════════════════════════════

pub struct UrbanPlugin {
    config: UrbanConfig,
}

impl UrbanPlugin {
    pub fn new(config: UrbanConfig) -> Self {
        Self { config }
    }
    pub fn config(&self) -> &UrbanConfig {
        &self.config
    }

    // ── 容积率 ──
    pub fn far(&self, total_floor_area_m2: f64, site_area_m2: f64) -> f64 {
        if site_area_m2 > 0.0 {
            total_floor_area_m2 / site_area_m2
        } else {
            0.0
        }
    }

    // ── 建筑密度 ──
    pub fn building_density(&self, footprint_m2: f64, site_area_m2: f64) -> f64 {
        if site_area_m2 > 0.0 {
            footprint_m2 / site_area_m2
        } else {
            0.0
        }
    }

    // ── 平均建筑高度推算 ──
    pub fn estimate_avg_height(&self, far: f64, coverage_ratio: f64) -> f64 {
        if coverage_ratio > 0.0 {
            let floors = far / coverage_ratio;
            (floors * self.config.density.height_per_floor_m).max(0.0)
        } else {
            0.0
        }
    }

    // ── 合规检查 ──
    pub fn check_compliance(&self, far: f64, density: f64) -> (bool, bool) {
        (
            far <= self.config.density.far_max,
            density <= self.config.density.density_max,
        )
    }

    // ── NLCD 用地分类 ──
    pub fn classify_land_use(
        &self,
        ndvi: Option<f64>,
        impervious_pct: Option<f64>,
    ) -> LandUseClass {
        let lu = &self.config.land_use;
        match (ndvi, impervious_pct) {
            (Some(n), _) if n <= lu.water_ndvi_max => LandUseClass::Water,
            (_, Some(imp)) if imp <= lu.green_impervious_max => LandUseClass::GreenSpace,
            (_, Some(imp)) if imp <= lu.low_impervious_max => LandUseClass::LowIntensityUrban,
            (_, Some(imp)) if imp <= lu.medium_impervious_max => LandUseClass::MediumIntensityUrban,
            (_, Some(_)) => LandUseClass::HighIntensityUrban,
            // fallback: from NDVI alone
            (Some(n), None) if n > 0.3 => LandUseClass::GreenSpace,
            (Some(n), None) if n > 0.0 => LandUseClass::BareSoil,
            (None, None) => LandUseClass::BareSoil,
            _ => LandUseClass::BareSoil,
        }
    }

    /// 面积占比统计。
    pub fn land_use_stats(
        &self,
        ndvi_values: &[Option<f64>],
        impervious_values: &[Option<f64>],
        total_area_ha: f64,
    ) -> Vec<(String, f64)> {
        let n = ndvi_values.len().max(impervious_values.len());
        if n == 0 || total_area_ha <= 0.0 {
            return vec![];
        }
        let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for i in 0..n {
            let ndvi = ndvi_values.get(i).copied().flatten();
            let imp = impervious_values.get(i).copied().flatten();
            let cls = self.classify_land_use(ndvi, imp);
            let label = match cls {
                LandUseClass::Water => "水体",
                LandUseClass::GreenSpace => "绿地",
                LandUseClass::BareSoil => "裸土",
                LandUseClass::LowIntensityUrban => "低强度城市",
                LandUseClass::MediumIntensityUrban => "中强度城市",
                LandUseClass::HighIntensityUrban => "高强度城市",
            };
            *counts.entry(label).or_insert(0) += 1;
        }
        let total = counts.values().sum::<usize>() as f64;
        let mut result: Vec<_> = counts
            .into_iter()
            .map(|(label, count)| (label.to_string(), count as f64 / total * total_area_ha))
            .collect();
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        result
    }

    // ── 日照分析 ──
    pub fn solar_analysis(&self, building_height_m: f64, neighbor_distance_m: f64) -> SolarResult {
        let s = &self.config.solar;
        let winter_rad = s.winter_sun_altitude_deg.to_radians();
        let summer_rad = s.summer_sun_altitude_deg.to_radians();
        let winter_shadow = if winter_rad.sin() > 0.001 {
            building_height_m / winter_rad.tan()
        } else {
            f64::MAX
        };
        let summer_shadow = if summer_rad.sin() > 0.001 {
            building_height_m / summer_rad.tan()
        } else {
            f64::MAX
        };
        let compliant = winter_shadow <= neighbor_distance_m || neighbor_distance_m <= 0.0;
        SolarResult {
            winter_shadow_m: winter_shadow,
            summer_shadow_m: summer_shadow,
            shadow_azimuth_deg: s.winter_sun_azimuth_deg,
            compliant,
        }
    }

    // ── 热岛效应 ──
    pub fn uhi_index(
        &self,
        impervious_ratio: f64,
        building_density: f64,
        green_ratio: f64,
    ) -> UhiResult {
        let u = &self.config.uhi;
        let idx = u.impervious_weight * impervious_ratio
            + u.density_weight * building_density
            + u.green_weight * (1.0 - green_ratio);
        let level = if idx >= u.high_threshold {
            "高"
        } else if idx >= u.medium_threshold {
            "中"
        } else {
            "低"
        };
        UhiResult {
            uhi_index: idx.clamp(0.0, 1.0),
            risk_level: level.to_string(),
        }
    }

    // ── 绿地率 ──
    pub fn green_ratio(&self, green_area_m2: f64, total_area_m2: f64) -> f64 {
        if total_area_m2 > 0.0 {
            green_area_m2 / total_area_m2
        } else {
            0.0
        }
    }

    pub fn green_per_capita(&self, green_area_m2: f64, population: u64) -> f64 {
        if population > 0 {
            green_area_m2 / population as f64
        } else {
            0.0
        }
    }

    // ── 通风走廊分析 ──
    pub fn ventilation(
        &self,
        avg_building_height_m: f64,
        building_density: f64,
    ) -> VentilationResult {
        // 粗糙度长度 z0 ≈ 0.1 × h × λp (MacDonald 1998)
        let rough_len = 0.1 * avg_building_height_m * building_density.min(0.6);
        let max_rough = avg_building_height_m * 0.06; // worst case: h * 0.6 * 0.1
        let vi = if max_rough > 0.0 {
            (1.0 - rough_len / max_rough).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let quality = if vi >= 0.7 {
            "良好"
        } else if vi >= 0.4 {
            "一般"
        } else {
            "较差"
        };
        VentilationResult {
            roughness_length_m: rough_len,
            ventilation_index: vi,
            quality: quality.to_string(),
        }
    }

    // ── 综合评估 ──
    #[allow(clippy::too_many_arguments)]
    pub fn assess(
        &self,
        total_floor_area_m2: f64,
        building_footprint_m2: f64,
        site_area_m2: f64,
        green_area_m2: f64,
        population: u64,
        impervious_ratio: f64,
        ndvi_values: &[Option<f64>],
        impervious_values: &[Option<f64>],
    ) -> UrbanAssessment {
        let far = self.far(total_floor_area_m2, site_area_m2);
        let density = self.building_density(building_footprint_m2, site_area_m2);
        let avg_h = self.estimate_avg_height(far, density);
        let (fc, dc) = self.check_compliance(far, density);
        let gr = self.green_ratio(green_area_m2, site_area_m2);
        let gpc = self.green_per_capita(green_area_m2, population);
        let land_use = self.land_use_stats(ndvi_values, impervious_values, site_area_m2 / 10000.0);
        let uhi = self.uhi_index(impervious_ratio, density, gr);
        let solar = self.solar_analysis(avg_h, 30.0);
        let vent = self.ventilation(avg_h, density);
        UrbanAssessment {
            far,
            building_density: density,
            estimated_avg_height_m: avg_h,
            far_compliant: fc,
            density_compliant: dc,
            green_ratio: gr,
            green_per_capita_m2: gpc,
            land_use,
            uhi,
            solar,
            ventilation: vent,
        }
    }
}

// ═══ 测试 ═══
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;

    fn default_plugin() -> UrbanPlugin {
        UrbanPlugin::new(UrbanConfig {
            plugin: PluginMeta {
                name: "urban".into(),
                version: "0.1".into(),
                description: "test".into(),
            },
            density: DensityParams::default(),
            land_use: LandUseParams::default(),
            solar: SolarParams::default(),
            uhi: UhiParams::default(),
            vegetation: VegetationParams::default(),
        })
    }

    #[test]
    fn test_far() {
        let p = default_plugin();
        assert!((p.far(3500.0, 1000.0) - 3.5).abs() < 0.01);
    }

    #[test]
    fn test_building_density() {
        let p = default_plugin();
        assert!((p.building_density(300.0, 1000.0) - 0.3).abs() < 0.01);
    }

    #[test]
    fn test_estimate_height() {
        let p = default_plugin();
        // FAR=3.5, coverage=0.3 → floors ≈ 11.67 → height ≈ 35m
        let h = p.estimate_avg_height(3.5, 0.3);
        assert!((h - 35.0).abs() < 0.5);
    }

    #[test]
    fn test_check_compliance() {
        let p = default_plugin();
        assert_eq!(p.check_compliance(3.0, 0.3), (true, true));
        assert_eq!(p.check_compliance(5.0, 0.5), (false, false));
    }

    #[test]
    fn test_classify_land_use() {
        let p = default_plugin();
        assert_eq!(
            p.classify_land_use(Some(-0.2), Some(0.0)),
            LandUseClass::Water
        );
        assert_eq!(
            p.classify_land_use(Some(0.5), Some(0.1)),
            LandUseClass::GreenSpace
        );
        assert_eq!(
            p.classify_land_use(Some(0.2), Some(0.6)),
            LandUseClass::MediumIntensityUrban
        );
        assert_eq!(
            p.classify_land_use(Some(0.2), Some(0.9)),
            LandUseClass::HighIntensityUrban
        );
        assert_eq!(p.classify_land_use(Some(0.0), None), LandUseClass::BareSoil);
        assert_eq!(p.classify_land_use(None, None), LandUseClass::BareSoil);
    }

    #[test]
    fn test_land_use_stats() {
        let p = default_plugin();
        let ndvi = vec![Some(0.6), Some(-0.1), Some(0.4)];
        let imp = vec![Some(0.1), Some(0.7), Some(0.9)];
        let stats = p.land_use_stats(&ndvi, &imp, 100.0);
        assert!(!stats.is_empty());
        let total_area: f64 = stats.iter().map(|(_, a)| a).sum();
        assert!((total_area - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_solar_analysis() {
        let p = default_plugin();
        // 30m building, neighbor at 70m → winter shadow ~60.2m < 70m → compliant
        let r = p.solar_analysis(30.0, 70.0);
        assert!(r.winter_shadow_m > r.summer_shadow_m);
        assert!(r.compliant);
        // 30m building, neighbor at 40m → winter shadow ~60.2m > 40m → non-compliant
        let r2 = p.solar_analysis(30.0, 40.0);
        assert!(!r2.compliant);
    }

    #[test]
    fn test_uhi() {
        let p = default_plugin();
        let r = p.uhi_index(0.8, 0.5, 0.1);
        assert!(r.uhi_index > 0.5);
    }

    #[test]
    fn test_green() {
        let p = default_plugin();
        assert!((p.green_ratio(300.0, 1000.0) - 0.3).abs() < 0.01);
        assert!((p.green_per_capita(3000.0, 300) - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_ventilation() {
        let p = default_plugin();
        let r = p.ventilation(35.0, 0.3);
        assert!(r.roughness_length_m > 0.0);
        assert!(r.ventilation_index >= 0.0 && r.ventilation_index <= 1.0);
    }

    #[test]
    fn test_assess() {
        let p = default_plugin();
        let a = p.assess(
            3500.0,
            300.0,
            1000.0,
            200.0,
            500,
            0.6,
            &[Some(0.5), Some(-0.1)],
            &[Some(0.2), Some(0.8)],
        );
        assert!((a.far - 3.5).abs() < 0.01);
        assert!(!a.land_use.is_empty());
    }
}
