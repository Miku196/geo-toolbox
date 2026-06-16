//! 降雨强度-历时曲线 (Intensity-Duration threshold)
//!
//! ID 曲线: I = α × D^(-β)
//! 其中 I = 降雨强度 (mm/h), D = 历时 (h)
//!
//! 用于滑坡降雨阈值的经验模型。
//! 参考: Guzzetti et al. (2008), Caine (1980)

use std::fmt;

/// 降雨 ID 曲线参数。
///
/// I = α × D^(-β)
#[derive(Debug, Clone, Copy)]
pub struct IdCurve {
    /// 乘数 α (mm/h^β)。
    pub alpha: f64,
    /// 指数 β。
    pub beta: f64,
}

impl IdCurve {
    /// 新建 ID 曲线。
    pub fn new(alpha: f64, beta: f64) -> Self {
        Self { alpha, beta }
    }

    /// 对数线性回归拟合 ID 曲线参数。
    ///
    /// 输入为 (强度 mm/h, 历时 h) 的观测对。
    /// 对 ln I = ln α - β ln D 做最小二乘拟合。
    pub fn fit(intensities: &[f64], durations: &[f64]) -> Self {
        let n = intensities.len().min(durations.len());
        if n < 2 {
            return Self {
                alpha: 10.0,
                beta: 0.5,
            };
        }

        let mut sum_x = 0.0_f64;
        let mut sum_y = 0.0_f64;
        let mut sum_xx = 0.0_f64;
        let mut sum_xy = 0.0_f64;

        for i in 0..n {
            let d = durations[i].max(1e-10);
            let ii = intensities[i].max(1e-10);
            let x = d.ln();
            let y = ii.ln();
            sum_x += x;
            sum_y += y;
            sum_xx += x * x;
            sum_xy += x * y;
        }

        let n_f = n as f64;
        let denom = n_f * sum_xx - sum_x * sum_x;
        if denom.abs() < 1e-12 {
            return Self {
                alpha: 10.0,
                beta: 0.5,
            };
        }

        let neg_beta = (n_f * sum_xy - sum_x * sum_y) / denom;
        let ln_alpha = (sum_y - neg_beta * sum_x) / n_f;

        let alpha = ln_alpha.exp();
        let beta = -neg_beta;

        // Clamp to reasonable ranges
        Self {
            alpha: alpha.clamp(1.0, 500.0),
            beta: beta.clamp(0.1, 1.5),
        }
    }

    /// 计算给定历时的降雨强度。
    ///
    /// I = α × D^(-β)
    pub fn intensity(&self, duration_hours: f64) -> f64 {
        let d = duration_hours.max(0.01);
        self.alpha * d.powf(-self.beta)
    }

    /// 计算给定历时的累计降雨量。
    ///
    /// R = I × D = α × D^(1-β)
    pub fn cumulative_rainfall(&self, duration_hours: f64) -> f64 {
        let d = duration_hours.max(0.01);
        self.alpha * d.powf(1.0 - self.beta)
    }

    /// 判断观测降雨是否超过滑坡阈值。
    ///
    /// 若 I_obs > I_threshold(D) 则返回 true（滑坡风险升高中）。
    pub fn is_landslide_trigger(
        &self,
        observed_intensity_mmh: f64,
        duration_hours: f64,
    ) -> bool {
        let threshold_i = self.intensity(duration_hours);
        observed_intensity_mmh > threshold_i
    }

    /// 计算返回周期 (Return Period) 的降雨强度。
    ///
    /// 使用 Sherman 公式: I_T = c × T^d / D^β
    /// 其中 T = 返回周期 (年), c, d = 气候参数。
    /// 默认参数适用于中国亚热带季风区。
    pub fn for_return_period(&self, return_period_years: f64, c: f64, d: f64) -> IdCurve {
        let alpha = c * return_period_years.powf(d);
        IdCurve {
            alpha,
            beta: self.beta,
        }
    }

    // ── 全球经验阈值 ─────────────────────────────────────────

    /// Caine (1980) 全球滑坡降雨阈值。
    /// I = 14.82 × D^(-0.39)
    pub fn caine_1980() -> Self {
        Self {
            alpha: 14.82,
            beta: 0.39,
        }
    }

    /// Guzzetti et al. (2008) 全球阈值。
    /// I = 2.20 × D^(-0.44)
    pub fn guzzetti_2008() -> Self {
        Self {
            alpha: 2.20,
            beta: 0.44,
        }
    }

    /// 中国西南地区阈值 (基于文献综合)。
    /// I = 32.5 × D^(-0.52)
    pub fn china_southwest() -> Self {
        Self {
            alpha: 32.5,
            beta: 0.52,
        }
    }

