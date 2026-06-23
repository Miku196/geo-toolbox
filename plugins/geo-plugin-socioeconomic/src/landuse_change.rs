/// CA-Markov 土地利用变化模拟模块。
use serde::{Deserialize, Serialize};

/// CA 模拟结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaMarkovResult {
    /// 模拟后的 LULC 网格
    pub final_lulc: Vec<u8>,
    /// 每步迭代的 LULC 快照 (每步变化像元数)
    pub changes_per_step: Vec<usize>,
    /// 总变化像元数
    pub total_changed: usize,
    /// 最终各类面积占比
    pub class_area_fractions: Vec<(u8, f64)>,
}

/// 计算两期 LULC 之间的转移概率矩阵。
/// matrix[from][to] = 转移概率 (0-1)
pub fn transition_probability(from_lulc: &[u8], to_lulc: &[u8], n_classes: u8) -> Vec<Vec<f64>> {
    let n = n_classes as usize;
    let mut count = vec![vec![0usize; n]; n];
    let mut from_count = vec![0usize; n];
    for (f, t) in from_lulc.iter().zip(to_lulc.iter()) {
        let fi = *f as usize;
        let ti = *t as usize;
        if fi < n && ti < n {
            count[fi][ti] += 1;
            from_count[fi] += 1;
        }
    }
    let mut matrix = vec![vec![0.0; n]; n];
    for i in 0..n {
        if from_count[i] > 0 {
            for j in 0..n {
                matrix[i][j] = count[i][j] as f64 / from_count[i] as f64;
            }
        } else {
            matrix[i][i] = 1.0; // no change for missing class
        }
    }
    matrix
}

/// 应用转移概率到当前 LULC（单步随机模拟）。
pub fn apply_transition(current_lulc: &[u8], transition_matrix: &[Vec<f64>], seed: u64) -> Vec<u8> {
    use std::collections::hash_map::RandomState;
    let mut rng = simple_rng(seed);
    let n = transition_matrix.len();
    current_lulc
        .iter()
        .map(|&cl| {
            let ci = cl as usize;
            if ci >= n {
                return cl;
            }
            let probs = &transition_matrix[ci];
            let r: f64 = rng.next();
            let mut cum = 0.0;
            for (j, &p) in probs.iter().enumerate() {
                cum += p;
                if r < cum {
                    return j as u8;
                }
            }
            cl
        })
        .collect()
}

/// 简单伪随机数生成器（线性同余）。
struct SimpleRng {
    state: u64,
}
impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next(&mut self) -> f64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 11) as f64 / (1u64 << 53) as f64
    }
}
fn simple_rng(seed: u64) -> SimpleRng {
    SimpleRng::new(seed)
}

/// 结合邻域影响的 CA 单步（摩尔邻域 3×3）。
pub fn cellular_automata_step(
    current_lulc: &[u8],
    suitability: &[Vec<f64>], // suitability[class][cell]
    neighborhood_weight: f64,
    cols: usize,
) -> Vec<u8> {
    let n = current_lulc.len();
    let rows = n / cols;
    let n_classes = suitability.len();
    let mut result = current_lulc.to_vec();

    for i in 0..n {
        let col = i % cols;
        let row = i / cols;
        // 计算邻域同类比例
        let mut same_neighbor = 0.0;
        let mut neighbor_count = 0;
        for dr in -1..=1 {
            for dc in -1..=1 {
                if dr == 0 && dc == 0 {
                    continue;
                }
                let nr = row as isize + dr;
                let nc = col as isize + dc;
                if nr >= 0 && nr < rows as isize && nc >= 0 && nc < cols as isize {
                    let ni = (nr * cols as isize + nc) as usize;
                    if ni < n && current_lulc[ni] == current_lulc[i] {
                        same_neighbor += 1.0;
                    }
                    neighbor_count += 1;
                }
            }
        }
        let nb_ratio = if neighbor_count > 0 {
            same_neighbor / neighbor_count as f64
        } else {
            0.0
        };

        // 计算转移概率 = suitability × (1-nb_weight) + nb_ratio × nb_weight
        let cl = current_lulc[i] as usize;
        if cl >= n_classes {
            continue;
        }

        let mut scores: Vec<f64> = (0..n_classes)
            .map(|c| {
                let s = suitability
                    .get(c)
                    .and_then(|v| v.get(i))
                    .copied()
                    .unwrap_or(0.0);
                s * (1.0 - neighborhood_weight)
                    + (if c == cl { nb_ratio } else { 0.0 }) * neighborhood_weight
            })
            .collect();

        // 选择最高分（含随机扰动）
        let max_score = scores.iter().copied().fold(0.0_f64, f64::max);
        if max_score > 0.0 {
            // 归一化随机选择
            let sum: f64 = scores.iter().sum();
            if sum > 0.0 {
                let r = fast_rng(current_lulc, i) * sum;
                let mut cum = 0.0;
                for (c, &s) in scores.iter().enumerate() {
                    cum += s;
                    if r < cum {
                        result[i] = c as u8;
                        break;
                    }
                }
            }
        }
    }
    result
}

