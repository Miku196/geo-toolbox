//! 物种分布模块 — 简化 MaxEnt 物种分布模型（存在点 vs 背景）。
use serde::{Deserialize, Serialize};

/// MaxEnt 拟合系数（每环境变量 + 截距）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaxEntModel {
    /// 环境变量名
    pub feature_names: Vec<String>,
    /// 系数 [intercept, beta_1, ..., beta_n]
    pub coefficients: Vec<f64>,
    /// 正则化系数
    pub regularization: f64,
}

/// 物种分布适生性结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesSuitability {
    /// 适生性值 (0-1)
    pub suitability: Vec<f64>,
    /// 高适生区占比 (> 0.7)
    pub high_suitability_ratio: f64,
    /// 中等适生区占比 (0.4-0.7)
    pub medium_suitability_ratio: f64,
    /// 低适生区占比 (< 0.4)
    pub low_suitability_ratio: f64,
    /// 最大适生性
    pub max_suitability: f64,
}

/// 用逻辑回归拟合存在点 vs 背景点的环境值。
///
/// 参数:
/// - `presence_values` — 存在点的环境变量值 [n_presence x n_vars]
/// - `background_values` — 背景点的环境变量值 [n_background x n_vars]
/// - `regularization` — L1 正则化系数 (lambda)
///
/// 返回拟合系数 [intercept, beta_1, ..., beta_n]
pub fn fit_maxent(
    presence_values: &[Vec<f64>],
    background_values: &[Vec<f64>],
    regularization: f64,
) -> Vec<f64> {
    let n_vars = if let Some(p) = presence_values.first() {
        p.len()
    } else {
        return vec![];
    };
    let n_pres = presence_values.len();
    let n_bg = background_values.len();

    if n_pres == 0 || n_bg == 0 {
        return vec![0.0; n_vars + 1];
    }

    // 简化的梯度下降 (无截距 = 均匀先验)
    let lr = 0.01;
    let n_iter = 1000;
    let mut coeffs = vec![0.0; n_vars + 1]; // [intercept, b1..bn]

    for _ in 0..n_iter {
        let mut grad = vec![0.0; n_vars + 1];
        let mut _total_loss = 0.0;

        // 存在点梯度 (最大化)
        for pv in presence_values {
            let linear = coeffs[0]
                + pv.iter()
                    .zip(coeffs[1..].iter())
                    .map(|(x, b)| x * b)
                    .sum::<f64>();
            let prob = 1.0 / (1.0 + (-linear).exp());
            grad[0] += (1.0 - prob) / n_pres as f64;
            for (j, &x) in pv.iter().enumerate() {
                grad[j + 1] += (1.0 - prob) * x / n_pres as f64;
            }
            _total_loss += (prob.max(1e-10)).ln();
        }

        // 背景点梯度 (最小化)
        for bv in background_values {
            let linear = coeffs[0]
                + bv.iter()
                    .zip(coeffs[1..].iter())
                    .map(|(x, b)| x * b)
                    .sum::<f64>();
            let prob = 1.0 / (1.0 + (-linear).exp());
            grad[0] -= prob / n_bg as f64;
            for (j, &x) in bv.iter().enumerate() {
                grad[j + 1] -= prob * x / n_bg as f64;
            }
        }

        // L1 正则化
        for j in 1..coeffs.len() {
            grad[j] -= regularization * coeffs[j].signum() / (n_pres + n_bg) as f64;
        }

        // 更新
        for j in 0..coeffs.len() {
            coeffs[j] += lr * grad[j];
        }
    }

    coeffs
}

/// 计算单像素物种适生性 (0-1): P = 1 / (1 + exp(-(b0 + b1*x1 + ... + bn*xn)))
pub fn species_suitability(env_values: &[f64], coefficients: &[f64]) -> f64 {
    if coefficients.is_empty() {
        return 0.0;
    }
    let linear = coefficients[0]
        + env_values
            .iter()
            .zip(coefficients[1..].iter())
            .map(|(x, b)| x * b)
            .sum::<f64>();
    1.0 / (1.0 + (-linear).exp())
}

