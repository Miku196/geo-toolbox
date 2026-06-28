//! 生境质量模块 — InVEST-like Habitat Quality 模型。
use serde::{Deserialize, Serialize};

/// 生境退化类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecayType {
    /// 线性衰减
    Linear,
    /// 指数衰减
    Exponential,
}

/// InVEST 生境质量评估结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InVestHabitatQuality {
    /// 生境退化度 (0-1)
    pub degradation: Vec<f64>,
    /// 生境质量 (0-1)
    pub quality: Vec<f64>,
    /// 每像素威胁总强度
    pub threat_intensity: Vec<f64>,
    /// 生境适宜性均值
    pub mean_quality: f64,
    /// 退化面积占比 (quality < 0.5)
    pub degraded_ratio: f64,
}

/// 计算生境退化度: D_j = sum_r(w_r / sum(w)) * i_rxy * S_jr
///
/// 参数:
/// - `landcover` — 土地覆盖类型编码 (0 = 非生境, >0 = 生境类型)
/// - `threat_layers` — 威胁图层 (每个威胁一个 Vec<f64>, 0-1 强度)
/// - `threat_weights` — 各威胁的权重
/// - `sensitivity` — 每种土地覆盖对各威胁的敏感度 [n_covers x n_threats]
/// - `decay` — 衰减类型 (Linear/Exponential)
/// - `half_saturation` — 半饱和常数 (km)
/// - `cell_size_m` — 像元大小 (m)
/// - `cols` — 列数
pub fn habitat_degradation(
    landcover: &[u32],
    threat_layers: &[Vec<f64>],
    threat_weights: &[f64],
    sensitivity: &[Vec<f64>],
    decay: DecayType,
    half_saturation: f64,
    cell_size_m: f64,
    cols: usize,
) -> Vec<f64> {
    let n = landcover.len();
    let n_threats = threat_layers.len();
    let _rows = n / cols;
    let total_weight: f64 = threat_weights.iter().sum();
    let cell_km = cell_size_m / 1000.0;

    let mut degradation = vec![0.0; n];

    for i in 0..n {
        let lc = landcover[i] as usize;
        if lc == 0 {
            continue;
        }
        let mut d = 0.0;
        for t in 0..n_threats {
            let w_r = threat_weights.get(t).copied().unwrap_or(1.0) / total_weight.max(1.0);
            let s_jr = sensitivity
                .get(lc)
                .and_then(|sens| sens.get(t))
                .copied()
                .unwrap_or(0.0);
            if s_jr <= 0.0 {
                continue;
            }
            let row = i / cols;
            let col = i % cols;

            // Sum threat intensity from all pixels within influence radius
            for ti in 0..n {
                if threat_layers[t].get(ti).copied().unwrap_or(0.0) <= 0.0 {
                    continue;
                }
                let t_row = ti / cols;
                let t_col = ti % cols;
                let dr = (row as f64 - t_row as f64).abs() * cell_km;
                let dc = (col as f64 - t_col as f64).abs() * cell_km;
                let dist_km = (dr * dr + dc * dc).sqrt();
                let i_rxy = match decay {
                    DecayType::Linear => (1.0 - dist_km / half_saturation).max(0.0),
                    DecayType::Exponential => (-dist_km / half_saturation).exp(),
                };
                d += threat_layers[t][ti] * w_r * i_rxy * s_jr;
            }
        }
        degradation[i] = d;
    }
    degradation
}

/// 计算生境质量: Q_j = H_j * (1 - D_j^z / (D_j^z + k^z))
///
/// 其中 H_j = 生境适宜性, k = half_saturation, z = 默认 2.5
pub fn habitat_quality(
    landcover: &[u32],
    habitat_suitability: &[f64],
    degradation: &[f64],
    half_saturation: f64,
    z: f64,
) -> Vec<f64> {
    let n = landcover.len();
    let k = half_saturation;
    (0..n)
        .map(|i| {
            let lc = landcover[i] as usize;
            let h = habitat_suitability.get(lc).copied().unwrap_or(0.0);
            let d = degradation.get(i).copied().unwrap_or(0.0);
            if h <= 0.0 || d <= 0.0 {
                return 0.0;
            }
            let dz = d.powf(z);
            let q = h * (1.0 - dz / (dz + k.powf(z)));
            q.max(0.0)
        })
        .collect()
}