fn fast_rng(lulc: &[u8], i: usize) -> f64 {
    let h = (i as u64).wrapping_mul(2654435761)
        ^ (lulc.get(i).copied().unwrap_or(0) as u64).wrapping_mul(2246822519);
    (h % 10000) as f64 / 10000.0
}

/// 完整 CA-Markov 模拟：计算转移矩阵 → 迭代 CA 多步。
pub fn ca_markov_simulate(
    current_lulc: &[u8],
    transition_matrix: &[Vec<f64>],
    drivers: &[Vec<f64>], // driver suitability layers per class
    iterations: usize,
    neighborhood_weight: f64,
    cols: usize,
) -> CaMarkovResult {
    let mut lulc = current_lulc.to_vec();
    let mut changes_per_step = Vec::with_capacity(iterations);
    let n_classes = transition_matrix.len();

    // 用 drivers 构建 suitability 叠加 (如果没有driver, 用转移概率)
    let suitability: Vec<Vec<f64>> = if drivers.is_empty() {
        (0..n_classes)
            .map(|c| {
                current_lulc
                    .iter()
                    .map(|&cl| {
                        let ci = cl as usize;
                        if ci < n_classes {
                            transition_matrix[ci][c]
                        } else {
                            0.0
                        }
                    })
                    .collect()
            })
            .collect()
    } else {
        drivers.to_vec()
    };

    for step in 0..iterations {
        // 先 apply transition
        let seed = 42 + step as u64;
        lulc = apply_transition(&lulc, transition_matrix, seed);
        // 再 CA step
        lulc = cellular_automata_step(&lulc, &suitability, neighborhood_weight, cols);

        // 计算变化
        let changed = lulc
            .iter()
            .zip(current_lulc.iter())
            .filter(|(new, old)| new != old)
            .count();
        changes_per_step.push(changed);
    }

    let total_changed: usize = changes_per_step.iter().sum();

    // 各类面积占比
    let total = lulc.len() as f64;
    let mut class_counts = vec![0usize; n_classes.max(1)];
    for &cl in &lulc {
        let ci = cl as usize;
        if ci < class_counts.len() {
            class_counts[ci] += 1;
        }
    }
    let class_area_fractions: Vec<(u8, f64)> = class_counts
        .iter()
        .enumerate()
        .map(|(c, &cnt)| (c as u8, cnt as f64 / total))
        .collect();

    CaMarkovResult {
        final_lulc: lulc,
        changes_per_step,
        total_changed,
        class_area_fractions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transition_probability_simple() {
        let from = vec![0, 0, 1, 1, 2];
        let to = vec![0, 1, 1, 1, 2];
        let mat = transition_probability(&from, &to, 3);
        assert!((mat[0][0] - 0.5).abs() < 1e-6);
        assert!((mat[0][1] - 0.5).abs() < 1e-6);
        assert!((mat[1][1] - 1.0).abs() < 1e-6);
        assert!((mat[2][2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_apply_transition() {
        let lulc = vec![0u8, 1u8];
        let mat = vec![vec![0.3, 0.7], vec![0.0, 1.0]];
        let result = apply_transition(&lulc, &mat, 12345);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_ca_step() {
        let lulc = vec![
            0u8, 0u8, 1u8, 1u8, 0u8, 0u8, 1u8, 1u8, 0u8, 0u8, 1u8, 1u8, 0u8, 0u8, 1u8, 1u8,
        ]; // 4x4 grid
        let suitability = vec![
            vec![0.5; 16], // class 0
            vec![0.3; 16], // class 1
        ];
        let result = cellular_automata_step(&lulc, &suitability, 0.3, 4);
        assert_eq!(result.len(), 16);
    }

    #[test]
    fn test_ca_markov_simulate() {
        let lulc = vec![
            0u8, 0u8, 1u8, 1u8, 0u8, 0u8, 1u8, 1u8, 0u8, 0u8, 1u8, 1u8, 0u8, 0u8, 1u8, 1u8,
        ];
        let mat = vec![vec![0.8, 0.2], vec![0.3, 0.7]];
        let result = ca_markov_simulate(&lulc, &mat, &[], 5, 0.3, 4);
        assert_eq!(result.final_lulc.len(), 16);
        assert_eq!(result.changes_per_step.len(), 5);
    }
}
