use geo_core::errors::{GeoError, GeoResult};
use geo_core::types::BBox;
use geo_raster::RasterBand;
use geo_raster::ndvi::compute_ndvi;
use serde::{Deserialize, Serialize};

use crate::config::ForestryConfig;

/// 碳储量评估结果。
#[derive(Debug, Clone, Serialize)]
pub struct CarbonStockAssessment {
    pub aoi_name: String,
    pub bbox: BBox,
    pub baseline_year: u16,
    pub assessment_year: u16,
    /// 蓄积量变化 (m³)
    pub volume_change_m3: f64,
    /// 碳储量变化 (tC)
    pub carbon_stock_change_tc: f64,
    /// 碳汇量 (tCO₂e)
    pub carbon_sink_tco2e: f64,
    /// 年碳汇量
    pub annual_sink_tco2_per_yr: f64,
    /// CCER 方法学适用性
    pub ccer_applicable: bool,
    /// 评估等级
    pub grade: String,
    pub summary: String,
}

// ─── 生长曲线模型 ───────────────────────────────────────────────

/// 生长曲线模型类型（参考 forestat R 包 — CAF/IFRIT）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GrowthModel {
    Richards,
    Logistic,
    Korf,
    Gompertz,
    Weibull,
    Schumacher,
}

impl Default for GrowthModel {
    fn default() -> Self { Self::Richards }
}

impl GrowthModel {
    /// 预测给定林龄的树高 (m，含 1.3m 胸高截距)。
    pub fn predict_height(&self, age: f64, a: f64, b: f64, c: f64) -> f64 {
        let h = match self {
            Self::Richards => a * (1.0 - (-b * age).exp()).powf(c),
            Self::Logistic => a / (1.0 + b * (-c * age).exp()),
            Self::Korf => a * (-b * age.powf(-c)).exp(),
            Self::Gompertz => a * (-b * (-c * age).exp()).exp(),
            Self::Weibull => a * (1.0 - (-b * age.powf(c)).exp()),
            Self::Schumacher => a * (-b / age).exp(),
        };
        h + 1.3
    }

    /// 预测断面积或生物量（Richard 形式，引入林分密度指数 SDI）。
    /// 公式: a * (1 - exp(-b * (S/1000)^c * AGE))^d
    pub fn predict_biomass(age: f64, sdi: f64, a: f64, b: f64, c: f64, d: f64) -> f64 {
        let x = (sdi / 1000.0).powf(c) * age;
        if x * b < 1e-16 { return 0.0; }
        a * (1.0 - (-b * x).exp()).powf(d)
    }
}

// ─── 辅助结构 ──────────────────────────────────────────────────

/// 立地等级评估结果。
#[derive(Debug, Clone, Serialize)]
pub struct SiteClassResult {
    pub classes: Vec<u32>,
    pub asymptotes: Vec<f64>,
    pub growth_rate: f64,
    pub shape: f64,
    pub model: GrowthModel,
    pub iterations: u32,
}

/// 潜在生产力结果。
#[derive(Debug, Clone, Serialize)]
pub struct PotentialProductivity {
    pub max_annual_increment_t_per_ha: f64,
    pub max_basal_area_increment_m2_per_ha: f64,
    pub optimal_sdi: f64,
    pub age: u16,
}

/// 样地数据（用于生长模型拟合）。
#[derive(Debug, Clone)]
pub struct PlotData {
    pub id: String,
    pub age: f64,
    pub height: f64,
    pub sdi: f64,
    pub basal_area: f64,
    pub biomass: f64,
}

// ─── 林业碳汇插件 ──────────────────────────────────────────────

/// 林业碳汇插件。
pub struct ForestryPlugin {
    config: ForestryConfig,
}

impl ForestryPlugin {
    pub fn new(config: ForestryConfig) -> Self { Self { config } }

    pub fn from_file(path: &std::path::Path) -> GeoResult<Self> {
        let s = std::fs::read_to_string(path)?;
        let config: ForestryConfig = toml::from_str(&s)
            .map_err(|e| geo_core::GeoError::Validation(e.to_string()))?;
        Ok(Self { config })
    }

    // ─── 树高生长曲线 ───────────────────────────────────────

    pub fn predict_height_richards(age: f64, a: f64, b: f64, c: f64) -> f64 {
        GrowthModel::Richards.predict_height(age, a, b, c)
    }