/// 完整 InVEST 生境质量评估管线。
pub fn assess_habitat_quality(
    landcover: &[u32],
    habitat_suitability: &[f64],
    threat_layers: &[Vec<f64>],
    threat_weights: &[f64],
    sensitivity: &[Vec<f64>],
    decay: DecayType,
    half_saturation: f64,
    cell_size_m: f64,
    cols: usize,
) -> InVestHabitatQuality {
    let degradation = habitat_degradation(
        landcover,
        threat_layers,
        threat_weights,
        sensitivity,
        decay,
        half_saturation,
        cell_size_m,
        cols,
    );
    let quality = habitat_quality(
        landcover,
        habitat_suitability,
        &degradation,
        half_saturation,
        2.5,
    );
    let mean_q = quality.iter().copied().sum::<f64>() / quality.len().max(1) as f64;
    let degraded =
        quality.iter().filter(|&&q| q < 0.5).count() as f64 / quality.len().max(1) as f64;
    let threat_int: Vec<f64> = (0..landcover.len())
        .map(|i| {
            threat_layers
                .iter()
                .map(|tl| tl.get(i).copied().unwrap_or(0.0))
                .sum()
        })
        .collect();
    InVestHabitatQuality {
        degradation,
        quality,
        threat_intensity: threat_int,
        mean_quality: mean_q,
        degraded_ratio: degraded,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_habitat_degradation_no_threats() {
        let lc = vec![1u32; 100];
        let threats: Vec<Vec<f64>> = vec![vec![0.0; 100]];
        let weights = vec![1.0];
        let sens = vec![vec![1.0]];
        let deg = habitat_degradation(
            &lc,
            &threats,
            &weights,
            &sens,
            DecayType::Linear,
            5.0,
            30.0,
            10,
        );
        assert!(deg.iter().all(|&d| d == 0.0));
    }

    #[test]
    fn test_habitat_degradation_with_threat() {
        let mut lc = vec![1u32; 100];
        lc[0] = 0;
        let mut threat = vec![0.0; 100];
        threat[50] = 1.0;
        let threats = vec![threat];
        let weights = vec![1.0];
        // sensitivity[lc][threat]: lc=0 is non-habitat, lc=1 is forest with sens=0.8
        let sens = vec![vec![0.0], vec![0.8]];
        let deg = habitat_degradation(
            &lc,
            &threats,
            &weights,
            &sens,
            DecayType::Linear,
            10.0,
            30.0,
            10,
        );
        assert!(deg[50] > 0.0, "deg[50] should be > 0, got {}", deg[50]);
        assert!(deg.iter().any(|&d| d > 0.0));
    }

    #[test]
    fn test_habitat_quality_uniform() {
        let lc = vec![1u32; 25];
        let suit = vec![1.0];
        let deg = vec![0.0; 25];
        let q = habitat_quality(&lc, &suit, &deg, 0.5, 2.5);
        assert!(q.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn test_habitat_quality_high_degradation() {
        let lc = vec![1u32; 1];
        let suit = vec![1.0];
        let deg = vec![10.0];
        let q = habitat_quality(&lc, &suit, &deg, 0.5, 2.5);
        assert!(q[0] < 0.01);
    }

    #[test]
    fn test_assess_full() {
        let lc = vec![1u32; 100];
        let suit = vec![0.0, 0.8];
        let mut threat = vec![0.0; 100];
        threat[0] = 1.0;
        threat[99] = 1.0;
        let result = assess_habitat_quality(
            &lc,
            &suit,
            &[threat],
            &[1.0],
            &[vec![0.0], vec![0.5]],
            DecayType::Linear,
            10.0,
            30.0,
            10,
        );
        assert!(result.mean_quality > 0.0);
        assert!(!result.degradation.is_empty());
    }
}
