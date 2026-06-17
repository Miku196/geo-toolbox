use crate::config::GeohazardConfig;
use geo_core::errors::{GeoError, GeoResult};
use serde::{Deserialize, Serialize};

/// 综合分析结果（滑坡 + 泥石流 + 综合评级）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeohazardAssessment {
    pub aoi_name: String,

    /// 滑坡敏感性。
    pub landslide: LandslideResult,
    /// 泥石流危险性（可选）。
    pub debris_flow: Option<DebrisFlowResult>,

    /// 综合风险等级。
    pub overall_risk: RiskLevel,

    /// 评估时间。
    pub calculated_at: String,
}

/// 滑坡敏感性分析结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandslideResult {
    /// 综合敏感性指数 [0,1]。
    pub susceptibility: f64,

    /// 风险等级。
    pub risk_level: RiskLevel,

    /// 各因子归一化值。
    pub factor_scores: FactorScores,
}

/// 各因子得分（归一化 [0,1]，越高越危险）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorScores {
    pub slope: f64,
    pub aspect: f64,
    pub lithology: f64,
    pub rainfall: f64,
    pub fault_distance: f64,
    pub vegetation: f64,
}

/// 泥石流危险性结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebrisFlowResult {
    /// 危险性指数 [0,1]。
    pub hazard: f64,

    /// 风险等级。
    pub risk_level: RiskLevel,

    /// 沟床比降得分。
    pub gradient_score: f64,
    /// 松散物源量得分。
    pub material_score: f64,
    /// 降雨触发得分。
    pub rainfall_score: f64,
}

/// 泥石流体积-冲出距离经验模型结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebrisFlowRunout {
    /// 估算总物质体积（m³）。
    pub volume_m3: f64,
    /// 预测冲出距离（m）。
    pub runout_distance_m: f64,
    /// 视摩擦角 / 等效摩擦系数（°）。
    pub travel_angle_deg: f64,
    /// 估算冲击压力（kPa）。
    pub impact_pressure_kpa: f64,
    /// 影响扇面积（m²）。
    pub affected_area_m2: f64,
    /// 风险等级。
    pub risk_level: RiskLevel,
}

/// 风险等级：5级。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    /// ≤ 0.15
    VeryLow,
    /// 0.15–0.35
    Low,
    /// 0.35–0.55
    Moderate,
    /// 0.55–0.75
    High,
    /// > 0.75
    VeryHigh,
}

impl RiskLevel {
    pub fn from_score(s: f64) -> Self {
        if s <= 0.15 {
            RiskLevel::VeryLow
        } else if s <= 0.35 {
            RiskLevel::Low
        } else if s <= 0.55 {
            RiskLevel::Moderate
        } else if s <= 0.75 {
            RiskLevel::High
        } else {
            RiskLevel::VeryHigh
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::VeryLow => "极低",
            RiskLevel::Low => "低",
            RiskLevel::Moderate => "中",
            RiskLevel::High => "高",
            RiskLevel::VeryHigh => "极高",
        }
    }

    pub fn as_english(&self) -> &'static str {
        match self {
            RiskLevel::VeryLow => "very_low",
            RiskLevel::Low => "low",
            RiskLevel::Moderate => "moderate",
            RiskLevel::High => "high",
            RiskLevel::VeryHigh => "very_high",
        }
    }
}

/// 地质灾害插件。
pub struct GeohazardPlugin {
    config: GeohazardConfig,
}

