//! K-means 空间聚类 (Lloyd 算法 + k-means++ 初始化)。
//!
//! 使用欧氏距离和 k-means++ 初始化策略获得稳定的聚类结果。

use serde::{Deserialize, Serialize};

/// K-means 聚类结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KMeansResult {
    /// k 个质心，每个质心为一个 dim 维向量
    pub centroids: Vec<Vec<f64>>,
    /// 每个数据点所属的簇标签 (0..k-1)
    pub labels: Vec<usize>,
    /// 实际迭代次数
    pub iterations: usize,
    /// 总惯性 (所有点到最近质心的距离平方和)
    pub inertia: f64,
    /// 是否收敛 (质心移位 < 1e-6)
    pub converged: bool,
}

// ── 小型 xorshift64 PRNG（无 rand 依赖） ──

struct Prng(u64);

impl Prng {
    fn new(seed: u64) -> Self {
        // 避免全零种子
        let s = if seed == 0 { 1 } else { seed };
        Self(s)
    }

    fn next_f64(&mut self) -> f64 {
        // xorshift64
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        // 映射到 [0, 1)
        (self.0 as f64) / (u64::MAX as f64)
    }
}

// ── 距离计算 ──

/// 计算两个点之间的欧氏距离。
fn euclidean(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let d = x - y;
            d * d
        })
        .sum::<f64>()
        .sqrt()
}

/// 为 k-means++ 计算点到最近质心的距离平方。
fn nearest_centroid_dist_sq(point: &[f64], centroids: &[Vec<f64>]) -> f64 {
    centroids
        .iter()
        .map(|c| {
            let d = euclidean(point, c);
            d * d
        })
        .fold(f64::MAX, f64::min)
}

// ── 公共函数 ──