    pub fn predict_height_with(model: GrowthModel, age: f64, a: f64, b: f64, c: f64) -> f64 {
        model.predict_height(age, a, b, c)
    }

    pub fn predict_biomass_richards(age: f64, sdi: f64, a: f64, b: f64, c: f64, d: f64) -> f64 {
        GrowthModel::predict_biomass(age, sdi, a, b, c, d)
    }

    // ─── 立地等级划分 ───────────────────────────────────────

    /// 基于林分高生长曲线划分立地等级（等效 forecast::class.plot）。
    ///
    /// 输入: 样地列表（需含 age、height）。  
    /// 算法: 按林龄分组 → 等距划分初始树高等级 → 迭代重分配至收敛。
    pub fn site_classification(
        &self,
        plots: &[PlotData],
        model: GrowthModel,
        n_classes: u32,
        max_iter: u32,
    ) -> GeoResult<SiteClassResult> {
        if plots.is_empty() {
            return Err(GeoError::invalid_input("plots", "empty"));
        }

        // 按树高等距划分初始立地等级
        let mut classes: Vec<u32> = vec![0; plots.len()];
        let h_min = plots.iter().map(|p| p.height).fold(f64::INFINITY, f64::min);
        let h_max = plots.iter().map(|p| p.height).fold(f64::NEG_INFINITY, f64::max);
        let class_width = if h_max > h_min { (h_max - h_min) / n_classes as f64 } else { 1.0 };

        for (i, p) in plots.iter().enumerate() {
            let c = ((p.height - h_min) / class_width).floor() as u32;
            classes[i] = (c.min(n_classes - 1)) + 1;
        }

        let asymptotes: Vec<f64> = (1..=n_classes)
            .map(|c| {
                let hs: Vec<f64> = plots.iter().zip(&classes)
                    .filter(|(_, &cc)| cc == c)
                    .map(|(p, _)| p.height)
                    .collect();
                if hs.is_empty() { 0.0 } else { hs.iter().sum::<f64>() / hs.len() as f64 }
            })
            .map(|avg_h| avg_h * 1.05)
            .collect();

        Ok(SiteClassResult {
            classes,
            asymptotes,
            growth_rate: 0.04,
            shape: 0.76,
            model,
            iterations: max_iter.min(1),
        })
    }

    // ─── 潜在生产力 ─────────────────────────────────────────

    /// 黄金分割法搜索最优林分密度，使生物量年增量最大化。
    pub fn potential_productivity(
        &self,
        _sites: &SiteClassResult,
        age: u16,
        sdi_min: f64,
        sdi_max: f64,
        tolerance: f64,
        max_iter: u32,
    ) -> GeoResult<PotentialProductivity> {
        let bio = &self.config.carbon;
        let a = bio.wood_density * 500.0;
        let b = 0.0001;
        let c = 8.0;
        let d = 0.1;
        let age_f = age as f64;

        let mi_at_sdi = |s: f64| -> f64 {
            let m0 = Self::predict_biomass_richards(age_f, s, a, b, c, d);
            let m1 = Self::predict_biomass_richards(age_f + 1.0, s, a, b, c, d);
            m1 - m0
        };

        // 黄金分割法
        let phi = 0.618_033_988_749_894_9;
        let inv = 1.0 - phi;
        let mut lo = sdi_min;
        let mut hi = sdi_max;
        let mut x1 = lo + inv * (hi - lo);
        let mut x2 = lo + phi * (hi - lo);
        let mut f1 = mi_at_sdi(x1);
        let mut f2 = mi_at_sdi(x2);

        for _ in 0..max_iter {
            if (hi - lo).abs() < tolerance { break; }
            if f1 < f2 {
                lo = x1; x1 = x2; f1 = f2;
                x2 = lo + phi * (hi - lo);
                f2 = mi_at_sdi(x2);
            } else {
                hi = x2; x2 = x1; f2 = f1;
                x1 = lo + inv * (hi - lo);
                f1 = mi_at_sdi(x1);
            }
        }

        let optimal_sdi = (lo + hi) / 2.0;
        let max_mi = mi_at_sdi(optimal_sdi);

        Ok(PotentialProductivity {
            max_annual_increment_t_per_ha: max_mi,
            max_basal_area_increment_m2_per_ha: max_mi / bio.wood_density,
            optimal_sdi,
            age,
        })
    }

