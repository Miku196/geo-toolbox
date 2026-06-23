//! 地震目录工具 — Gutenberg-Richter 关系、Poisson 概率、b 值估计。
use serde::{Deserialize, Serialize};

/// 地震目录统计结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeismicityResult {
    /// 总事件数
    pub event_count: usize,
    /// 最小完备震级 Mc
    pub min_completeness_mag: f64,
    /// b 值 (MLE)
    pub b_value_mle: f64,
    /// a 值
    pub a_value: f64,
    /// 年均发生率 (≥ Mc)
    pub annual_rate: f64,
}

/// Gutenberg-Richter 分布: log10(N) = a - b*M
pub fn gutenberg_richter(a: f64, b: f64, m_min: f64, m_max: f64, step: f64) -> Vec<(f64, f64)> {
    let mut points = Vec::new();
    let mut m = m_min;
    while m <= m_max {
        let n = 10.0_f64.powf(a - b * m);
        points.push((m, n));
        m += step;
    }
    points
}

/// Poisson 概率: P(N≥1 in t years) = 1 - exp(-λ*t)
pub fn poisson_probability(annual_rate: f64, time_years: f64) -> f64 {
    if annual_rate <= 0.0 {
        return 0.0;
    }
    1.0 - (-annual_rate * time_years).exp()
}

/// b 值极大似然估计 (Aki 1965): b = log10(e) / (M_mean - M_min)
pub fn b_value_mle(magnitudes: &[f64], min_mag: f64) -> f64 {
    if magnitudes.is_empty() {
        return 0.0;
    }
    let filtered: Vec<f64> = magnitudes
        .iter()
        .copied()
        .filter(|&m| m >= min_mag)
        .collect();
    if filtered.len() < 2 {
        return 0.0;
    }
    let mean_m = filtered.iter().sum::<f64>() / filtered.len() as f64;
    if mean_m <= min_mag {
        return 1.0;
    }
    (std::f64::consts::LOG10_E) / (mean_m - min_mag)
}

/// a 值从 b 值 + 发生率计算: a = log10(N_total) + b * M_min
pub fn a_value_from_b(b: f64, total_events: f64, min_mag: f64, time_span_years: f64) -> f64 {
    if time_span_years <= 0.0 || total_events <= 0.0 {
        return 0.0;
    }
    let annual_rate = total_events / time_span_years;
    annual_rate.log10() + b * min_mag
}

/// 完整地震目录统计。
pub fn seismicity_analysis(
    magnitudes: &[f64],
    min_mag: f64,
    time_span_years: f64,
) -> SeismicityResult {
    let total: Vec<f64> = magnitudes
        .iter()
        .copied()
        .filter(|&m| m >= min_mag)
        .collect();
    let b = b_value_mle(magnitudes, min_mag);
    let a = a_value_from_b(b, total.len() as f64, min_mag, time_span_years);
    let annual_rate = if time_span_years > 0.0 {
        total.len() as f64 / time_span_years
    } else {
        0.0
    };
    SeismicityResult {
        event_count: total.len(),
        min_completeness_mag: min_mag,
        b_value_mle: (b * 100.0).round() / 100.0,
        a_value: (a * 100.0).round() / 100.0,
        annual_rate: (annual_rate * 100.0).round() / 100.0,
    }
}

/// 地震复发间隔: Tr = 1 / λ (year)
pub fn recurrence_interval(annual_rate: f64) -> f64 {
    if annual_rate <= 0.0 {
        f64::INFINITY
    } else {
        1.0 / annual_rate
    }
}

/// 给定时间段内至少发生一次 M≥m 的概率。
pub fn probability_at_least_one(annual_rate: f64, years: f64) -> f64 {
    poisson_probability(annual_rate, years)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gutenberg_richter_points() {
        let points = gutenberg_richter(4.0, 1.0, 4.0, 8.0, 0.5);
        assert_eq!(points.len(), 9);
        assert!((points[0].1 - 1.0).abs() < 0.1); // 10^(4-4) = 1
        assert!((points[4].1 - 0.01).abs() < 0.001); // 10^(4-6) = 0.01
    }

    #[test]
    fn test_poisson_probability() {
        let p = poisson_probability(0.01, 100.0);
        assert!((p - (1.0 - (-1.0_f64).exp())).abs() < 0.001);
    }

    #[test]
    fn test_b_value_mle() {
        let mags: Vec<f64> = (0..100).map(|i| 3.0 + (i as f64) * 0.01).collect();
        let b = b_value_mle(&mags, 3.0);
        assert!(b > 0.3 && b < 3.0);
    }

    #[test]
    fn test_seismicity_analysis() {
        let mags: Vec<f64> = (0..200)
            .map(|i| 2.0 + (i as f64).sin().abs() * 5.0)
            .collect();
        let result = seismicity_analysis(&mags, 3.0, 50.0);
        assert!(result.event_count > 0);
        assert!(result.b_value_mle > 0.0);
        assert!(result.annual_rate > 0.0);
    }

    #[test]
    fn test_recurrence_interval() {
        let tr = recurrence_interval(0.02);
        assert!((tr - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_probability_at_least_one() {
        let p = probability_at_least_one(0.002, 50.0);
        assert!(p > 0.05 && p < 0.2);
    }
}
