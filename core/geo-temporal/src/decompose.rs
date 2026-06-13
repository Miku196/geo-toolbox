//! 季节分解 — 移动平均分离趋势/季节/残差。
//!
//! 经典乘法分解: Y(t) = T(t) × S(t) × R(t)
//! 或加法分解: Y(t) = T(t) + S(t) + R(t)
//!
//! 适用于月度 NDVI（周期=12）、季度数据（周期=4）等。

/// 分解模式。
#[derive(Debug, Clone, Copy)]
pub enum DecomposeMode {
    /// Y = T + S + R
    Additive,
    /// Y = T × S × R
    Multiplicative,
}

/// 季节分解结果。
#[derive(Debug, Clone)]
pub struct DecomposeResult {
    /// 趋势分量。
    pub trend: Vec<f64>,
    /// 季节分量。
    pub seasonal: Vec<f64>,
    /// 残差分量。
    pub residual: Vec<f64>,
    /// 原始值。
    pub original: Vec<f64>,
}

/// 经典季节分解。
///
/// - `period`: 周期长度（月数据=12，季数据=4）
/// - `mode`: 加法或乘法
pub fn seasonal_decompose(values: &[f64], period: usize, mode: DecomposeMode) -> DecomposeResult {
    let n = values.len();
    if n < 2 * period {
        // 数据太短，返回简单趋势
        let trend = centered_moving_average(values, period);
        let seasonal = vec![0.0; n];
        let residual = values.iter().zip(&trend).map(|(y, t)| y - t).collect();
        return DecomposeResult {
            trend,
            seasonal,
            residual,
            original: values.to_vec(),
        };
    }

    // 1. 趋势 = 中心移动平均
    let trend = centered_moving_average(values, period);

    // 2. 去趋势
    let detrended: Vec<f64> = match mode {
        DecomposeMode::Additive => values.iter().zip(&trend).map(|(y, t)| y - t).collect(),
        DecomposeMode::Multiplicative => values
            .iter()
            .zip(&trend)
            .map(|(y, t)| if *t != 0.0 { y / t } else { 1.0 })
            .collect(),
    };

    // 3. 季节分量 = 各周期同位置的平均值
    let mut seasonal = vec![0.0; n];
    for i in 0..period {
        let mut sum = 0.0;
        let mut count = 0;
        let mut idx = i;
        while idx < n {
            sum += detrended[idx];
            count += 1;
            idx += period;
        }
        let avg = if count > 0 { sum / count as f64 } else { 0.0 };
        let mut idx = i;
        while idx < n {
            seasonal[idx] = avg;
            idx += period;
        }
    }

    // 季节分量中心化
    let season_mean = seasonal.iter().sum::<f64>() / n as f64;
    for s in &mut seasonal {
        *s -= season_mean;
    }

    // 4. 残差
    let residual: Vec<f64> = match mode {
        DecomposeMode::Additive => values
            .iter()
            .zip(&trend)
            .zip(&seasonal)
            .map(|((y, t), s)| y - t - s)
            .collect(),
        DecomposeMode::Multiplicative => values
            .iter()
            .zip(&trend)
            .zip(&seasonal)
            .map(|((y, t), s)| if *t * *s != 0.0 { y / (t * s) } else { 1.0 })
            .collect(),
    };

    DecomposeResult {
        trend,
        seasonal,
        residual,
        original: values.to_vec(),
    }
}

#[allow(clippy::needless_range_loop)]
/// 中心移动平均（消除季节波动）。
fn centered_moving_average(values: &[f64], period: usize) -> Vec<f64> {
    let n = values.len();
    let half = period / 2;
    let mut result = vec![0.0; n];

    for i in 0..n {
        let start = i.saturating_sub(half);
        let end = (i + half + 1).min(n);
        let count = end - start;
        if count > 0 {
            result[i] = values[start..end].iter().sum::<f64>() / count as f64;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decompose_seasonal() {
        // 模拟年周期数据（12个月），有上升趋势
        let mut values = Vec::new();
        for year in 0..3 {
            for month in 0..12 {
                // 趋势: 每年 +0.1, 季节: 夏天高冬天低
                let trend = 0.3 + year as f64 * 0.1;
                let season = 0.1 * ((month as f64 - 3.0) * std::f64::consts::PI / 6.0).sin();
                values.push(trend + season);
            }
        }

        let result = seasonal_decompose(&values, 12, DecomposeMode::Additive);

        assert_eq!(result.original.len(), 36);
        assert_eq!(result.trend.len(), 36);
        // 趋势应该递增
        assert!(result.trend[0] < result.trend[35]);

        // 残差应该很小
        let mean_residual = result.residual.iter().map(|x| x.abs()).sum::<f64>() / 36.0;
        assert!(mean_residual < 0.1);
    }
}
