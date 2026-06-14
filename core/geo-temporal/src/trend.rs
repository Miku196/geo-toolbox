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
            if diff > 0.0 {
                s += 1;
            } else if diff < 0.0 {
                s -= 1;
            }
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
            tau: 0.0,
            p_value: 1.0,
            significant: false,
            sen_slope: 0.0,
            ols_slope: 0.0,
            ols_intercept: 0.0,
            r_squared: 0.0,
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
    let ss_res: f64 = values
        .iter()
        .enumerate()
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
    let sen_slope = if slopes.is_empty() {
        0.0
    } else {
        slopes[slopes.len() / 2]
    };

    let (tau, p_value) = mann_kendall(values);
    let significant = p_value < 0.05;

    TrendResult {
        tau,
        p_value,
        significant,
        sen_slope,
        ols_slope,
        ols_intercept,
        r_squared,
    }
}

/// 标准正态 CDF（Abramowitz & Stegun 近似）。
pub fn normal_cdf(x: f64) -> f64 {
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

/// Pettitt 突变点检测结果。
#[derive(Debug, Clone, serde::Serialize)]
pub struct PettittResult {
    /// 突变点索引（0-based，指向变化之后的第一个值）。
    pub breakpoint: usize,
    /// Pettitt U 统计量。
    pub u_stat: f64,
    /// 近似 p 值。
    pub p_value: f64,
    /// 是否显著 (p < 0.05)。
    pub significant: bool,
    /// 突变前均值。
    pub mean_before: f64,
    /// 突变后均值。
    pub mean_after: f64,
    /// 均值变化量。
    pub change: f64,
}

/// Pettitt 突变点检测：非参数检验，遍历分割点找最大 U 统计量。
pub fn pettitt_test(values: &[f64]) -> PettittResult {
    let n = values.len();
    if n < 3 {
        return PettittResult {
            breakpoint: 0, u_stat: 0.0, p_value: 1.0,
            significant: false, mean_before: 0.0, mean_after: 0.0, change: 0.0,
        };
    }
    let mut max_u = 0.0f64;
    let mut bp = 0usize;
    for t in 1..n {
        let mut u = 0.0f64;
        for i in 0..t {
            for j in t..n {
                u += (values[j] - values[i]).signum();
            }
        }
        let u_abs = u.abs();
        if u_abs > max_u { max_u = u_abs; bp = t; }
    }
    let p = (2.0 * (-6.0 * max_u * max_u / ((n as f64).powi(3) + (n as f64).powi(2))).exp()).min(1.0);
    let mean_before = if bp > 0 { values[..bp].iter().sum::<f64>() / bp as f64 } else { 0.0 };
    let mean_after = if bp < n { values[bp..].iter().sum::<f64>() / (n - bp) as f64 } else { 0.0 };
    PettittResult {
        breakpoint: bp, u_stat: max_u, p_value: p,
        significant: p < 0.05, mean_before, mean_after, change: mean_after - mean_before,
    }
}

/// Theil-Sen 斜率估计（独立函数，两点斜率中位数）。
pub fn sen_slope(values: &[f64]) -> f64 {
    let n = values.len();
    if n < 2 { return 0.0; }
    let mut slopes = Vec::with_capacity(n * (n - 1) / 2);
    for i in 0..n {
        for j in (i + 1)..n {
            slopes.push((values[j] - values[i]) / (j - i) as f64);
        }
    }
    slopes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if slopes.is_empty() { 0.0 } else { slopes[slopes.len() / 2] }
}

/// 季节性 Mann-Kendall：对每个季节分别做 MK 后汇总 S 统计量。
pub fn seasonal_mann_kendall(values: &[f64], season_len: usize) -> TrendResult {
    let n = values.len();
    if n < season_len * 2 || season_len == 0 {
        return TrendResult {
            tau: 0.0, p_value: 1.0, significant: false,
            sen_slope: 0.0, ols_slope: 0.0, ols_intercept: 0.0, r_squared: 0.0,
        };
    }
    let num_seasons = season_len.min(n);
    let mut total_s = 0i64;
    let mut total_var_s = 0.0f64;
    let mut total_m = 0usize;
    for season in 0..num_seasons {
        let mut vals: Vec<f64> = Vec::new();
        let mut k = season;
        while k < n { vals.push(values[k]); k += season_len; }
        let m = vals.len();
        if m < 2 { continue; }
        let mut s = 0i64;
        for i in 0..m {
            for j in (i + 1)..m {
                let d = vals[j] - vals[i];
                if d > 0.0 { s += 1; } else if d < 0.0 { s -= 1; }
            }
        }
        let mut var_s = (m * (m - 1) * (2 * m + 5)) as f64 / 18.0;
        let mut sorted = vals.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mut i = 0;
        while i < m {
            let mut j = i + 1;
            while j < m && (sorted[j] - sorted[i]).abs() < 1e-10 { j += 1; }
            let t = (j - i) as f64;
            if t > 1.0 { var_s -= t * (t - 1.0) * (2.0 * t + 5.0) / 18.0; }
            i = j;
        }
        total_s += s; total_var_s += var_s; total_m += m;
    }
    if total_m < 2 {
        return TrendResult { tau: 0.0, p_value: 1.0, significant: false,
            sen_slope: 0.0, ols_slope: 0.0, ols_intercept: 0.0, r_squared: 0.0 };
    }
    let tau = if total_m < 2 {
        0.0
    } else {
        // tau = S / max(|S|) where max(|S|) = sum of per-season pairs
        let max_s: f64 = (0..num_seasons)
            .map(|season| {
                let mut k = season;
                let mut count = 0usize;
                while k < n { count += 1; k += season_len; }
                let m = count;
                if m >= 2 { (m * (m - 1) / 2) as f64 } else { 0.0 }
            })
            .sum();
        if max_s > 0.0 { total_s as f64 / max_s } else { 0.0 }
    };
    let z = if total_s > 0 { (total_s as f64 - 1.0) / total_var_s.sqrt() }
            else if total_s < 0 { (total_s as f64 + 1.0) / total_var_s.sqrt() }
            else { 0.0 };
    let p = 2.0 * (1.0 - normal_cdf(z.abs()));
    let slope = sen_slope(values);
    let ols = linear_trend(values);
    TrendResult { tau, p_value: p, significant: p < 0.05, sen_slope: slope,
        ols_slope: ols.ols_slope, ols_intercept: ols.ols_intercept, r_squared: ols.r_squared }
}

/// 简化 BFAST：去季节 → Pettitt 分段检测 → 递归断点。
pub fn bfast_simple(values: &[f64], season_len: usize, max_breaks: usize) -> Vec<usize> {
    let n = values.len();
    if n < 6 { return vec![]; }
    let deseasoned: Vec<f64> = if season_len > 0 && n >= season_len * 2 {
        let half = season_len / 2;
        let mut ds = vec![f64::NAN; n];
        for i in 0..n {
            let start = i.saturating_sub(half);
            let end = (i + half + 1).min(n);
            let window: Vec<f64> = values[start..end].iter().filter(|v| v.is_finite()).cloned().collect();
            if window.len() >= season_len / 2 { ds[i] = window.iter().sum::<f64>() / window.len() as f64; }
        }
        for i in 0..n {
            if ds[i].is_nan() {
                if i > 0 && ds[i - 1].is_finite() { ds[i] = ds[i - 1]; }
                else { for j in (i + 1)..n { if ds[j].is_finite() { ds[i] = ds[j]; break; } } }
            }
        }
        ds
    } else { values.to_vec() };

    let mut breaks: Vec<usize> = Vec::new();
    fn detect(data: &[f64], max_b: usize, breaks: &mut Vec<usize>, offset: usize) {
        if breaks.len() >= max_b || data.len() < 6 { return; }
        let r = pettitt_test(data);
        if r.significant && r.breakpoint > 1 && r.breakpoint < data.len() - 1 {
            let bp = offset + r.breakpoint;
            breaks.push(bp);
            detect(&data[..r.breakpoint], max_b, breaks, offset);
            detect(&data[r.breakpoint..], max_b, breaks, bp);
        }
    }
    detect(&deseasoned, max_breaks, &mut breaks, 0);
    breaks.sort();
    breaks.dedup();
    breaks
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
        let (_tau, p) = mann_kendall(&values);
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

    #[test]
    fn test_seasonal_mk() {
        // 月度数据，12个月周期，每周期递增
        let mut vals = Vec::new();
        for yr in 0..3 {
            for m in 0..12 {
                vals.push(0.2 + yr as f64 * 0.1 + m as f64 * 0.01);
            }
        }
        let result = seasonal_mann_kendall(&vals, 12);
        assert!(result.tau > 0.5);
        assert!(result.significant);
    }

    #[test]
    fn test_pettitt() {
        // 前 10 个值 = 1.0，后 10 个 = 10.0
        let mut vals = vec![1.0; 10];
        vals.extend(vec![10.0; 10]);
        let result = pettitt_test(&vals);
        assert_eq!(result.breakpoint, 10);
        assert!(result.significant);
        assert!((result.mean_before - 1.0).abs() < 0.01);
        assert!((result.mean_after - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_sen_slope_standalone() {
        let vals = vec![0.3, 0.35, 0.4, 0.45, 0.5];
        let s = sen_slope(&vals);
        assert!((s - 0.05).abs() < 0.01);
    }

    #[test]
    fn test_bfast_simple() {
        // 前 10 年恒定，后 10 年上升
        let mut vals = vec![0.5; 10];
        for i in 0..10 { vals.push(0.5 + i as f64 * 0.1); }
        let breaks = bfast_simple(&vals, 0, 3);
        assert!(!breaks.is_empty(), "Should detect at least one break: {:?}", breaks);
        // 断点应在 10 附近
        assert!(breaks.contains(&10) || breaks.iter().any(|&b| (b as isize - 10isize).abs() <= 1),
            "Break should be near 10, got {:?}", breaks);
    }
}