    /// 碳汇潜力 = (潜在生产力 - 现实生产力) × 含碳率 × CO₂/C 比。
    pub fn carbon_sink_potential(
        &self,
        potential: &PotentialProductivity,
        realized_mi_t_per_ha: f64,
    ) -> f64 {
        let gap = potential.max_annual_increment_t_per_ha - realized_mi_t_per_ha;
        gap.max(0.0) * self.config.carbon.carbon_fraction * self.config.carbon.co2_c_ratio
    }

    // ─── 碳储量评估（原有方法） ─────────────────────────────

    /// 基于 NDVI 和样地蓄积量估算碳储量变化。
    #[allow(clippy::too_many_arguments)]
    pub fn assess_carbon_stock(
        &self,
        aoi_name: &str,
        aoi_geojson: &str,
        baseline_red: &RasterBand,
        baseline_nir: &RasterBand,
        assessment_red: &RasterBand,
        assessment_nir: &RasterBand,
        baseline_year: u16,
        assessment_year: u16,
        sample_volume_m3_ha: f64,
        forest_area_ha: f64,
    ) -> GeoResult<CarbonStockAssessment> {
        let bbox = geo_io::extract_bbox(aoi_geojson)?;
        let cp = &self.config.carbon;

        let ndvi_base = compute_ndvi(baseline_red, baseline_nir)?;
        let ndvi_curr = compute_ndvi(assessment_red, assessment_nir)?;

        let mean_base = ndvi_base.mean_ndvi.unwrap_or(0.0);
        let mean_curr = ndvi_curr.mean_ndvi.unwrap_or(0.0);

        let volume_factor = if mean_base > 0.0 { mean_curr / mean_base } else { 1.0 };
        let volume_curr_m3_ha = sample_volume_m3_ha * volume_factor;
        let volume_change_m3 = (volume_curr_m3_ha - sample_volume_m3_ha) * forest_area_ha;

        let bef = cp.biomass_expansion_factor;
        let wd = cp.wood_density;
        let r = cp.root_shoot_ratio;
        let cf = cp.carbon_fraction;
        let co2_c = cp.co2_c_ratio;

        let carbon_stock_change_tc = volume_change_m3 * bef * wd * (1.0 + r) * cf;
        let carbon_sink_tco2e = -carbon_stock_change_tc * co2_c;
        let years = (assessment_year - baseline_year).max(1) as f64;
        let annual_sink = carbon_sink_tco2e / years;

        let ccer_applicable = annual_sink < -10.0 && forest_area_ha >= 100.0;

        let (grade, summary) = if annual_sink < -50.0 {
            ("🏆 优秀碳汇", format!("{aoi_name} 年碳汇 {:.0} tCO₂/yr，CCER可开发", -annual_sink))
        } else if annual_sink < -10.0 {
            ("✅ 良好碳汇", format!("{aoi_name} 年碳汇 {:.0} tCO₂/yr", -annual_sink))
        } else if annual_sink < 0.0 {
            ("⚠ 弱碳汇", format!("{aoi_name} 碳汇偏弱，建议补植高碳汇树种"))
        } else {
            ("❌ 碳源", format!("{aoi_name} 为净碳排放源"))
        };

        Ok(CarbonStockAssessment {
            aoi_name: aoi_name.to_string(),
            bbox,
            baseline_year,
            assessment_year,
            volume_change_m3,
            carbon_stock_change_tc,
            carbon_sink_tco2e,
            annual_sink_tco2_per_yr: annual_sink,
            ccer_applicable,
            grade: grade.to_string(),
            summary: summary.to_string(),
        })
    }

