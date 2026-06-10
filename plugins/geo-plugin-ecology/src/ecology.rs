//! 生态修复评估核心逻辑。
//!
//! 矿山修复典型案例：
//! 1. 读取两期 NDVI 影像（修复前/后）
//! 2. 计算 NDVI 差值
//! 3. 分区统计植被恢复面积
//! 4. 碳汇计算（直接调用 geo-carbon-math）
//! 5. 组装评估报告

use crate::config::EcologyConfig;
use geo_carbon_math::{CarbonEngine, CarbonReport, EmissionFactor, GeoFeature};
use geo_core::errors::{GeoError, GeoResult};
use geo_core::types::BBox;
use geo_raster::ndvi::{compute_ndvi, ndvi_difference, NdviResult};
use geo_raster::RasterBand;
use serde::{Deserialize, Serialize};

/// 生态修复评估结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestorationAssessment {
    /// AOI 名称。
    pub aoi_name: String,
    /// 基准年份。
    pub baseline_year: u16,
    /// 评估年份。
    pub assessment_year: u16,

    /// AOI 边界框。
    pub bbox: BBox,

    // ── NDVI 分析 ──
    /// 基准年 NDVI 统计。
    pub baseline_ndvi: NdviStats,
    /// 评估年 NDVI 统计。
    pub assessment_ndvi: NdviStats,
    /// NDVI 差值统计。
    pub ndvi_change: NdviChange,

    // ── 碳核算 ──
    /// 碳核算报告（直接调用碳核算引擎）。
    pub carbon: CarbonReport,

    /// 评估结论。
    pub conclusion: RestorationConclusion,

    /// 计算时间。
    pub calculated_at: String,
}

/// NDVI 统计。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NdviStats {
    pub year: u16,
    pub mean_ndvi: Option<f64>,
    pub healthy_ratio: Option<f64>,
    pub degraded_ratio: Option<f64>,
    pub valid_pixels: usize,
}

/// NDVI 变化统计。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NdviChange {
    /// NDVI 差值均值。
    pub mean_diff: Option<f64>,
    /// 植被改善面积比例（NDVI 差 > improvement_threshold）。
    pub improved_ratio: Option<f64>,
    /// 植被退化面积比例（NDVI 差 < degradation_threshold）。
    pub degraded_ratio: Option<f64>,
    /// 稳定区域比例。
    pub stable_ratio: Option<f64>,
}

/// 恢复评估结论。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestorationConclusion {
    /// 总体评级："优秀"/"良好"/"一般"/"差"。
    pub grade: String,
    /// 是否达到恢复目标（植被覆盖显著改善 + 碳汇为正）。
    pub target_met: bool,
    /// 植被恢复面积占比。
    pub restored_ratio: Option<f64>,
    /// 碳汇量 tCO₂e/yr（负值表示碳汇，取绝对值显示）。
    pub carbon_sink_tco2_per_yr: f64,
    /// 详细描述。
    pub summary: String,
}

/// 生态修复插件。
pub struct EcologyPlugin {
    config: EcologyConfig,
}

impl EcologyPlugin {
    /// 从配置创建插件。
    pub fn new(config: EcologyConfig) -> Self {
        Self { config }
    }

    /// 获取配置引用。
    pub fn config(&self) -> &EcologyConfig {
        &self.config
    }

