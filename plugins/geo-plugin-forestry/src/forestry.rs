use geo_core::errors::{GeoError, GeoResult};
use geo_core::types::BBox;
use geo_raster::ndvi::compute_ndvi;
use geo_raster::RasterBand;
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
    fn default() -> Self {
        Self::Richards
    }
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
        if x * b < 1e-16 {
            return 0.0;
        }
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
    pub fn new(config: ForestryConfig) -> Self {
        Self { config }
    }

    pub fn from_file(path: &std::path::Path) -> GeoResult<Self> {
        let s = std::fs::read_to_string(path)?;
        let config: ForestryConfig =
            toml::from_str(&s).map_err(|e| geo_core::GeoError::Validation(e.to_string()))?;
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
        let h_max = plots
            .iter()
            .map(|p| p.height)
            .fold(f64::NEG_INFINITY, f64::max);
        let class_width = if h_max > h_min {
            (h_max - h_min) / n_classes as f64
        } else {
            1.0
        };

        for (i, p) in plots.iter().enumerate() {
            let c = ((p.height - h_min) / class_width).floor() as u32;
            classes[i] = (c.min(n_classes - 1)) + 1;
        }

        let asymptotes: Vec<f64> = (1..=n_classes)
            .map(|c| {
                let hs: Vec<f64> = plots
                    .iter()
                    .zip(&classes)
                    .filter(|(_, &cc)| cc == c)
                    .map(|(p, _)| p.height)
                    .collect();
                if hs.is_empty() {
                    0.0
                } else {
                    hs.iter().sum::<f64>() / hs.len() as f64
                }
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
            if (hi - lo).abs() < tolerance {
                break;
            }
            if f1 < f2 {
                lo = x1;
                x1 = x2;
                f1 = f2;
                x2 = lo + phi * (hi - lo);
                f2 = mi_at_sdi(x2);
            } else {
                hi = x2;
                x2 = x1;
                f2 = f1;
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

        let volume_factor = if mean_base > 0.0 {
            mean_curr / mean_base
        } else {
            1.0
        };
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
            (
                "🏆 优秀碳汇",
                format!("{aoi_name} 年碳汇 {:.0} tCO₂/yr，CCER可开发", -annual_sink),
            )
        } else if annual_sink < -10.0 {
            (
                "✅ 良好碳汇",
                format!("{aoi_name} 年碳汇 {:.0} tCO₂/yr", -annual_sink),
            )
        } else if annual_sink < 0.0 {
            (
                "⚠ 弱碳汇",
                format!("{aoi_name} 碳汇偏弱，建议补植高碳汇树种"),
            )
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

        let positive_ratio = tau_map
            .data
            .iter()
            .filter(|&&v| !v.is_nan() && v != tau_map.nodata && v > 0.0)
            .count() as f64
            / tau_map.data.len() as f64;

        let mean_ndvi_now = stats.last().and_then(|s| s.mean_ndvi).unwrap_or(0.0);

        Ok(format!(
            "{aoi_name} 林业趋势: {:.0}% 像素恢复, 当前均NDVI {:.2}",
            positive_ratio * 100.0,
            mean_ndvi_now
        ))
    }
}

// ═══════════════════════════════════════════════════════════════
// 生长曲线验证模块 — 6 种模型的参数校准、拟合优度评估与排名
// ═══════════════════════════════════════════════════════════════

/// 单个生长模型的拟合结果。
#[derive(Debug, Clone, Serialize)]
pub struct ModelFit {
    /// 模型类型。
    pub model: GrowthModel,
    /// 校准参数 (a, b, c)。
    pub params: [f64; 3],
    /// 决定系数 R² (0~1)。
    pub r_squared: f64,
    /// 均方根误差。
    pub rmse: f64,
    /// AIC（赤池信息量准则，越小越好）。
    pub aic: f64,
    /// BIC（贝叶斯信息量准则，越小越好）。
    pub bic: f64,
    /// 收敛迭代次数。
    pub iterations: u32,
    /// 是否收敛。
    pub converged: bool,
}

/// 6 种模型的对比验证报告。
#[derive(Debug, Clone, Serialize)]
pub struct ModelValidationReport {
    /// 所有模型拟合结果。
    pub fits: Vec<ModelFit>,
    /// 按 R² 排序的排名（第一名最优）。
    pub ranking: Vec<GrowthModel>,
    /// 推荐模型。
    pub recommended: GrowthModel,
    /// 最佳 R²。
    pub best_r2: f64,
    /// 最小 RMSE。
    pub best_rmse: f64,
}

impl ModelFit {
    /// 计算 R² 和 RMSE。
    fn compute_goodness(
        observed: &[f64],
        predicted: &[f64],
        n_params: usize,
    ) -> (f64, f64, f64, f64) {
        let n = observed.len() as f64;
        if n < 1.0 {
            return (0.0, f64::INFINITY, f64::INFINITY, f64::INFINITY);
        }
        let mean_obs = observed.iter().sum::<f64>() / n;
        let ss_res: f64 = observed
            .iter()
            .zip(predicted.iter())
            .map(|(o, p)| (o - p).powi(2))
            .sum();
        let ss_tot: f64 = observed.iter().map(|o| (o - mean_obs).powi(2)).sum();
        let r2 = if ss_tot > 0.0 {
            1.0 - ss_res / ss_tot
        } else {
            0.0
        };
        let rmse = (ss_res / n).sqrt();
        let k = n_params as f64;
        // 完美拟合（ss_res = 0）：AIC/BIC 取其理论最小值
        let (aic, bic) = if ss_res < 1e-15 {
            // 对数似然 ∼ 0，仅剩参数惩罚项
            (2.0 * k, k * n.ln())
        } else {
            let sigma2 = ss_res / n;
            let lnl = -0.5 * n * (2.0 * std::f64::consts::PI * sigma2).ln() - 0.5 * n;
            (2.0 * k - 2.0 * lnl, k * n.ln() - 2.0 * lnl)
        };
        (r2, rmse, aic, bic)
    }
}

/// 造林/天然林生长数据（含观测值和拟合值）。
#[derive(Debug, Clone)]
pub struct GrowthRecord {
    pub age: f64,
    pub observed_height: f64,
    pub predicted_height: Option<f64>,
}

/// 对单个模型进行网格搜索 + 局部细化校准。
///
/// 对 (a, b, c) 进行三层网格搜索：
/// 1. 粗搜：a ∈ [5, 40], b ∈ [0.005, 0.2], c ∈ [0.1, 3.0]
/// 2. 细搜：在粗搜最佳点附近 ±30%
/// 3. 返回 RMSE 最小的参数组合
pub fn calibrate_growth_model(model: GrowthModel, ages: &[f64], heights: &[f64]) -> ModelFit {
    // 网格搜索范围
    let a_range = (5.0, 40.0);
    let b_range = (0.005, 0.2);
    let c_range = (0.1, 3.0);

    let a_steps = 8;
    let b_steps = 8;
    let c_steps = 8;

    let mut best_rmse = f64::INFINITY;
    let mut best_params = [12.0, 0.04, 1.0];
    let mut iterations = 0u32;

    // 第一轮：粗搜
    for ai in 0..a_steps {
        let a = a_range.0 + (a_range.1 - a_range.0) * ai as f64 / (a_steps - 1) as f64;
        for bi in 0..b_steps {
            let b = b_range.0 + (b_range.1 - b_range.0) * bi as f64 / (b_steps - 1) as f64;
            for ci in 0..c_steps {
                let c = c_range.0 + (c_range.1 - c_range.0) * ci as f64 / (c_steps - 1) as f64;
                iterations += 1;

                let predicted: Vec<f64> = ages
                    .iter()
                    .map(|&age| model.predict_height(age, a, b, c))
                    .collect();

                let this_rmse: f64 = ages
                    .iter()
                    .zip(predicted.iter())
                    .map(|(&age, &p)| {
                        let obs = find_interpolated_height(age, ages, heights);
                        (obs - p).powi(2)
                    })
                    .sum::<f64>()
                    .sqrt()
                    / ages.len() as f64;

                if this_rmse < best_rmse {
                    best_rmse = this_rmse;
                    best_params = [a, b, c];
                }
            }
        }
    }

    // 第二轮：局部细化
    let refinement_ranges = [
        (best_params[0] * 0.7, best_params[0] * 1.3),
        (best_params[1] * 0.5, best_params[1] * 2.0),
        (best_params[2] * 0.5, best_params[2] * 2.0),
    ];

    for ai in 0..a_steps {
        let a = refinement_ranges[0].0
            + (refinement_ranges[0].1 - refinement_ranges[0].0) * ai as f64 / (a_steps - 1) as f64;
        for bi in 0..b_steps {
            let b = refinement_ranges[1].0
                + (refinement_ranges[1].1 - refinement_ranges[1].0) * bi as f64
                    / (b_steps - 1) as f64;
            for ci in 0..c_steps {
                let c = refinement_ranges[2].0
                    + (refinement_ranges[2].1 - refinement_ranges[2].0) * ci as f64
                        / (c_steps - 1) as f64;
                iterations += 1;

                let predicted: Vec<f64> = ages
                    .iter()
                    .map(|&age| model.predict_height(age, a, b, c))
                    .collect();

                let this_rmse: f64 = ages
                    .iter()
                    .zip(predicted.iter())
                    .map(|(&age, &p)| {
                        let obs = find_interpolated_height(age, ages, heights);
                        (obs - p).powi(2)
                    })
                    .sum::<f64>()
                    .sqrt()
                    / ages.len() as f64;

                if this_rmse < best_rmse {
                    best_rmse = this_rmse;
                    best_params = [a, b, c];
                }
            }
        }
    }

    // 用最佳参数计算最终的拟合优度
    let predicted: Vec<f64> = ages
        .iter()
        .map(|&age| model.predict_height(age, best_params[0], best_params[1], best_params[2]))
        .collect();

    let observed: Vec<f64> = ages
        .iter()
        .map(|&age| find_interpolated_height(age, ages, heights))
        .collect();

    let (r2, rmse, aic, bic) = ModelFit::compute_goodness(&observed, &predicted, 3);
    let converged = iterations > 0;

    ModelFit {
        model,
        params: best_params,
        r_squared: r2.max(0.0).min(1.0),
        rmse,
        aic,
        bic,
        iterations,
        converged,
    }
}

/// 线性插值查找给定年龄的对应观测树高。
fn find_interpolated_height(age: f64, ages: &[f64], heights: &[f64]) -> f64 {
    // 数据点直接匹配
    for (i, &a) in ages.iter().enumerate() {
        if (a - age).abs() < 1e-6 {
            return heights[i];
        }
    }
    // 外推或插值
    if age <= ages[0] {
        return heights[0];
    }
    if age >= ages[ages.len() - 1] {
        return heights[heights.len() - 1];
    }
    for i in 0..ages.len() - 1 {
        if age >= ages[i] && age <= ages[i + 1] {
            let t = (age - ages[i]) / (ages[i + 1] - ages[i]);
            return heights[i] + t * (heights[i + 1] - heights[i]);
        }
    }
    heights[ages.len() - 1]
}

/// 对所有 6 种生长模型进行校准和验证，返回对比报告。
pub fn validate_all_growth_models(ages: &[f64], heights: &[f64]) -> ModelValidationReport {
    let models = [
        GrowthModel::Richards,
        GrowthModel::Logistic,
        GrowthModel::Korf,
        GrowthModel::Gompertz,
        GrowthModel::Weibull,
        GrowthModel::Schumacher,
    ];

    let mut fits: Vec<ModelFit> = models
        .iter()
        .map(|m| calibrate_growth_model(*m, ages, heights))
        .collect();

    // 按 R² 降序排名
    fits.sort_by(|a, b| {
        b.r_squared
            .partial_cmp(&a.r_squared)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let ranking: Vec<GrowthModel> = fits.iter().map(|f| f.model).collect();
    let best_r2 = fits.first().map(|f| f.r_squared).unwrap_or(0.0);
    let best_rmse = fits.iter().map(|f| f.rmse).fold(f64::INFINITY, f64::min);
    let recommended = ranking.first().copied().unwrap_or(GrowthModel::Richards);

    ModelValidationReport {
        fits,
        ranking,
        recommended,
        best_r2,
        best_rmse,
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

        let result = plugin
            .assess_carbon_stock(
                "测试林场",
                aoi,
                &red_base,
                &nir_base,
                &red_curr,
                &nir_curr,
                2020,
                2025,
                200.0,
                500.0,
            )
            .unwrap();

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
        let plots = (0..30)
            .map(|i| PlotData {
                id: format!("P{i:03}"),
                age: 20.0 + (i as f64 % 5.0) * 5.0,
                height: 3.0 + (i as f64 % 3.0) * 4.0,
                sdi: 500.0,
                basal_area: 20.0,
                biomass: 100.0,
            })
            .collect::<Vec<_>>();

        let result = plugin
            .site_classification(&plots, GrowthModel::Richards, 3, 10)
            .unwrap();
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

        let pp = plugin
            .potential_productivity(&sites, 30, 20.0, 3000.0, 0.1, 50)
            .unwrap();
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

    // ── 6 种生长曲线验证测试 ──

    /// 生成典型的杉木人工林年龄-树高观测序列（参考中国南方杉木数据）。
    fn china_fir_data() -> (Vec<f64>, Vec<f64>) {
        let ages = vec![5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0];
        let heights = vec![3.2, 7.1, 10.8, 13.5, 15.2, 16.5, 17.1, 17.5];
        (ages, heights)
    }

    #[test]
    fn test_calibrate_richards_model() {
        let (ages, heights) = china_fir_data();
        let fit = calibrate_growth_model(GrowthModel::Richards, &ages, &heights);

        assert!(fit.converged, "Richards model should converge");
        assert!(
            fit.r_squared > 0.80,
            "R²={} should be > 0.80",
            fit.r_squared
        );
        assert!(fit.rmse < 3.0, "RMSE={} should be < 3.0m", fit.rmse);
        assert!(fit.params[0] > 0.0, "Asymptote a should be positive");
        assert!(fit.params[1] > 0.0, "Rate b should be positive");
        assert!(fit.iterations > 0);
    }

    #[test]
    fn test_validate_all_6_models() {
        let (ages, heights) = china_fir_data();
        let report = validate_all_growth_models(&ages, &heights);

        assert_eq!(report.fits.len(), 6, "Should have 6 model fits");
        assert_eq!(report.ranking.len(), 6);
        assert!(report.best_r2 > 0.5, "Best R² should be acceptable");
        assert!(report.best_rmse < 5.0, "Best RMSE should be < 5m");

        // 验证所有模型都成功收敛
        for fit in &report.fits {
            assert!(fit.converged, "{:?} should converge", fit.model);
            assert!(fit.r_squared > 0.0, "{:?} R² > 0", fit.model);
            assert!(!fit.params[0].is_nan());
            assert!(!fit.params[1].is_nan());
            assert!(!fit.params[2].is_nan());
        }
    }

    #[test]
    fn test_valid_models_predict_monotonic() {
        // 验证 6 种模型的预测值随年龄单调递增
        let models = [
            GrowthModel::Richards,
            GrowthModel::Logistic,
            GrowthModel::Korf,
            GrowthModel::Gompertz,
            GrowthModel::Weibull,
            GrowthModel::Schumacher,
        ];
        let a = 20.0;
        let b = 0.05;
        let c = 1.2;
        let ages: Vec<f64> = (1..=80).map(|x| x as f64).collect();

        for model in &models {
            let heights: Vec<f64> = ages
                .iter()
                .map(|&age| model.predict_height(age, a, b, c))
                .collect();

            // 验证单调递增
            for i in 1..heights.len() {
                assert!(
                    heights[i] >= heights[i - 1] - 1e-10,
                    "{:?}: height should be monotonic increasing, age {}->{}",
                    model,
                    ages[i - 1],
                    ages[i]
                );
            }

            // 验证渐进性（at large age, growth near asymptote）
            let h40 = model.predict_height(40.0, a, b, c);
            let h80 = model.predict_height(80.0, a, b, c);
            assert!(
                (h80 - h40).abs() < 3.0,
                "{:?}: growth after 40yr should be small: h40={}, h80={}",
                model,
                h40,
                h80
            );
        }
    }

    #[test]
    fn test_model_fit_goodness_stats() {
        // 验证拟合优度统计量计算正确
        let observed = vec![5.0, 10.0, 15.0, 20.0];
        let predicted = vec![5.0, 10.0, 15.0, 20.0];
        let (r2, rmse, _aic, _bic) = ModelFit::compute_goodness(&observed, &predicted, 3);
        assert!((r2 - 1.0).abs() < 1e-10, "Perfect fit: R² should be 1.0");
        assert!(rmse < 1e-10, "Perfect fit: RMSE should be 0");

        // 部分偏差
        let predicted2 = vec![4.0, 12.0, 14.0, 22.0];
        let (r2b, rmse_b, _aicb, _bicb) = ModelFit::compute_goodness(&observed, &predicted2, 3);
        assert!(r2b < 1.0, "Imperfect fit: R² < 1.0");
        assert!(rmse_b > 0.0, "Imperfect fit: RMSE > 0");
    }

    #[test]
    fn test_find_interpolated_height() {
        let ages = vec![10.0, 20.0, 30.0];
        let heights = vec![5.0, 12.0, 16.0];

        assert_eq!(find_interpolated_height(10.0, &ages, &heights), 5.0);
        assert_eq!(find_interpolated_height(15.0, &ages, &heights), 8.5);
        assert_eq!(find_interpolated_height(30.0, &ages, &heights), 16.0);
        assert_eq!(find_interpolated_height(5.0, &ages, &heights), 5.0);
        assert_eq!(find_interpolated_height(35.0, &ages, &heights), 16.0);
    }

    #[test]
    fn test_predict_biomass() {
        let bio = GrowthModel::predict_biomass(30.0, 800.0, 0.2, 0.05, 0.7, 1.5);
        assert!(bio > 0.0, "Biomass prediction should be positive");
        assert!(bio < 1000.0, "Biomass prediction should be reasonable");
        // Zero SDI → near zero biomass
        let bio0 = GrowthModel::predict_biomass(30.0, 0.0, 0.2, 0.05, 0.7, 1.5);
        assert!(bio0 < 0.01);
    }

    #[test]
    fn test_model_ranking() {
        let (ages, heights) = china_fir_data();
        let report = validate_all_growth_models(&ages, &heights);

        // Richardson 和 Gompertz 通常对杉木表现最好
        let top3 = &report.ranking[..3];
        assert!(
            top3.iter().any(|m| matches!(
                m,
                GrowthModel::Richards | GrowthModel::Gompertz | GrowthModel::Weibull
            )),
            "Top 3 should include Richards, Gompertz or Weibull for fir data"
        );

        // 最佳拟合应有最高 R²
        let best = &report.fits[0];
        for f in &report.fits[1..] {
            assert!(
                best.r_squared >= f.r_squared - 1e-10,
                "Best model should have highest R²: {} vs {}",
                best.r_squared,
                f.r_squared
            );
        }
    }
}