    /// 中国东南沿海台风区阈值。
    /// I = 49.0 × D^(-0.48)
    pub fn china_southeast_typhoon() -> Self {
        Self {
            alpha: 49.0,
            beta: 0.48,
        }
    }
}

/// 降雨等级（中国气象局标准）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RainfallClass {
    /// 小雨: < 2.5 mm/h
    Light,
    /// 中雨: 2.5–8 mm/h
    Moderate,
    /// 大雨: 8–16 mm/h
    Heavy,
    /// 暴雨: 16–32 mm/h
    Rainstorm,
    /// 大暴雨: 32–64 mm/h
    Downpour,
    /// 特大暴雨: ≥ 64 mm/h
    Extreme,
}

impl RainfallClass {
    /// 根据降雨强度 (mm/h) 分类。
    pub fn classify(intensity_mmh: f64) -> Self {
        if intensity_mmh < 2.5 {
            RainfallClass::Light
        } else if intensity_mmh < 8.0 {
            RainfallClass::Moderate
        } else if intensity_mmh < 16.0 {
            RainfallClass::Heavy
        } else if intensity_mmh < 32.0 {
            RainfallClass::Rainstorm
        } else if intensity_mmh < 64.0 {
            RainfallClass::Downpour
        } else {
            RainfallClass::Extreme
        }
    }

    /// 危险性权重（用于滑坡触发权重计算）。
    pub fn hazard_weight(&self) -> f64 {
        match self {
            RainfallClass::Light => 0.05,
            RainfallClass::Moderate => 0.15,
            RainfallClass::Heavy => 0.35,
            RainfallClass::Rainstorm => 0.55,
            RainfallClass::Downpour => 0.75,
            RainfallClass::Extreme => 0.95,
        }
    }
}