/// 执行 K-means 聚类 (Lloyd 算法 + k-means++ 初始化)。
///
/// # 参数
/// * `data` — 数据点列表，每个点为 Vec<f64> (所有点维度必须一致)
/// * `k` — 聚类数量
/// * `max_iters` — 最大迭代次数
/// * `seed` — 可选的随机种子 (None = 使用当前时间)
///
/// # 返回
/// * `Some(KMeansResult)` — 聚类结果
/// * `None` — 如果输入无效 (空数据、k=0、k>点数、维度不一致)
///
/// # 算法
/// 1. k-means++ 初始化：第一个质心随机，后续以 D² 加权概率选择
/// 2. Lloyd 迭代：分配点到最近质心 → 更新质心
/// 3. 收敛条件：所有质心移位 < 1e-6
pub fn kmeans(
    data: &[Vec<f64>],
    k: usize,
    max_iters: usize,
    seed: Option<u64>,
) -> Option<KMeansResult> {
    if data.is_empty() || k == 0 || k > data.len() {
        return None;
    }

    let dim = data[0].len();
    if dim == 0 {
        return None;
    }
    // 验证所有点维度一致
    for pt in data.iter() {
        if pt.len() != dim {
            return None;
        }
    }

    let n = data.len();
    let mut rng = match seed {
        Some(s) => Prng::new(s),
        None => {
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            Prng::new(t as u64)
        }
    };

    // ── k-means++ 初始化 ──
    let mut centroids: Vec<Vec<f64>> = Vec::with_capacity(k);

    // 第一个质心：随机选择
    let first_idx = (rng.next_f64() * n as f64).floor() as usize;
    centroids.push(data[first_idx].clone());

    // 后续质心：D² 加权概率
    for _ in 1..k {
        let mut dist_sq_sum = 0.0_f64;
        let dists_sq: Vec<f64> = data
            .iter()
            .map(|pt| {
                let d2 = nearest_centroid_dist_sq(pt, &centroids);
                dist_sq_sum += d2;
                d2
            })
            .collect();

        if dist_sq_sum < f64::EPSILON {
            // 所有点都在现有质心上，直接均匀选
            let idx = (rng.next_f64() * n as f64).floor() as usize;
            centroids.push(data[idx].clone());
            continue;
        }

        let threshold = rng.next_f64() * dist_sq_sum;
        let mut cumulative = 0.0_f64;
        let mut chosen = 0;
        for (i, &d2) in dists_sq.iter().enumerate() {
            cumulative += d2;
            if cumulative >= threshold {
                chosen = i;
                break;
            }
        }
        centroids.push(data[chosen].clone());
    }

    // ── Lloyd 迭代 ──
    let mut labels = vec![0_usize; n];
    let mut converged = false;
    let mut iterations = 0;

    for iter in 0..max_iters {
        iterations = iter + 1;

        // 分配步骤：每个点分配到最近质心
        let mut changed = false;
        for (i, pt) in data.iter().enumerate() {
            let mut best_dist = f64::MAX;
            let mut best_idx = 0;
            for (j, c) in centroids.iter().enumerate() {
                let d = euclidean(pt, c);
                if d < best_dist {
                    best_dist = d;
                    best_idx = j;
                }
            }
            if labels[i] != best_idx {
                labels[i] = best_idx;
                changed = true;
            }
        }

        // 如果无点改变分配，已收敛
        if !changed {
            converged = true;
        }

        // 更新步骤：重新计算质心
        let mut new_centroids = vec![vec![0.0_f64; dim]; k];
        let mut counts = vec![0_usize; k];

        for (i, pt) in data.iter().enumerate() {
            let label = labels[i];
            counts[label] += 1;
            for d in 0..dim {
                new_centroids[label][d] += pt[d];
            }
        }

        // 处理空簇：用距全局质心最远的点重新初始化
        if counts.contains(&0) {
            // 计算全局质心
            let _global_centroid: Vec<f64> = (0..dim)
                .map(|d| data.iter().map(|pt| pt[d]).sum::<f64>() / n as f64)
                .collect();
            for j in 0..k {
                if counts[j] == 0 {
                    // 找距该质心最远的点
                    let mut farthest_dist = -1.0_f64;
                    let mut farthest_idx = 0;
                    for (i, pt) in data.iter().enumerate() {
                        let d = euclidean(pt, &new_centroids[j]); // still zeros
                        if d > farthest_dist {
                            farthest_dist = d;
                            farthest_idx = i;
                        }
                    }
                    new_centroids[j] = data[farthest_idx].clone();
                    counts[j] = 1;
                    // 这个点属于这个簇，同时从原来簇移除
                }
            }
        }

        for j in 0..k {
            if counts[j] > 0 {
                let inv = 1.0 / counts[j] as f64;
                for item in new_centroids[j].iter_mut().take(dim) {
                    *item *= inv;
                }
            }
        }

        // 检查收敛：质心移位 < 1e-6
        let mut max_shift = 0.0_f64;
        for j in 0..k {
            let shift = euclidean(&centroids[j], &new_centroids[j]);
            if shift > max_shift {
                max_shift = shift;
            }
        }
        centroids = new_centroids;

        if max_shift < 1e-6 {
            converged = true;
            break;
        }
    }

    // 计算 inertia (总平方和)
    let mut inertia = 0.0_f64;
    for (i, pt) in data.iter().enumerate() {
        let d = euclidean(pt, &centroids[labels[i]]);
        inertia += d * d;
    }

    Some(KMeansResult {
        centroids,
        labels,
        iterations,
        inertia,
        converged,
    })
}