impl GeohazardPlugin {
    pub fn new(config: GeohazardConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &GeohazardConfig {
        &self.config
    }

    // ── 模糊隶属度函数 ──

    /// S 型增长隶属度函数（低→高危险）。
    /// 在 [a, b] 区间内曲线上升。
    fn fuzzy_s(x: f64, a: f64, b: f64) -> f64 {
        if x <= a {
            0.0
        } else if x >= b {
            1.0
        } else {
            // 余弦过渡，平滑
            let t = (x - a) / (b - a);
            t * t * (3.0 - 2.0 * t) // smoothstep
        }
    }

    /// L 型下降隶属度函数（高→低危险）。
    /// x 越小越危险。
    fn fuzzy_l(x: f64, a: f64, b: f64) -> f64 {
        if x >= b {
            0.0
        } else if x <= a {
            1.0
        } else {
            let t = (x - a) / (b - a);
            1.0 - t * t * (3.0 - 2.0 * t) // inverse smoothstep
        }
    }

    // ── 各因子归一化 ──

    /// 坡度因子归一化（度）。越陡越危险。
    pub fn normalize_slope(&self, slope_deg: f64) -> f64 {
        let p = &self.config.landslide;
        Self::fuzzy_s(slope_deg, p.slope_a, p.slope_b)
    }

    /// 坡向因子（朝南性）。北半球朝南（180°±45°）更易滑动。
    /// aspect_deg: 0–360°，北=0°，东=90°，南=180°，西=270°。
    pub fn normalize_aspect(&self, aspect_deg: f64) -> f64 {
        // 朝南程度的指标：cos(aspect - 180°) 映射到 [0,1]
        // 0°(N) = 0, 180°(S) = 1, 90°(E)/270°(W) = 0.5
        let rad = aspect_deg.to_radians();
        let southness = (rad - std::f64::consts::PI).cos();
        // 从 [-1,1] 映射到 [0,1]，并加一点阈值
        let val = (southness + 1.0) * 0.5;
        val.clamp(0.0, 1.0)
    }

    /// 岩性归一化。传入预分类 [0,1]：0=基岩, 0.33=半成岩, 0.67=松散堆积, 1.0=极软岩。
    pub fn normalize_lithology(&self, lithology_index: f64) -> f64 {
        lithology_index.clamp(0.0, 1.0)
    }

    /// 日降雨量归一化（mm/24h）。
    pub fn normalize_rainfall(&self, rainfall_mm: f64) -> f64 {
        let p = &self.config.landslide;
        Self::fuzzy_s(rainfall_mm, p.rainfall_a, p.rainfall_b)
    }

    /// 距断层距离归一化（m）。越近越危险。
    pub fn normalize_fault_distance(&self, distance_m: f64) -> f64 {
        let p = &self.config.landslide;
        Self::fuzzy_l(distance_m, p.fault_a, p.fault_b)
    }

    /// 植被覆盖归一化（NDVI）。植被越多越安全。
    pub fn normalize_vegetation(&self, ndvi: f64) -> f64 {
        let p = &self.config.landslide;
        Self::fuzzy_l(ndvi, p.veg_a, p.veg_b)
    }

    // ── 综合计算 ──

    /// 6因子滑坡敏感性综合指数。
    pub fn landslide_susceptibility(
        &self,
        slope_deg: f64,
        aspect_deg: f64,
        lithology_index: f64,
        rainfall_mm: f64,
        fault_distance_m: f64,
        ndvi: f64,
    ) -> LandslideResult {
        let w = &self.config.landslide;

        let s_slope = self.normalize_slope(slope_deg);
        let s_aspect = self.normalize_aspect(aspect_deg);
        let s_lithology = self.normalize_lithology(lithology_index);
        let s_rainfall = self.normalize_rainfall(rainfall_mm);
        let s_fault = self.normalize_fault_distance(fault_distance_m);
        let s_veg = self.normalize_vegetation(ndvi);

        // 加权综合
        let total = s_slope * w.slope_weight
            + s_aspect * w.aspect_weight
            + s_lithology * w.lithology_weight
            + s_rainfall * w.rainfall_weight
            + s_fault * w.fault_weight
            + s_veg * w.vegetation_weight;

        let susceptibility = total.clamp(0.0, 1.0);

        LandslideResult {
            susceptibility,
            risk_level: RiskLevel::from_score(susceptibility),
            factor_scores: FactorScores {
                slope: s_slope,
                aspect: s_aspect,
                lithology: s_lithology,
                rainfall: s_rainfall,
                fault_distance: s_fault,
                vegetation: s_veg,
            },
        }
    }

    /// 泥石流危险性评估。
    ///
    /// # Arguments
    /// * `channel_gradient_deg` - 沟床比降（°）
    /// * `material_volume_per_km` - 松散物源量（m³/km）
    /// * `rainfall_24h_mm` - 24小时降雨量（mm）
    pub fn debris_flow_hazard(
        &self,
        channel_gradient_deg: f64,
        material_volume_per_km: f64,
        rainfall_24h_mm: f64,
    ) -> DebrisFlowResult {
        let dp = &self.config.debris_flow;

        let gradient_score = Self::fuzzy_s(
            channel_gradient_deg,
            dp.channel_gradient_threshold,
            dp.channel_gradient_max,
        );
        let material_score = Self::fuzzy_s(
            material_volume_per_km,
            dp.material_threshold,
            dp.material_max,
        );
        let rainfall_score = Self::fuzzy_s(
            rainfall_24h_mm,
            dp.rainfall_trigger,
            dp.rainfall_trigger * 3.0,
        );

        // 加权平均：沟床条件 0.45 + 物源 0.35 + 降雨触发 0.20
        let hazard =
            (gradient_score * 0.45 + material_score * 0.35 + rainfall_score * 0.20).clamp(0.0, 1.0);

        DebrisFlowResult {
            hazard,
            risk_level: RiskLevel::from_score(hazard),
            gradient_score,
            material_score,
            rainfall_score,
        }
    }

    /// 综合地质灾害风险评估（滑坡+泥石流取最大）。
    pub fn overall_assessment(
        &self,
        landside: LandslideResult,
        debris: Option<DebrisFlowResult>,
        aoi_name: &str,
    ) -> GeohazardAssessment {
        let combined = match &debris {
            Some(d) => landside.susceptibility.max(d.hazard),
            None => landside.susceptibility,
        };

        GeohazardAssessment {
            aoi_name: aoi_name.to_string(),
            landslide: landside,
            debris_flow: debris,
            overall_risk: RiskLevel::from_score(combined),
            calculated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// 估算滑坡体体积（简化方法）。
    /// 基于滑动面积和平均深度。
    ///
    /// # Arguments
    /// * `slide_area_m2` - 滑动面面积（m²）
    /// * `average_depth_m` - 平均滑动深度（m）
    /// * `bulk_density` - 物质容重（t/m³，默认 2.0）
    pub fn estimate_volume(
        &self,
        slide_area_m2: f64,
        average_depth_m: f64,
        bulk_density: Option<f64>,
    ) -> GeoResult<f64> {
        let density = bulk_density.unwrap_or(2.0);
        if slide_area_m2 <= 0.0 {
            return Err(GeoError::Validation("slide area must be positive".into()));
        }
        if average_depth_m <= 0.0 {
            return Err(GeoError::Validation("depth must be positive".into()));
        }
        if density <= 0.0 {
            return Err(GeoError::Validation("density must be positive".into()));
        }
        Ok(slide_area_m2 * average_depth_m * density)
    }

    /// 无限边坡安全系数 (Factor of Safety, FS)。
    ///
    /// FS = (c + (γ - m*γ_w) * z * cos²β * tanφ) / (γ * z * sinβ * cosβ)
    ///
    /// FS > 1: 稳定, FS = 1: 临界, FS < 1: 不稳定。
    pub fn factor_of_safety(
        &self,
        slope_deg: f64,          // 坡度 (°)
        soil_depth_m: f64,       // 土层厚度 (m)
        cohesion_kpa: f64,       // 粘聚力 (kPa)
        friction_deg: f64,       // 内摩擦角 (°)
        soil_density_kn_m3: f64, // 土体重度 (kN/m³) — 默认 20
        water_table_ratio: f64,  // 地下水位/土层厚度 (0~1)
    ) -> f64 {
        let beta = slope_deg.to_radians();
        let phi = friction_deg.to_radians();
        let gamma = soil_density_kn_m3.max(1.0);
        let gamma_w = 9.81; // 水的重度
        let m = water_table_ratio.clamp(0.0, 1.0);

        let sin_b = beta.sin();
        let cos_b = beta.cos();
        let tan_phi = phi.tan();

        let normal_stress = gamma * soil_depth_m * cos_b.powi(2);
        let shear_stress = gamma * soil_depth_m * sin_b * cos_b;
        let pore_pressure = m * gamma_w * soil_depth_m * cos_b.powi(2);

        if shear_stress < 1e-10 {
            return 99.0; // 近乎平坦，非常稳定
        }

        (cohesion_kpa + (normal_stress - pore_pressure) * tan_phi) / shear_stress
    }

    /// 估算泥石流物质体积（经验模型）。
    ///
    /// V = 0.5 × 流域面积(km²) × 降雨量(mm) × 物质因子 × volume_factor
    ///
    /// # Arguments
    /// * `watershed_area_km2` - 流域面积（km²）
    /// * `rainfall_24h_mm` - 24小时降雨量（mm）
    pub fn debris_flow_volume_empirical(
        &self,
        watershed_area_km2: f64,
        rainfall_24h_mm: f64,
    ) -> GeoResult<f64> {
        let dp = &self.config.debris_flow;
        if watershed_area_km2 <= 0.0 {
            return Err(GeoError::Validation(
                "watershed area must be positive".into(),
            ));
        }
        if rainfall_24h_mm < 0.0 {
            return Err(GeoError::Validation("rainfall cannot be negative".into()));
        }
        if dp.material_factor <= 0.0 {
            return Err(GeoError::Validation(
                "material factor must be positive".into(),
            ));
        }
        let volume =
            dp.volume_factor * 0.5 * watershed_area_km2 * rainfall_24h_mm * dp.material_factor;
        Ok(volume)
    }

    /// 估算泥石流冲出距离（基于视摩擦角 / 等效摩擦系数）。
    ///
    /// Takahashi (1991): α ≈ 18 – 6·log₁₀(V / 10⁴)
    /// L = H / tan(α)
    ///
    /// # Arguments
    /// * `volume_m3` - 泥石流物质体积（m³）
    /// * `elevation_drop_m` - 高差（m）
    pub fn debris_flow_runout_distance(&self, volume_m3: f64, elevation_drop_m: f64) -> f64 {
        if volume_m3 <= 0.0 || elevation_drop_m <= 0.0 {
            return 0.0;
        }
        let dp = &self.config.debris_flow;
        let travel_angle_deg = (18.0 - 6.0 * (volume_m3 / 10000.0).log10())
            .max(dp.min_travel_angle_deg)
            .min(40.0);
        let travel_angle_rad = travel_angle_deg.to_radians();
        let runout = elevation_drop_m / travel_angle_rad.tan();
        if runout.is_finite() {
            runout
        } else {
            0.0
        }
    }

    /// 估算泥石流冲击压力（kPa）。
    ///
    /// v ≈ √(2·g·H·sinθ)  流速（m/s）
    /// P = 0.5·ρ·v²       动压（Pa → kPa）
    ///
    /// # Arguments
    /// * `elevation_drop_m` - 高差（m）
    /// * `channel_gradient_deg` - 沟床比降（°）
    pub fn debris_flow_impact_pressure(
        &self,
        elevation_drop_m: f64,
        channel_gradient_deg: f64,
    ) -> f64 {
        if elevation_drop_m <= 0.0 || channel_gradient_deg <= 0.0 {
            return 0.0;
        }
        let dp = &self.config.debris_flow;
        let g = 9.81;
        let density_kg_m3 = dp.debris_density * 1000.0; // t/m³ → kg/m³
        let theta = channel_gradient_deg.to_radians();
        let velocity = (2.0 * g * elevation_drop_m * theta.sin()).sqrt();
        // dynamic pressure P = 0.5·ρ·v² (Pa) → kPa
        0.5 * density_kg_m3 * velocity * velocity / 1000.0
    }

    /// 估算泥石流影响扇面积（m²）。
    ///
    /// A_fan = 20 · V^0.83  (Iverson et al. 1998)
    ///
    /// # Arguments
    /// * `volume_m3` - 泥石流物质体积（m³）
    pub fn debris_flow_affected_area(&self, volume_m3: f64) -> f64 {
        if volume_m3 <= 0.0 {
            return 0.0;
        }
        20.0 * volume_m3.powf(0.83)
    }

    /// 综合泥石流体积-冲出距离评估。
    ///
    /// 组合体积估算、冲出距离、冲击压力、影响扇面积。
    ///
    /// # Arguments
    /// * `watershed_area_km2` - 流域面积（km²）
    /// * `rainfall_24h_mm` - 24小时降雨量（mm）
    /// * `elevation_drop_m` - 高差（m）
    /// * `channel_gradient_deg` - 沟床比降（°）
    pub fn debris_flow_runout_assessment(
        &self,
        watershed_area_km2: f64,
        rainfall_24h_mm: f64,
        elevation_drop_m: f64,
        channel_gradient_deg: f64,
    ) -> GeoResult<DebrisFlowRunout> {
        let volume = self.debris_flow_volume_empirical(watershed_area_km2, rainfall_24h_mm)?;
        let runout = self.debris_flow_runout_distance(volume, elevation_drop_m);
        let travel_angle = if runout > 0.01 {
            (elevation_drop_m / runout).atan().to_degrees()
        } else {
            0.0
        };
        let impact = self.debris_flow_impact_pressure(elevation_drop_m, channel_gradient_deg);
        let area = self.debris_flow_affected_area(volume);

        let risk = if runout > 1000.0 {
            RiskLevel::VeryHigh
        } else if runout > 500.0 {
            RiskLevel::High
        } else if runout > 200.0 {
            RiskLevel::Moderate
        } else if runout > 50.0 {
            RiskLevel::Low
        } else {
            RiskLevel::VeryLow
        };

        Ok(DebrisFlowRunout {
            volume_m3: volume,
            runout_distance_m: runout,
            travel_angle_deg: travel_angle,
            impact_pressure_kpa: impact,
            affected_area_m2: area,
            risk_level: risk,
        })
    }

    /// Newmark 永久位移（地震滑坡）。
    ///
    /// 简化的 Newmark 滑动块模型：临界加速度 ac = (FS - 1) * g * sin(β)
    /// 永久位移 D_N ≈ 取决于 PGA/ac 比值的经验公式。
    ///
    /// 返回位移 (cm)。
    pub fn newmark_displacement(
        &self,
        slope_deg: f64,
        factor_of_safety: f64,
        pga_g: f64, // 峰值地面加速度 (g)
    ) -> f64 {
        if factor_of_safety >= 99.0 || slope_deg < 0.1 {
            return 0.0;
        }
        let beta = slope_deg.to_radians();
        let ac = (factor_of_safety - 1.0).max(0.0) * 9.81 * beta.sin(); // critical accel (m/s²)
        let pga = pga_g * 9.81;

        if ac < 1e-6 || pga <= ac {
            return 0.0;
        }

        // Jibson (2007) 简化经验公式: log D_N = 0.215 + log[(1 - ac/PGA)^2.341 * (ac/PGA)^-1.438]
        let ratio = ac / pga;
        let log_dn = 0.215 + ((1.0 - ratio).powf(2.341) * ratio.powf(-1.438)).ln();
        (10.0_f64.powf(log_dn) * 100.0).min(1000.0) // cm, cap at 10m
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GeohazardConfig;

    fn make_plugin() -> GeohazardPlugin {
        let config: GeohazardConfig = toml::from_str(
            "[plugin]\nname = \"geohazard\"\nversion = \"0.1\"\ndescription = \"\"\n",
        )
        .unwrap();
        GeohazardPlugin::new(config)
    }

    #[test]
    fn test_fuzzy_s() {
        assert_eq!(GeohazardPlugin::fuzzy_s(0.0, 10.0, 30.0), 0.0);
        assert_eq!(GeohazardPlugin::fuzzy_s(40.0, 10.0, 30.0), 1.0);
        let mid = GeohazardPlugin::fuzzy_s(20.0, 10.0, 30.0);
        assert!(mid > 0.0 && mid < 1.0, "mid={mid} should be in (0,1)");
        // smoothstep at t=0.5
        assert!((mid - 0.5).abs() < 0.01, "mid={mid} should be ~0.5");
    }

    #[test]
    fn test_fuzzy_l() {
        assert_eq!(GeohazardPlugin::fuzzy_l(5.0, 10.0, 30.0), 1.0);
        assert_eq!(GeohazardPlugin::fuzzy_l(35.0, 10.0, 30.0), 0.0);
        let mid = GeohazardPlugin::fuzzy_l(20.0, 10.0, 30.0);
        assert!(mid > 0.0 && mid < 1.0);
        assert!((mid - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_normalize_aspect() {
        let p = make_plugin();
        // 朝南=180°→1.0
        let south = p.normalize_aspect(180.0);
        assert!((south - 1.0).abs() < 0.01, "south={south}");
        // 朝北=0°→~0.0
        let north = p.normalize_aspect(0.0);
        assert!(north < 0.01, "north={north}");
        // 朝东=90°→~0.5
        let east = p.normalize_aspect(90.0);
        assert!((east - 0.5).abs() < 0.05, "east={east}");
    }

    #[test]
    fn test_normalize_slope() {
        let p = make_plugin();
        assert_eq!(p.normalize_slope(5.0), 0.0); // 平坦
        assert!(p.normalize_slope(25.0) > 0.0 && p.normalize_slope(25.0) < 1.0);
        assert_eq!(p.normalize_slope(50.0), 1.0); // 极陡
    }

    #[test]
    fn test_landslide_susceptibility_6_factor() {
        let p = make_plugin();
        // 极端危险场景
        let result = p.landslide_susceptibility(40.0, 180.0, 1.0, 400.0, 10.0, 0.05);
        assert!(
            result.susceptibility > 0.8,
            "should be very high: {}",
            result.susceptibility
        );
        assert_eq!(result.risk_level, RiskLevel::VeryHigh);

        // 极端安全场景
        let safe = p.landslide_susceptibility(5.0, 0.0, 0.0, 20.0, 2000.0, 0.8);
        assert!(
            safe.susceptibility < 0.2,
            "should be very low: {}",
            safe.susceptibility
        );
        assert_eq!(safe.risk_level, RiskLevel::VeryLow);
    }

    #[test]
    fn test_debris_flow_hazard() {
        let p = make_plugin();
        // 极端危险
        let d = p.debris_flow_hazard(40.0, 20000.0, 200.0);
        assert!(
            d.hazard > 0.8,
            "debris flow hazard should be high: {}",
            d.hazard
        );
        assert_eq!(d.risk_level, RiskLevel::VeryHigh);

        // 安全
        let safe = p.debris_flow_hazard(5.0, 100.0, 10.0);
        assert!(safe.hazard < 0.2, "should be low: {}", safe.hazard);
    }

    #[test]
    fn test_overall_assessment() {
        let p = make_plugin();
        let ls = p.landslide_susceptibility(10.0, 90.0, 0.3, 50.0, 500.0, 0.5);
        let df = p.debris_flow_hazard(10.0, 500.0, 30.0);
        let overall = p.overall_assessment(ls, Some(df), "test");
        assert!(overall.overall_risk.as_str() == "低" || overall.overall_risk.as_str() == "中");
    }

    #[test]
    fn test_estimate_volume() {
        let p = make_plugin();
        let vol = p.estimate_volume(1000.0, 5.0, Some(2.0)).unwrap();
        assert!((vol - 10000.0).abs() < 0.01, "vol={vol}");
    }

    #[test]
    fn test_estimate_volume_invalid() {
        let p = make_plugin();
        assert!(p.estimate_volume(0.0, 5.0, None).is_err());
        assert!(p.estimate_volume(100.0, 0.0, None).is_err());
    }

    #[test]
    fn test_risk_level_from_score() {
        assert_eq!(RiskLevel::from_score(0.0), RiskLevel::VeryLow);
        assert_eq!(RiskLevel::from_score(0.2), RiskLevel::Low);
        assert_eq!(RiskLevel::from_score(0.45), RiskLevel::Moderate);
        assert_eq!(RiskLevel::from_score(0.65), RiskLevel::High);
        assert_eq!(RiskLevel::from_score(0.9), RiskLevel::VeryHigh);
    }

    #[test]
    fn test_risk_level_str() {
        let r = RiskLevel::VeryHigh;
        assert_eq!(r.as_str(), "极高");
        assert_eq!(r.as_english(), "very_high");
    }

    #[test]
    fn test_factor_scores_all_present() {
        let p = make_plugin();
        let result = p.landslide_susceptibility(25.0, 180.0, 0.5, 200.0, 500.0, 0.3);
        // 检查6个因子都有值
        assert!(result.factor_scores.slope > 0.0);
        assert!(result.factor_scores.aspect > 0.0);
        assert!(result.factor_scores.lithology >= 0.0);
        assert!(result.factor_scores.rainfall > 0.0);
        assert!(result.factor_scores.fault_distance > 0.0);
        assert!(result.factor_scores.vegetation > 0.0);
    }

    // ——————————— DebrisFlowRunout tests ———————————

    #[test]
    fn test_debris_flow_volume_empirical() {
        let p = make_plugin();
        // watershed=2 km², rainfall=100mm → V = 1.0 * 0.5 * 2.0 * 100.0 * 1.0 = 100
        let vol = p.debris_flow_volume_empirical(2.0, 100.0).unwrap();
        assert!((vol - 100.0).abs() < 0.01, "vol={vol} expected ~100");
    }

    #[test]
    fn test_debris_flow_volume_empirical_invalid() {
        let p = make_plugin();
        assert!(p.debris_flow_volume_empirical(0.0, 100.0).is_err());
        assert!(p.debris_flow_volume_empirical(-1.0, 100.0).is_err());
        assert!(p.debris_flow_volume_empirical(1.0, -10.0).is_err());
    }

    #[test]
    fn test_debris_flow_runout_distance() {
        let p = make_plugin();
        // V=10000 m³, H=100 m → α=18°, L=100/tan(18°)≈307.8m
        let runout = p.debris_flow_runout_distance(10000.0, 100.0);
        assert!(
            runout > 300.0 && runout < 320.0,
            "runout={runout} expected ~308m"
        );
        assert_eq!(p.debris_flow_runout_distance(0.0, 100.0), 0.0);
    }

    #[test]
    fn test_debris_flow_impact_pressure() {
        let p = make_plugin();
        // H=50m, gradient=20° → P ~ (0.5*2000*334)/1000 ≈ 334 kPa
        let pressure = p.debris_flow_impact_pressure(50.0, 20.0);
        assert!(
            pressure > 200.0 && pressure < 500.0,
            "pressure={pressure} kPa expected ~334"
        );
        assert_eq!(p.debris_flow_impact_pressure(0.0, 20.0), 0.0);
    }

    #[test]
    fn test_debris_flow_affected_area() {
        let p = make_plugin();
        // V=10000 m³ → A_fan=20*10000^0.83≈20*1949≈38978 m²
        let area = p.debris_flow_affected_area(10000.0);
        assert!(
            area > 30000.0 && area < 50000.0,
            "area={area} m² expected ~38978"
        );
        assert_eq!(p.debris_flow_affected_area(0.0), 0.0);
    }

    #[test]
    fn test_debris_flow_runout_assessment_full() {
        let p = make_plugin();
        let assessment = p
            .debris_flow_runout_assessment(3.0, 120.0, 200.0, 25.0)
            .unwrap();
        // V = 1.0 * 0.5 * 3.0 * 120.0 * 1.0 = 180 m³
        assert!(
            (assessment.volume_m3 - 180.0).abs() < 1.0,
            "vol={}",
            assessment.volume_m3
        );
        assert!(assessment.runout_distance_m > 0.0);
        assert!(assessment.travel_angle_deg > 0.0);
        assert!(assessment.impact_pressure_kpa > 0.0);
        assert!(assessment.affected_area_m2 > 0.0);
        assert!(assessment.risk_level.as_english().len() > 0);
    }
}