impl fmt::Display for RainfallClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            RainfallClass::Light => "小雨",
            RainfallClass::Moderate => "中雨",
            RainfallClass::Heavy => "大雨",
            RainfallClass::Rainstorm => "暴雨",
            RainfallClass::Downpour => "大暴雨",
            RainfallClass::Extreme => "特大暴雨",
        };
        write!(f, "{s}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_curve_intensity() {
        // α=50, β=0.5: at D=1h, I=50; at D=4h, I=50/√4=25
        let curve = IdCurve::new(50.0, 0.5);
        let i1 = curve.intensity(1.0);
        let i4 = curve.intensity(4.0);

        assert!((i1 - 50.0).abs() < 0.5);
        assert!((i4 - 25.0).abs() < 0.5);
    }

    #[test]
    fn test_id_curve_fit() {
        // Generate data from I = 80 × D^(-0.4) with exact values
        let durations = vec![1.0, 2.0, 4.0, 8.0, 12.0, 24.0];
        let intensities: Vec<f64> = durations.iter().map(|&d| 80.0 * d.powf(-0.4)).collect();

        let curve = IdCurve::fit(&intensities, &durations);
        // Fitted parameters should be close to original
        assert!((curve.alpha - 80.0).abs() < 5.0, "alpha={}", curve.alpha);
        assert!((curve.beta - 0.4).abs() < 0.05, "beta={}", curve.beta);
    }

    #[test]
    fn test_id_curve_fit_with_noise() {
        // Realistic: add ±5% noise
        let durations: Vec<f64> = (1..=12).map(|x| x as f64).collect();
        let intensities: Vec<f64> = durations
            .iter()
            .enumerate()
            .map(|(i, &d)| {
                let base = 60.0 * d.powf(-0.55);
                let noise = 1.0 + (i as f64 % 3.0 - 1.0) * 0.08;
                base * noise
            })
            .collect();

        let curve = IdCurve::fit(&intensities, &durations);
        // Should be in reasonable range
        assert!(curve.alpha > 20.0 && curve.alpha < 150.0);
        assert!(curve.beta > 0.1 && curve.beta < 1.0);
    }

    #[test]
    fn test_id_curve_min_input() {
        // Single data point: returns defaults
        let curve = IdCurve::fit(&[30.0], &[2.0]);
        assert_eq!(curve.alpha, 10.0);
        assert_eq!(curve.beta, 0.5);
    }

    #[test]
    fn test_rainfall_classify() {
        assert_eq!(RainfallClass::classify(1.0), RainfallClass::Light);
        assert_eq!(RainfallClass::classify(5.0), RainfallClass::Moderate);
        assert_eq!(RainfallClass::classify(10.0), RainfallClass::Heavy);
        assert_eq!(RainfallClass::classify(20.0), RainfallClass::Rainstorm);
        assert_eq!(RainfallClass::classify(40.0), RainfallClass::Downpour);
        assert_eq!(RainfallClass::classify(80.0), RainfallClass::Extreme);
    }

    #[test]
    fn test_rainfall_classify_boundaries() {
        assert_eq!(RainfallClass::classify(2.5), RainfallClass::Moderate);
        assert_eq!(RainfallClass::classify(2.499), RainfallClass::Light);
        assert_eq!(RainfallClass::classify(8.0), RainfallClass::Heavy);
        assert_eq!(RainfallClass::classify(16.0), RainfallClass::Rainstorm);
        assert_eq!(RainfallClass::classify(32.0), RainfallClass::Downpour);
        assert_eq!(RainfallClass::classify(64.0), RainfallClass::Extreme);
    }

    #[test]
    fn test_rainfall_display() {
        assert_eq!(RainfallClass::Light.to_string(), "小雨");
        assert_eq!(RainfallClass::Moderate.to_string(), "中雨");
        assert_eq!(RainfallClass::Heavy.to_string(), "大雨");
        assert_eq!(RainfallClass::Rainstorm.to_string(), "暴雨");
        assert_eq!(RainfallClass::Downpour.to_string(), "大暴雨");
        assert_eq!(RainfallClass::Extreme.to_string(), "特大暴雨");
    }

    #[test]
    fn test_hazard_weight() {
        assert!((RainfallClass::Light.hazard_weight() - 0.05).abs() < 1e-10);
        assert!(RainfallClass::Extreme.hazard_weight() > 0.9);
        // Monotonic
        let weights = [
            RainfallClass::Light.hazard_weight(),
            RainfallClass::Moderate.hazard_weight(),
            RainfallClass::Heavy.hazard_weight(),
            RainfallClass::Rainstorm.hazard_weight(),
            RainfallClass::Downpour.hazard_weight(),
            RainfallClass::Extreme.hazard_weight(),
        ];
        for w in weights.windows(2) {
            assert!(w[0] < w[1]);
        }
    }

    #[test]
    fn test_cumulative_rainfall() {
        let curve = IdCurve::new(50.0, 0.5);
        // R = α × D^(1-β) = 50 × D^0.5
        let r1 = curve.cumulative_rainfall(1.0);
        let r4 = curve.cumulative_rainfall(4.0);
        assert!((r1 - 50.0).abs() < 1.0);
        assert!((r4 - 100.0).abs() < 2.0); // 50 × 2 = 100
    }

    #[test]
    fn test_is_landslide_trigger() {
        let curve = IdCurve::caine_1980();
        // At D=1h, threshold I ≈ 14.82
        let threshold = curve.intensity(1.0);
        assert!(!curve.is_landslide_trigger(threshold, 1.0));
        assert!(curve.is_landslide_trigger(threshold + 1.0, 1.0));
        assert!(!curve.is_landslide_trigger(threshold - 1.0, 1.0));
    }

    #[test]
    fn test_return_period() {
        let base = IdCurve::new(50.0, 0.5);
        // c=10, d=0.3, T=100yr
        let rp = base.for_return_period(100.0, 10.0, 0.3);
        assert!(rp.alpha > base.alpha); // Return period amplifies α
        assert_eq!(rp.beta, base.beta); // β unchanged
        // α = c × T^d = 10 × 100^0.3 ≈ 39.8
        assert!((rp.alpha - 39.81).abs() < 1.0);
    }

    #[test]
    fn test_prebuilt_thresholds() {
        let caine = IdCurve::caine_1980();
        assert_eq!(caine.alpha, 14.82);
        assert_eq!(caine.beta, 0.39);

        let guzzetti = IdCurve::guzzetti_2008();
        assert_eq!(guzzetti.alpha, 2.20);
        assert_eq!(guzzetti.beta, 0.44);

        let sw = IdCurve::china_southwest();
        assert!(sw.intensity(1.0) > 10.0);

        let se = IdCurve::china_southeast_typhoon();
        assert!(se.intensity(1.0) > 20.0);
    }

    #[test]
    fn test_threshold_consistency() {
        // Higher duration → lower intensity (monotonic decreasing)
        let curve = IdCurve::china_southwest();
        let i1 = curve.intensity(1.0);
        let i6 = curve.intensity(6.0);
        let i24 = curve.intensity(24.0);
        assert!(i1 > i6);
        assert!(i6 > i24);

        // Higher duration → higher cumulative rainfall
        let r1 = curve.cumulative_rainfall(1.0);
        let r24 = curve.cumulative_rainfall(24.0);
        assert!(r24 > r1);
    }
}