/// 2D K-means 便捷函数：输入 x/y 坐标对，输出聚类结果。
///
/// 内部将 (x[i], y[i]) 打包为 Vec<Vec<f64>> 后调用 `kmeans()`。
pub fn kmeans_2d(
    x: &[f64],
    y: &[f64],
    k: usize,
    max_iters: usize,
    seed: Option<u64>,
) -> Option<KMeansResult> {
    if x.len() != y.len() {
        return None;
    }
    let data: Vec<Vec<f64>> = x
        .iter()
        .zip(y.iter())
        .map(|(&xi, &yi)| vec![xi, yi])
        .collect();
    kmeans(&data, k, max_iters, seed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_three_clear_clusters() {
        // 3 个明显分离的簇，用固定种子保证可复现
        let mut data = Vec::new();
        // 簇 A: (0,0) 附近
        for _ in 0..10 {
            data.push(vec![0.0, 0.0]);
        }
        // 簇 B: (10,10) 附近
        for _ in 0..10 {
            data.push(vec![10.0, 10.0]);
        }
        // 簇 C: (20,0) 附近
        for _ in 0..10 {
            data.push(vec![20.0, 0.0]);
        }

        let result = kmeans(&data, 3, 100, Some(42)).expect("kmeans failed");
        assert_eq!(result.centroids.len(), 3);
        assert_eq!(result.labels.len(), 30);
        assert!(result.converged);
        assert!(result.inertia < 1.0); // 完美簇情况下 inertia 很小
    }

    #[test]
    fn test_kmeans_2d_three_clusters() {
        let mut x = Vec::new();
        let mut y = Vec::new();
        // 簇 A
        for _ in 0..10 {
            x.push(0.0);
            y.push(0.0);
        }
        // 簇 B
        for _ in 0..10 {
            x.push(10.0);
            y.push(10.0);
        }
        // 簇 C
        for _ in 0..10 {
            x.push(20.0);
            y.push(0.0);
        }

        let result = kmeans_2d(&x, &y, 3, 100, Some(42)).expect("kmeans_2d failed");
        assert_eq!(result.centroids.len(), 3);
        assert_eq!(result.labels.len(), 30);
    }

    #[test]
    fn test_k_equals_one() {
        let data = vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]];
        let result = kmeans(&data, 1, 100, Some(0)).expect("k=1 failed");
        assert_eq!(result.centroids.len(), 1);
        assert_eq!(result.labels, vec![0, 0, 0]);
    }

    #[test]
    fn test_k_equals_data_len() {
        let data = vec![vec![1.0], vec![10.0], vec![100.0]];
        let result = kmeans(&data, 3, 100, Some(0)).expect("k=n failed");
        assert_eq!(result.centroids.len(), 3);
        // 每个点独自一簇
        let mut unique: Vec<usize> = result.labels.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 3);
    }

    #[test]
    fn test_empty_data() {
        let data: Vec<Vec<f64>> = vec![];
        assert!(kmeans(&data, 3, 100, Some(0)).is_none());
    }

    #[test]
    fn test_k_zero() {
        let data = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        assert!(kmeans(&data, 0, 100, Some(0)).is_none());
    }

    #[test]
    fn test_k_larger_than_n() {
        let data = vec![vec![1.0], vec![2.0]];
        assert!(kmeans(&data, 5, 100, Some(0)).is_none());
    }

    #[test]
    fn test_inconsistent_dimensions() {
        let data = vec![vec![1.0, 2.0], vec![3.0]]; // dim mismatch
        assert!(kmeans(&data, 2, 100, Some(0)).is_none());
    }

    #[test]
    fn test_zero_dim() {
        let data = vec![vec![], vec![]];
        assert!(kmeans(&data, 2, 100, Some(0)).is_none());
    }

    #[test]
    fn test_kmeans_2d_mismatched_length() {
        assert!(kmeans_2d(&[1.0], &[2.0, 3.0], 2, 100, Some(0)).is_none());
    }

    #[test]
    fn test_deterministic_with_seed() {
        let data = vec![
            vec![0.0, 0.0],
            vec![0.5, 0.5],
            vec![10.0, 10.0],
            vec![10.5, 10.5],
            vec![20.0, 0.0],
            vec![20.5, 0.5],
        ];
        let result_a = kmeans(&data, 3, 100, Some(12345)).expect("kmeans failed");
        let result_b = kmeans(&data, 3, 100, Some(12345)).expect("kmeans failed");
        // 相同种子应得相同结果
        assert_eq!(result_a.labels, result_b.labels);
        for (ca, cb) in result_a.centroids.iter().zip(result_b.centroids.iter()) {
            for (a, b) in ca.iter().zip(cb.iter()) {
                assert!((a - b).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_inertia_non_negative() {
        let data = vec![
            vec![1.0, 2.0],
            vec![3.0, 4.0],
            vec![5.0, 6.0],
            vec![7.0, 8.0],
        ];
        let result = kmeans(&data, 2, 100, Some(0)).expect("kmeans failed");
        assert!(result.inertia >= 0.0);
    }
}
