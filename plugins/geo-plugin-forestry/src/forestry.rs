//! 林业碳汇计量核心逻辑。
//!
//! ## IPCC 蓄积量法
//!
//! 碳储量 = 蓄积量 × BEF × WD × (1+R) × CF
//! 碳汇量 = Δ碳储量 × 44/12
//!
//! - BEF: 生物量扩展因子 (树干→全株)
//! - WD:  木材密度 (t d.m./m³)
//! - R:   根冠比
//! - CF:  含碳率
//! - 44/12: C→CO₂ 转换系数

use geo_core::errors::GeoResult;
use geo_core::types::BBox;
use geo_raster::RasterBand;
use geo_raster::ndvi::compute_ndvi;
use serde::Serialize;

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

    /// 基于 NDVI 和样地蓄积量估算碳储量变化。
    ///
    /// ## 参数
    /// - `red/nir`: 基准年和评估年的红/近红外波段
    /// - `sample_volume_m3_ha`: 样地平均蓄积量 (m³/ha)，从野外调查获取
    /// - `forest_area_ha`: 林地面积 (ha)
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

        // 1. 两期 NDVI
        let ndvi_base = compute_ndvi(baseline_red, baseline_nir)?;
        let ndvi_curr = compute_ndvi(assessment_red, assessment_nir)?;

        let mean_base = ndvi_base.mean_ndvi.unwrap_or(0.0);
        let mean_curr = ndvi_curr.mean_ndvi.unwrap_or(0.0);

        // 2. NDVI → 蓄积量代理
        // V = V_sample × (NDVI_current / NDVI_baseline)
        let volume_factor = if mean_base > 0.0 { mean_curr / mean_base } else { 1.0 };
        let volume_curr_m3_ha = sample_volume_m3_ha * volume_factor;
        let volume_change_m3 = (volume_curr_m3_ha - sample_volume_m3_ha) * forest_area_ha;

        // 3. IPCC 蓄积量→碳储量
        // C = V × BEF × WD × (1+R) × CF
        let bef = cp.biomass_expansion_factor;
        let wd = cp.wood_density;
        let r = cp.root_shoot_ratio;
        let cf = cp.carbon_fraction;
        let co2_c = cp.co2_c_ratio;

        let carbon_stock_change_tc = volume_change_m3 * bef * wd * (1.0 + r) * cf;
        // 碳储量增加 = 从大气吸收 CO₂ = 负排放（碳汇）
        let carbon_sink_tco2e = -carbon_stock_change_tc * co2_c;
        let years = (assessment_year - baseline_year).max(1) as f64;
        let annual_sink = carbon_sink_tco2e / years;

        // 4. CCER 判断
        let ccer_applicable = annual_sink < -10.0 && forest_area_ha >= 100.0;

        // 5. 评级
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
            return Err(geo_core::GeoError::Validation("need at least 4 time steps".into()));
        }

        // 逐像素 MK 趋势
        let mut ts = geo_temporal::raster_ts::RasterTimeSeries::new();
        for (i, band) in ndvi_series.iter().enumerate() {
            ts.add(years[i], band.clone())?;
        }

        let tau_map = ts.pixelwise_trend()?;
        let stats = ts.yearly_stats();

        // 统计正趋势像素占比
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

        // NDVI 上升 → 碳汇应为负值（吸收CO₂）
        assert!(result.carbon_sink_tco2e < 0.0);
        assert!(result.annual_sink_tco2_per_yr < 0.0);
        assert!(!result.grade.is_empty());
    }
}
