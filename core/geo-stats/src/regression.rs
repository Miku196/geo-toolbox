//! Ordinary Least Squares (OLS) 线性回归。
//!
//! 提供简单线性回归 y = a + bx 的计算、预测和残差分析。

use serde::{Deserialize, Serialize};

/// OLS 线性回归结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OlsResult {
    /// 回归斜率 (b in y = a + bx)
    pub slope: f64,
    /// 回归截距 (a in y = a + bx)
    pub intercept: f64,
    /// 决定系数 R²
    pub r_squared: f64,
    /// 均方根误差 RMSE
    pub rmse: f64,
    /// 样本数量
    pub n: usize,
    /// 斜率的标准误
    pub slope_se: f64,
    /// 截距的标准误
    pub intercept_se: f64,
}

/// 计算 OLS 线性回归 y = a + bx。
///
/// # 参数
/// * `x` — 自变量观测值
/// * `y` — 因变量观测值
///
/// # 返回
/// * `Some(OlsResult)` — 回归结果
/// * `None` — 如果输入长度不匹配、长度 < 2、或数据无有效变化（零方差）
///
/// # 公式
/// * b = Σ(xi-x̄)(yi-ȳ) / Σ(xi-x̄)²
/// * a = ȳ - b·x̄
/// * R² = 1 - SS_res / SS_tot
/// * RMSE = √(SS_res / (n-2))
/// * SE_slope = √(MSE / Σ(xi-x̄)²)
/// * SE_intercept = √(MSE · (1/n + x̄²/Σ(xi-x̄)²))
pub fn ols_regression(x: &[f64], y: &[f64]) -> Option<OlsResult> {
    if x.len() != y.len() || x.len() < 2 {
        return None;
    }

    let n = x.len();

    // 去除 NaN / Inf
    let mut clean_x = Vec::with_capacity(n);
    let mut clean_y = Vec::with_capacity(n);
    for (&xi, &yi) in x.iter().zip(y.iter()) {
        if xi.is_finite() && yi.is_finite() {
            clean_x.push(xi);
            clean_y.push(yi);
        }
    }

    if clean_x.len() < 2 {
        return None;
    }

    let n = clean_x.len();
    let nf = n as f64;

    let mean_x = clean_x.iter().sum::<f64>() / nf;
    let mean_y = clean_y.iter().sum::<f64>() / nf;

    let mut ss_xx = 0.0_f64;
    let mut ss_xy = 0.0_f64;
    for (&xi, &yi) in clean_x.iter().zip(clean_y.iter()) {
        let dx = xi - mean_x;
        let dy = yi - mean_y;
        ss_xx += dx * dx;
        ss_xy += dx * dy;
    }

    if ss_xx.abs() < f64::EPSILON {
        return None; // 零方差 x
    }

    let slope = ss_xy / ss_xx;
    let intercept = mean_y - slope * mean_x;

    let mut ss_res = 0.0_f64;
    let mut ss_tot = 0.0_f64;
    for (&xi, &yi) in clean_x.iter().zip(clean_y.iter()) {
        let y_pred = slope * xi + intercept;
        let res = yi - y_pred;
        let dev = yi - mean_y;
        ss_res += res * res;
        ss_tot += dev * dev;
    }

    let r_squared = if ss_tot.abs() < f64::EPSILON {
        1.0 // 完美预测
    } else {
        1.0 - ss_res / ss_tot
    };

    let mse = ss_res / (nf - 2.0); // 自由度为 n-2
    let rmse = mse.sqrt();
    let slope_se = (mse / ss_xx).sqrt();
    let intercept_se = (mse * (1.0 / nf + mean_x * mean_x / ss_xx)).sqrt();

    Some(OlsResult {
        slope,
        intercept,
        r_squared,
        rmse,
        n,
        slope_se,
        intercept_se,
    })
}

/// 根据回归结果预测单点 x 对应的 y 值。
pub fn predict(x: f64, result: &OlsResult) -> f64 {
    result.slope * x + result.intercept
}

/// 根据回归结果批量预测多个 x 值。
pub fn predict_batch(x: &[f64], result: &OlsResult) -> Vec<f64> {
    x.iter()
        .map(|&xi| result.slope * xi + result.intercept)
        .collect()
}

