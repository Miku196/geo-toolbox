//! 趋势分析 — 线性回归 + Mann-Kendall 非参数检验。
//!
//! ## Mann-Kendall
//!
//! 非参数趋势检验，不假设数据服从正态分布，适合 NDVI 时间序列。
//!
//! H₀: 无趋势  vs  H₁: 存在单调趋势
//!
//! τ ∈ [-1, 1]: 正值 = 上升趋势，负值 = 下降趋势
//! p < 0.05: 趋势显著


/// 趋势分析结果。
#[derive(Debug, Clone, serde::Serialize)]
pub struct TrendResult {
    /// Mann-Kendall τ 统计量 [-1, 1]。
    pub tau: f64,
    /// 双侧 p 值。
    pub p_value: f64,
    /// 是否显著 (p < 0.05)。
    pub significant: bool,
    /// Theil-Sen 斜率（稳健回归，不受异常值影响）。
    pub sen_slope: f64,
    /// 线性回归斜率 (OLS)。
    pub ols_slope: f64,
    /// 线性回归截距。
    pub ols_intercept: f64,
    /// 线性回归 R²。
    pub r_squared: f64,
}

/// Mann-Kendall 趋势检验。
///
/// 输入：按时间顺序排列的值序列。
/// 返回 (τ, 双侧p值)。
pub fn mann_kendall(values: &[f64]) -> (f64, f64) {
    let n = values.len();
    if n < 3 {
        return (0.0, 1.0);
    }

    // 计算 S 统计量
    let mut s = 0i64;
    for i in 0..n {
        for j in (i + 1)..n {
            let diff = values[j] - values[i];
            if diff > 0.0 { s += 1; }
            else if diff < 0.0 { s -= 1; }
        }
    }

    // 计算 τ
    let tau = s as f64 / (n * (n - 1) / 2) as f64;

    // 方差（含 ties 修正）
    let mut var_s = (n * (n - 1) * (2 * n + 5)) as f64 / 18.0;

    // ties 修正
    let mut sorted: Vec<f64> = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mut i = 0;
    while i < n {
        let mut j = i + 1;
        while j < n && (sorted[j] - sorted[i]).abs() < 1e-10 {
            j += 1;
        }
        let t = j - i;
        if t > 1 {
            let t = t as f64;
            var_s -= t * (t - 1.0) * (2.0 * t + 5.0) / 18.0;
        }
        i = j;
    }

    // Z 统计量
    let z = if s > 0 {
        (s as f64 - 1.0) / var_s.sqrt()
    } else if s < 0 {
        (s as f64 + 1.0) / var_s.sqrt()
    } else {
        0.0
    };

    // 双侧 p 值（正态近似）
    let p = 2.0 * (1.0 - normal_cdf(z.abs()));

    (tau, p)
}

/// 线性趋势分析（OLS 回归 + Theil-Sen 稳健回归）。
pub fn linear_trend(values: &[f64]) -> TrendResult {
    let n = values.len();
    if n < 2 {
        return TrendResult {
            tau: 0.0, p_value: 1.0, significant: false,
            sen_slope: 0.0, ols_slope: 0.0, ols_intercept: 0.0, r_squared: 0.0,
        };
    }

    // OLS: y = a + b*x, x = 0..n-1
    let x_mean = (n - 1) as f64 / 2.0;
    let y_mean = values.iter().sum::<f64>() / n as f64;

    let mut xy_cov = 0.0;
    let mut xx_var = 0.0;
    for (i, &y) in values.iter().enumerate() {
        let dx = i as f64 - x_mean;
        xy_cov += dx * (y - y_mean);
        xx_var += dx * dx;
    }

    let ols_slope = xy_cov / xx_var.max(1e-10);
    let ols_intercept = y_mean - ols_slope * x_mean;

    // R²
    let ss_res: f64 = values.iter().enumerate()
        .map(|(i, &y)| (y - (ols_intercept + ols_slope * i as f64)).powi(2))
        .sum();
    let ss_tot: f64 = values.iter().map(|&y| (y - y_mean).powi(2)).sum();
    let r_squared = 1.0 - ss_res / ss_tot.max(1e-10);

    // Theil-Sen 斜率（所有两点斜率的中位数）
    let mut slopes = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            let s = (values[j] - values[i]) / (j - i) as f64;
            slopes.push(s);
        }
    }
    slopes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let sen_slope = if slopes.is_empty() { 0.0 } else { slopes[slopes.len() / 2] };

    let (tau, p_value) = mann_kendall(values);
    let significant = p_value < 0.05;

    TrendResult { tau, p_value, significant, sen_slope, ols_slope, ols_intercept, r_squared }
}

/// 标准正态 CDF（Abramowitz & Stegun 近似）。
fn normal_cdf(x: f64) -> f64 {
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p_val = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + p_val * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x / 2.0).exp();

    0.5 * (1.0 + sign * y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_increasing_trend() {
        let values = vec![0.3, 0.35, 0.4, 0.45, 0.5];
        let result = linear_trend(&values);
        assert!(result.ols_slope > 0.0);
        assert!(result.sen_slope > 0.0);
        assert!(result.r_squared > 0.95);
    }

    #[test]
    fn test_decreasing_trend() {
        let values = vec![0.5, 0.45, 0.4, 0.35, 0.3];
        let result = linear_trend(&values);
        assert!(result.ols_slope < 0.0);
        assert!(result.tau < 0.0);
    }

    #[test]
    fn test_no_trend() {
        // 随机波动，无趋势
        let values = vec![0.5, 0.48, 0.52, 0.49, 0.51];
        let (tau, p) = mann_kendall(&values);
        assert!(p > 0.1, "p={p} should be > 0.1 for no-trend data");
    }

    #[test]
    fn test_significant_trend() {
        // 强上升趋势
        let values: Vec<f64> = (0..10).map(|i| 0.2 + i as f64 * 0.05).collect();
        let result = linear_trend(&values);
        assert!(result.significant);
        assert!(result.tau > 0.8);
    }

    #[test]
    fn test_normal_cdf() {
        assert!((normal_cdf(0.0) - 0.5).abs() < 0.01);
        assert!(normal_cdf(1.96) > 0.97);
        assert!(normal_cdf(-1.96) < 0.03);
    }
}