/// 批量计算栅格适生性。
pub fn maxent_predict(env_layers: &[Vec<f64>], coefficients: &[f64]) -> Vec<f64> {
    if env_layers.is_empty() {
        return vec![];
    }
    let n = env_layers[0].len();
    (0..n)
        .map(|i| {
            let vals: Vec<f64> = env_layers
                .iter()
                .map(|layer| layer.get(i).copied().unwrap_or(0.0))
                .collect();
            species_suitability(&vals, coefficients)
        })
        .collect()
}

/// 完整 MaxEnt 管线: 拟合 + 预测 + 分区统计。
pub fn maxent_simple(
    env_layers: &[Vec<f64>],
    presence_pixels: &[usize],
    background_pixels: &[usize],
    regularization: f64,
) -> SpeciesSuitability {
    let _n_vars = env_layers.len();

    // 构建存在点特征矩阵
    let presence_values: Vec<Vec<f64>> = presence_pixels
        .iter()
        .map(|&idx| {
            env_layers
                .iter()
                .map(|l| l.get(idx).copied().unwrap_or(0.0))
                .collect()
        })
        .collect();

    // 构建背景点特征矩阵
    let background_values: Vec<Vec<f64>> = background_pixels
        .iter()
        .map(|&idx| {
            env_layers
                .iter()
                .map(|l| l.get(idx).copied().unwrap_or(0.0))
                .collect()
        })
        .collect();

    let coeffs = fit_maxent(&presence_values, &background_values, regularization);

    // 预测全栅格
    let suitability = maxent_predict(env_layers, &coeffs);
    let total = suitability.len().max(1) as f64;
    let high = suitability.iter().filter(|&&v| v > 0.7).count() as f64 / total;
    let medium = suitability
        .iter()
        .filter(|&&v| (0.4..=0.7).contains(&v))
        .count() as f64
        / total;
    let low = suitability.iter().filter(|&&v| v < 0.4).count() as f64 / total;
    let max_s = suitability.iter().copied().fold(0.0_f64, f64::max);

    SpeciesSuitability {
        suitability,
        high_suitability_ratio: high,
        medium_suitability_ratio: medium,
        low_suitability_ratio: low,
        max_suitability: max_s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_species_suitability_uniform() {
        let val = species_suitability(&[0.0, 0.0], &[0.0, 1.0, 0.0]);
        assert!((val - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_species_suitability_high() {
        // score = 10*1 + 12*0.5 + (-10) = 10+6-10 = 6, prob = 1/(1+e^-6) ≈ 0.998 > 0.9
        let val = species_suitability(&[10.0, 12.0], &[-10.0, 1.0, 0.5]);
        assert!(val > 0.9);
    }

    #[test]
    fn test_species_suitability_low() {
        // linear = -10 + 0 + 0 = -10, prob = 1/(1+e^10) ≈ 0.000045 < 0.01
        let val = species_suitability(&[0.0, 0.0], &[-10.0, 1.0, 1.0]);
        assert!(val < 0.01);
    }

    #[test]
    fn test_fit_maxent_separable() {
        // Two clusters: presence at [5,5], background at [0,0]
        let pres = vec![vec![5.0, 5.0]];
        let bg = vec![vec![0.0, 0.0]];
        let coeffs = fit_maxent(&pres, &bg, 0.01);
        assert!(!coeffs.is_empty());
        // Presence should have high probability
        let p_prob = species_suitability(&[5.0, 5.0], &coeffs);
        let bg_prob = species_suitability(&[0.0, 0.0], &coeffs);
        assert!(p_prob > bg_prob);
    }

    #[test]
    fn test_maxent_predict() {
        let env = vec![vec![0.0, 5.0], vec![0.0, 5.0]];
        let coeffs = vec![-5.0, 1.0, 1.0];
        let pred = maxent_predict(&env, &coeffs);
        assert_eq!(pred.len(), 2);
        assert!(pred[1] > pred[0]);
    }

    #[test]
    fn test_maxent_simple() {
        let env = vec![
            vec![0.1, 0.2, 0.8, 0.9, 0.5, 0.3],
            vec![0.1, 0.2, 0.7, 0.8, 0.5, 0.3],
        ];
        let pres = vec![2, 3];
        let bg = vec![0, 1, 5];
        let result = maxent_simple(&env, &pres, &bg, 0.01);
        assert_eq!(result.suitability.len(), 6);
        assert!(result.high_suitability_ratio > 0.0 || result.max_suitability > 0.0);
    }
}