/// 计算回归残差 (观测值 - 预测值)。
///
/// 返回 None 如果 x.len() != y.len()。
pub fn residuals(x: &[f64], y: &[f64], result: &OlsResult) -> Option<Vec<f64>> {
    if x.len() != y.len() {
        return None;
    }
    Some(
        x.iter()
            .zip(y.iter())
            .map(|(&xi, &yi)| yi - (result.slope * xi + result.intercept))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_linear() {
        // y = 2x + 3
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let y = vec![3.0, 5.0, 7.0, 9.0, 11.0];
        let result = ols_regression(&x, &y).expect("regression failed");
        assert!((result.slope - 2.0).abs() < 1e-10);
        assert!((result.intercept - 3.0).abs() < 1e-10);
        assert!((result.r_squared - 1.0).abs() < 1e-10);
        assert!((result.rmse - 0.0).abs() < 1e-10);
        assert_eq!(result.n, 5);
    }

    #[test]
    fn test_noisy_data() {
        // y ≈ 0.5x + 1, with small noise
        let x = vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0];
        let y = vec![1.1, 2.0, 3.2, 4.1, 4.9, 6.0];
        let result = ols_regression(&x, &y).expect("regression failed");
        assert!((result.slope - 0.5).abs() < 0.1);
        assert!((result.intercept - 1.0).abs() < 0.3);
        assert!(result.r_squared > 0.95 && result.r_squared <= 1.0);
        assert!(result.rmse > 0.0);
    }

    #[test]
    fn test_predict() {
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let y = vec![3.0, 5.0, 7.0, 9.0, 11.0];
        let result = ols_regression(&x, &y).expect("regression failed");
        let pred = predict(5.0, &result);
        assert!((pred - 13.0).abs() < 1e-10);
    }

    #[test]
    fn test_predict_batch() {
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let y = vec![3.0, 5.0, 7.0, 9.0, 11.0];
        let result = ols_regression(&x, &y).expect("regression failed");
        let preds = predict_batch(&[5.0, 6.0], &result);
        assert!((preds[0] - 13.0).abs() < 1e-10);
        assert!((preds[1] - 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_residuals() {
        let x = vec![0.0, 1.0, 2.0, 3.0];
        let y = vec![3.0, 5.0, 7.0, 9.0];
        let result = ols_regression(&x, &y).expect("regression failed");
        let res = residuals(&x, &y, &result).expect("residuals failed");
        for r in &res {
            assert!(r.abs() < 1e-10);
        }
    }

    #[test]
    fn test_mismatched_length() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 2.0];
        assert!(ols_regression(&x, &y).is_none());
    }

    #[test]
    fn test_too_few_points() {
        let x = vec![1.0];
        let y = vec![2.0];
        assert!(ols_regression(&x, &y).is_none());
    }

    #[test]
    fn test_empty() {
        let x: Vec<f64> = vec![];
        let y: Vec<f64> = vec![];
        assert!(ols_regression(&x, &y).is_none());
    }

    #[test]
    fn test_nan_in_input() {
        let x = vec![1.0, f64::NAN, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let result = ols_regression(&x, &y);
        assert!(result.is_some()); // should skip NaN and work with remaining
        let r = result.unwrap();
        assert!((r.slope - 2.0).abs() < 1e-10);
        assert!((r.intercept - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_inf_in_input() {
        let x = vec![1.0, 2.0, f64::INFINITY, 4.0];
        let y = vec![2.0, 4.0, 8.0, 8.0];
        let result = ols_regression(&x, &y);
        assert!(result.is_some()); // should skip Inf
    }

    #[test]
    fn test_constant_x() {
        // x 全相等 → 零方差 → 应返回 None
        let x = vec![1.0, 1.0, 1.0, 1.0];
        let y = vec![1.0, 2.0, 3.0, 4.0];
        assert!(ols_regression(&x, &y).is_none());
    }

    #[test]
    fn test_standard_errors_positive() {
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.1, 4.2, 6.0, 8.3, 10.1, 11.9];
        let result = ols_regression(&x, &y).expect("regression failed");
        assert!(result.slope_se > 0.0);
        assert!(result.intercept_se > 0.0);
    }

    #[test]
    fn test_negative_correlation() {
        // y = -3x + 10
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let y = vec![10.0, 7.0, 4.0, 1.0, -2.0];
        let result = ols_regression(&x, &y).expect("regression failed");
        assert!((result.slope - (-3.0)).abs() < 1e-10);
        assert!((result.intercept - 10.0).abs() < 1e-10);
    }
}