    /// 从 rules.toml 文件路径加载。
    pub fn load_from_file(path: &std::path::Path) -> GeoResult<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| GeoError::Io(e))?;
        let config: EcologyConfig = toml::from_str(&content)
            .map_err(|e| GeoError::Validation(format!("Invalid rules.toml: {e}")))?;
        Ok(Self { config })
    }

    // ── NDVI 变化检测 ──

    /// 计算 NDVI 变化（仅 NDVI 分析，不含碳核算）。
    pub fn detect_ndvi_change(
        &self,
        baseline_red: &RasterBand,
        baseline_nir: &RasterBand,
        assessment_red: &RasterBand,
        assessment_nir: &RasterBand,
    ) -> GeoResult<(NdviResult, NdviResult)> {
        let prev = compute_ndvi(baseline_red, baseline_nir)?;
        let curr = compute_ndvi(assessment_red, assessment_nir)?;
        Ok((prev, curr))
    }

    // ── 完整矿山修复评估 ──

    /// 运行完整的生态修复评估。
    ///
    /// ## 参数
    /// - `aoi_name`: AOI 名称（如"XX矿山修复区"）
    /// - `aoi_geojson`: GeoJSON FeatureCollection 字符串
    /// - `baseline_red/nir`: 基准年遥感波段
    /// - `assessment_red/nir`: 评估年遥感波段
    /// - `baseline_year`: 基准年
    /// - `assessment_year`: 评估年
    /// - `raster_bbox`: 栅格覆盖的地理范围
    pub fn assess_restoration(
        &self,
        aoi_name: &str,
        aoi_geojson: &str,
        baseline_red: &RasterBand,
        baseline_nir: &RasterBand,
        assessment_red: &RasterBand,
        assessment_nir: &RasterBand,
        baseline_year: u16,
        assessment_year: u16,
        _raster_bbox: BBox,
    ) -> GeoResult<RestorationAssessment> {
        // 1. 解析 AOI
        let aoi_bbox = geo_io::extract_bbox(aoi_geojson)?;

        // 2. 计算两期 NDVI
        let prev_ndvi = compute_ndvi(baseline_red, baseline_nir)?;
        let curr_ndvi = compute_ndvi(assessment_red, assessment_nir)?;

        // 3. NDVI 差值
        let ndvi_diff = ndvi_difference(&prev_ndvi, &curr_ndvi)?;

        // 4. 分区统计 NDVI 变化（计算改善/退化面积比例）
        let thresholds = &self.config.ndvi;
        let total_valid = ndvi_diff.valid_count();
        let (improved_count, degraded_count, stable_count) = if total_valid > 0 {
            let (imp, deg, stab) = ndvi_diff.data.iter()
                .filter(|v| !v.is_nan() && **v != ndvi_diff.nodata)
                .fold((0usize, 0usize, 0usize), |(imp, deg, stab), v| {
                    if *v > thresholds.improvement_threshold {
                        (imp + 1, deg, stab)
                    } else if *v < thresholds.degradation_threshold {
                        (imp, deg + 1, stab)
                    } else {
                        (imp, deg, stab + 1)
                    }
                });
            (imp, deg, stab)
        } else {
            (0, 0, 0)
        };

        // 5. 碳核算：从 GeoJSON 提取 features → 调用 geo-carbon-math
        let carbon = self.calculate_carbon(aoi_geojson, assessment_year)?;

        // 6. 评估结论
        let restored_ratio = if total_valid > 0 {
            Some(improved_count as f64 / total_valid as f64)
        } else { None };

        let carbon_sink = carbon.total_emission_tco2e.abs();
        let improved_enough = restored_ratio.unwrap_or(0.0) >= 0.3; // 30%+ pixels improved
        let target_met = improved_enough && carbon.is_net_sink();
        let grade = match (target_met, restored_ratio.unwrap_or(0.0)) {
            (true, r) if r >= 0.6 => "优秀",
            (true, _) => "良好",
            (false, r) if r >= 0.2 => "一般",
            _ => "差",
        };

        let summary = if target_met {
            format!(
                "{}区域生态修复达标：{:.1}% 像素植被显著改善，年碳汇约 {:.1} tCO₂。",
                aoi_name,
                restored_ratio.unwrap_or(0.0) * 100.0,
                carbon_sink
            )
        } else {
            format!(
                "{}区域生态修复未完全达标：仅 {:.1}% 像素植被显著改善。建议加强植被恢复措施。",
                aoi_name,
                restored_ratio.unwrap_or(0.0) * 100.0
            )
        };

        Ok(RestorationAssessment {
            aoi_name: aoi_name.to_string(),
            baseline_year,
            assessment_year,
            bbox: aoi_bbox,
            baseline_ndvi: NdviStats {
                year: baseline_year,
                mean_ndvi: prev_ndvi.mean_ndvi,
                healthy_ratio: prev_ndvi.healthy_ratio,
                degraded_ratio: prev_ndvi.degraded_ratio,
                valid_pixels: prev_ndvi.valid_pixels,
            },
            assessment_ndvi: NdviStats {
                year: assessment_year,
                mean_ndvi: curr_ndvi.mean_ndvi,
                healthy_ratio: curr_ndvi.healthy_ratio,
                degraded_ratio: curr_ndvi.degraded_ratio,
                valid_pixels: curr_ndvi.valid_pixels,
            },
            ndvi_change: NdviChange {
                mean_diff: ndvi_diff.mean(),
                improved_ratio: if total_valid > 0 {
                    Some(improved_count as f64 / total_valid as f64)
                } else { None },
                degraded_ratio: if total_valid > 0 {
                    Some(degraded_count as f64 / total_valid as f64)
                } else { None },
                stable_ratio: if total_valid > 0 {
                    Some(stable_count as f64 / total_valid as f64)
                } else { None },
            },
            carbon,
            conclusion: RestorationConclusion {
                grade: grade.to_string(),
                target_met,
                restored_ratio,
                carbon_sink_tco2_per_yr: carbon_sink,
                summary,
            },
            calculated_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// 碳核算（直接调用 geo-carbon-math，不依赖 geo-plugin-carbon）。
    fn calculate_carbon(
        &self,
        aoi_geojson: &str,
        year: u16,
    ) -> GeoResult<CarbonReport> {
        let fc: serde_json::Value = serde_json::from_str(aoi_geojson)
            .map_err(|e| GeoError::Validation(format!("Invalid GeoJSON: {e}")))?;

        let features_json = fc["features"].as_array()
            .ok_or_else(|| GeoError::Validation("No 'features' array".into()))?;

        let mut features = Vec::with_capacity(features_json.len());
        for f in features_json {
            let feat_str = serde_json::to_string(f)
                .map_err(|e| GeoError::Serde(e))?;
            match GeoFeature::from_feature_json(&feat_str) {
                Ok(gf) => features.push(gf),
                Err(_) => continue,
            }
        }

        let cp = &self.config.carbon;
        let factors = vec![
            EmissionFactor::new("forest", cp.forest, cp.source.as_str()),
            EmissionFactor::new("grassland", cp.grassland, cp.source.as_str()),
            EmissionFactor::new("wetland", cp.wetland, cp.source.as_str()),
            EmissionFactor::new("cropland", cp.cropland, cp.source.as_str()),
            EmissionFactor::new("built_up", cp.built_up, cp.source.as_str()),
            EmissionFactor::new("bare", cp.bare, cp.source.as_str()),
        ];

        let engine = CarbonEngine::new();
        let mut report = engine.calculate(&features, &factors, year)
            .map_err(|e| GeoError::Validation(e))?;
        report.methodology = Some(format!("IPCC Tier 1 — {}", cp.source));
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_band_with_noise(data: Vec<f64>) -> RasterBand {
        RasterBand::new("band", 1, data.len(), data, -999.0)
    }

    #[test]
    fn test_mine_restoration_assessment() {
        let config: EcologyConfig = toml::from_str(r#"
            [plugin]
            name = "ecology"
            version = "0.1.0"
            description = "test"

            [ndvi]
            healthy_min = 0.5
            degraded_max = 0.2
            improvement_threshold = 0.1
            degradation_threshold = -0.1

            [carbon]
            source = "IPCC_2019"
            forest = -5.0
            grassland = -1.2
            built_up = 2.0
            bare = 0.0
        "#).unwrap();

        let plugin = EcologyPlugin::new(config);

        // 模拟 2020 年矿区：低 NDVI（退化状态）
        let red_2020 = make_band_with_noise(vec![0.40, 0.45, 0.42, 0.44]); // 高红波段 = 裸地
        let nir_2020 = make_band_with_noise(vec![0.15, 0.18, 0.16, 0.17]); // 低近红外 = 少植被

        // 模拟 2025 年修复后：NDVI 回升
        let red_2025 = make_band_with_noise(vec![0.10, 0.12, 0.35, 0.40]); // 红波段降低
        let nir_2025 = make_band_with_noise(vec![0.45, 0.50, 0.20, 0.18]); // 近红外升高

        let aoi = r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "properties": {"class": "forest"},
                    "geometry": {"type": "Polygon", "coordinates": [[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}
                }
            ]
        }"#;

        let raster_bbox = BBox::new(104.0, 30.5, 104.1, 30.6);

        let assessment = plugin.assess_restoration(
            "XX矿山修复区",
            aoi,
            &red_2020, &nir_2020,
            &red_2025, &nir_2025,
            2020, 2025,
            raster_bbox,
        ).unwrap();

        // 验证结构
        assert_eq!(assessment.aoi_name, "XX矿山修复区");
        assert_eq!(assessment.baseline_year, 2020);
        assert_eq!(assessment.assessment_year, 2025);

        // NDVI 应该有所恢复
        let base_mean = assessment.baseline_ndvi.mean_ndvi.unwrap_or(0.0);
        let assess_mean = assessment.assessment_ndvi.mean_ndvi.unwrap_or(0.0);
        assert!(assess_mean > base_mean, "修复后 NDVI 应高于修复前");

        // 碳核算应该有结果
        assert!(assessment.carbon.classes.len() > 0);
        assert!(assessment.carbon.total_emission_tco2e < 0.0, "应为净碳汇");

        // 结论应包含关键信息
        assert!(!assessment.conclusion.summary.is_empty());
        assert!(assessment.conclusion.carbon_sink_tco2_per_yr > 0.0);

        println!("Assessment: {assessment:#?}");
    }

    #[test]
    fn test_carbon_not_via_plugin() {
        // 验证我们不 import geo-plugin-carbon
        let config: EcologyConfig = toml::from_str(r#"
            [plugin]
            name = "ecology"
            version = "0.1.0"
            description = "test"
        "#).unwrap();

        let plugin = EcologyPlugin::new(config);

        let result = plugin.calculate_carbon(
            r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"class":"forest"},"geometry":{"type":"Polygon","coordinates":[[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}}]}"#,
            2025,
        ).unwrap();

        assert_eq!(result.classes.len(), 1);
        assert_eq!(result.classes[0].landcover_class, "forest");
    }
}