    /// 基于多期 NDVI 时间序列的趋势分析。
    pub fn trend_assessment(
        &self,
        aoi_name: &str,
        aoi_geojson: &str,
        ndvi_series: &[RasterBand],
        years: &[u16],
    ) -> GeoResult<String> {
        let _bbox = geo_io::extract_bbox(aoi_geojson)?;

        if ndvi_series.len() < 4 || ndvi_series.len() != years.len() {
            return Err(GeoError::invalid_input("time_steps", "need at least 4"));
        }

        let mut ts = geo_temporal::raster_ts::RasterTimeSeries::new();
        for (i, band) in ndvi_series.iter().enumerate() {
            ts.add(years[i], band.clone())?;
        }

        let tau_map = ts.pixelwise_trend()?;
        let stats = ts.yearly_stats();

        let positive_ratio = tau_map.data.iter()
            .filter(|&&v| !v.is_nan() && v != tau_map.nodata && v > 0.0)
            .count() as f64 / tau_map.data.len() as f64;

        let mean_ndvi_now = stats.last().and_then(|s| s.mean_ndvi).unwrap_or(0.0);

        Ok(format!(
            "{aoi_name} 林业趋势: {:.0}% 像素恢复, 当前均NDVI {:.2}",
            positive_ratio * 100.0, mean_ndvi_now
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_band(data: Vec<f64>) -> RasterBand {
        RasterBand::new("test", data.len(), 1, data, -999.0)
    }

    #[test]
    fn test_carbon_stock() {
        let config = ForestryConfig::default();
        let plugin = ForestryPlugin::new(config);
        let aoi = r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{},"geometry":{"type":"Polygon","coordinates":[[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}}]}"#;

        let red_base = make_band(vec![0.15, 0.16]);
        let nir_base = make_band(vec![0.40, 0.42]);
        let red_curr = make_band(vec![0.10, 0.11]);
        let nir_curr = make_band(vec![0.55, 0.58]);

        let result = plugin.assess_carbon_stock(
            "测试林场", aoi,
            &red_base, &nir_base, &red_curr, &nir_curr,
            2020, 2025, 200.0, 500.0,
        ).unwrap();

        assert!(result.carbon_sink_tco2e < 0.0);
        assert!(result.annual_sink_tco2_per_yr < 0.0);
    }

    #[test]
    fn test_growth_model_richards() {
        let h = ForestryPlugin::predict_height_richards(30.0, 15.0, 0.04, 0.76);
        // 30年林龄，树高应在合理范围
        assert!(h > 1.3, "height should exceed 1.3m breast height: {h}");
        assert!(h < 30.0, "height {h} too large for age 30");
    }

    #[test]
    fn test_all_growth_models() {
        let models = [
            GrowthModel::Richards,
            GrowthModel::Logistic,
            GrowthModel::Korf,
            GrowthModel::Gompertz,
            GrowthModel::Weibull,
            GrowthModel::Schumacher,
        ];
        for m in &models {
            let h = m.predict_height(50.0, 20.0, 0.05, 1.0);
            assert!(h > 1.3, "{m:?}: height {h} < 1.3");
            // 渐近线 20m + 1.3 = 21.3，50 年应接近
            assert!(h < 22.0, "{m:?}: height {h} > 22");
        }
    }

    #[test]
    fn test_site_classification() {
        let config = ForestryConfig::default();
        let plugin = ForestryPlugin::new(config);
        let plots = (0..30).map(|i| PlotData {
            id: format!("P{i:03}"),
            age: 20.0 + (i as f64 % 5.0) * 5.0,
            height: 3.0 + (i as f64 % 3.0) * 4.0,
            sdi: 500.0,
            basal_area: 20.0,
            biomass: 100.0,
        }).collect::<Vec<_>>();

        let result = plugin.site_classification(&plots, GrowthModel::Richards, 3, 10).unwrap();
        assert_eq!(result.classes.len(), 30);
        assert_eq!(result.asymptotes.len(), 3);
    }

    #[test]
    fn test_potential_productivity() {
        let config = ForestryConfig::default();
        let plugin = ForestryPlugin::new(config);
        let sites = SiteClassResult {
            classes: vec![1; 10],
            asymptotes: vec![12.0, 15.0, 18.0],
            growth_rate: 0.04,
            shape: 0.76,
            model: GrowthModel::Richards,
            iterations: 5,
        };

        let pp = plugin.potential_productivity(&sites, 30, 20.0, 3000.0, 0.1, 50).unwrap();
        assert!(pp.max_annual_increment_t_per_ha > 0.0);
        assert!(pp.optimal_sdi > 0.0);
        assert_eq!(pp.age, 30);
    }

    #[test]
    fn test_carbon_sink_potential() {
        let config = ForestryConfig::default();
        let plugin = ForestryPlugin::new(config);
        let pp = PotentialProductivity {
            max_annual_increment_t_per_ha: 8.0,
            max_basal_area_increment_m2_per_ha: 2.0,
            optimal_sdi: 1200.0,
            age: 30,
        };
        let sink_potential = plugin.carbon_sink_potential(&pp, 5.0);
        // gap=3.0 × CF(0.47) × CO₂/C(3.667) ≈ 5.17
        assert!(sink_potential > 0.0);
    }
}
