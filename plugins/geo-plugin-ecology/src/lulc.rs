//! 随机森林土地覆盖分类 (Random Forest LULC Classification)
//!
//! 基于 Sentinel-2 光谱指数（NDVI, NDWI, NDBI）的纯 Rust 随机森林分类器。
//! 6 类土地覆盖: 林地 / 草地 / 耕地 / 水域 / 建设用地 / 裸地

use std::fmt;

// ════════════════════════════════════════════════════════════════
//  LulcClass — 土地覆盖类型枚举
// ════════════════════════════════════════════════════════════════

/// 6 类土地覆盖类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LulcClass {
    Forest,
    Grassland,
    Cropland,
    Water,
    BuiltUp,
    Bare,
}

impl LulcClass {
    pub fn from_usize(v: usize) -> Self {
        match v {
            0 => LulcClass::Forest,
            1 => LulcClass::Grassland,
            2 => LulcClass::Cropland,
            3 => LulcClass::Water,
            4 => LulcClass::BuiltUp,
            _ => LulcClass::Bare,
        }
    }

    pub fn to_usize(self) -> usize {
        match self {
            LulcClass::Forest => 0,
            LulcClass::Grassland => 1,
            LulcClass::Cropland => 2,
            LulcClass::Water => 3,
            LulcClass::BuiltUp => 4,
            LulcClass::Bare => 5,
        }
    }

    pub const NUM_CLASSES: usize = 6;
}

impl fmt::Display for LulcClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LulcClass::Forest => write!(f, "林地"),
            LulcClass::Grassland => write!(f, "草地"),
            LulcClass::Cropland => write!(f, "耕地"),
            LulcClass::Water => write!(f, "水域"),
            LulcClass::BuiltUp => write!(f, "建设用地"),
            LulcClass::Bare => write!(f, "裸地"),
        }
    }
}

// ════════════════════════════════════════════════════════════════
//  SpectralFeatures — 光谱特征
// ════════════════════════════════════════════════════════════════

/// 用于分类的光谱指数。
#[derive(Debug, Clone, Copy)]
pub struct SpectralFeatures {
    /// 归一化植被指数 NDVI = (NIR - Red) / (NIR + Red)
    pub ndvi: f64,
    /// 归一化水体指数 NDWI = (Green - NIR) / (Green + NIR)
    pub ndwi: f64,
    /// 归一化建筑指数 NDBI = (SWIR - NIR) / (SWIR + NIR)
    pub ndbi: f64,
}

impl SpectralFeatures {
    pub fn to_vec(&self) -> Vec<f64> {
        vec![self.ndvi, self.ndwi, self.ndbi]
    }
}

// ════════════════════════════════════════════════════════════════
//  DecisionTree — 二叉决策树
// ════════════════════════════════════════════════════════════════

/// 决策树节点。
struct TreeNode {
    /// 如果是叶节点，存储类别标签。
    label: Option<usize>,
    /// 分裂特征索引（0=ndvi, 1=ndwi, 2=ndbi）。
    split_feature: usize,
    /// 分裂阈值。
    split_threshold: f64,
    /// 左子树（feature < threshold）。
    left: Option<Box<TreeNode>>,
    /// 右子树（feature >= threshold）。
    right: Option<Box<TreeNode>>,
}

/// 单棵决策树。
pub struct DecisionTree {
    root: Option<Box<TreeNode>>,
}

impl DecisionTree {
    /// 创建未训练的决策树。
    pub fn new() -> Self {
        DecisionTree { root: None }
    }

    /// 使用样本训练决策树。
    ///
    /// # Arguments
    /// * `samples` - 每行为一个样本，每列为特征
    /// * `labels` - 每行对应的类别索引
    /// * `max_depth` - 最大树深度
    pub fn train(&mut self, samples: &[Vec<f64>], labels: &[usize], max_depth: usize) {
        if samples.is_empty() {
            return;
        }
        let n_features = samples[0].len();
        self.root = Self::build_tree(samples, labels, 0, max_depth, n_features);
    }

    fn build_tree(
        samples: &[Vec<f64>],
        labels: &[usize],
        depth: usize,
        max_depth: usize,
        n_features: usize,
    ) -> Option<Box<TreeNode>> {
        if samples.is_empty() || depth >= max_depth {
            return Self::make_leaf(labels);
        }

        // 检查纯度：所有样本同类？
        let first_label = labels[0];
        let all_same = labels.iter().all(|&l| l == first_label);
        if all_same {
            return Some(Box::new(TreeNode {
                label: Some(first_label),
                split_feature: 0,
                split_threshold: 0.0,
                left: None,
                right: None,
            }));
        }

        // 搜索最佳分裂
        if let Some((best_feat, best_thresh)) = Self::best_split(samples, labels, n_features) {
            let mut left_idx: Vec<usize> = Vec::new();
            let mut right_idx: Vec<usize> = Vec::new();

            for (i, sample) in samples.iter().enumerate() {
                if sample[best_feat] < best_thresh {
                    left_idx.push(i);
                } else {
                    right_idx.push(i);
                }
            }

            // 避免无增益的分裂（一侧为空）
            if left_idx.is_empty() || right_idx.is_empty() {
                return Self::make_leaf(labels);
            }

            let left_samples: Vec<Vec<f64>> =
                left_idx.iter().map(|&i| samples[i].clone()).collect();
            let left_labels: Vec<usize> = left_idx.iter().map(|&i| labels[i]).collect();
            let right_samples: Vec<Vec<f64>> =
                right_idx.iter().map(|&i| samples[i].clone()).collect();
            let right_labels: Vec<usize> = right_idx.iter().map(|&i| labels[i]).collect();

            let left = Self::build_tree(
                &left_samples,
                &left_labels,
                depth + 1,
                max_depth,
                n_features,
            );
            let right = Self::build_tree(
                &right_samples,
                &right_labels,
                depth + 1,
                max_depth,
                n_features,
            );

            Some(Box::new(TreeNode {
                label: None,
                split_feature: best_feat,
                split_threshold: best_thresh,
                left,
                right,
            }))
        } else {
            Self::make_leaf(labels)
        }
    }

    fn make_leaf(labels: &[usize]) -> Option<Box<TreeNode>> {
        // 多数表决
        let class = majority_vote(labels).unwrap_or(0);
        Some(Box::new(TreeNode {
            label: Some(class),
            split_feature: 0,
            split_threshold: 0.0,
            left: None,
            right: None,
        }))
    }

    /// 对单个样本进行预测，返回类别索引。
    pub fn predict_one(&self, features: &[f64]) -> Option<usize> {
        let mut node = self.root.as_ref()?;
        loop {
            if let Some(label) = node.label {
                return Some(label);
            }
            if features[node.split_feature] < node.split_threshold {
                node = node.left.as_ref()?;
            } else {
                node = node.right.as_ref()?;
            }
        }
    }

    /// 找到最佳分裂（最大 Gini 增益）。
    fn best_split(
        samples: &[Vec<f64>],
        labels: &[usize],
        n_features: usize,
    ) -> Option<(usize, f64)> {
        let n = samples.len();
        if n < 2 {
            return None;
        }

        let parent_gini = gini_impurity(labels);
        let mut best_gain = f64::NEG_INFINITY;
        let mut best_split: Option<(usize, f64)> = None;

        // 尝试每个特征的随机阈值候选
        for feat in 0..n_features {
            // 获取该特征的所有唯一值
            let mut vals: Vec<f64> = samples.iter().map(|s| s[feat]).collect();
            vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            // 尝试相邻值的中点作为候选阈值
            for i in 0..vals.len().saturating_sub(1) {
                let thresh = (vals[i] + vals[i + 1]) / 2.0;
                if (vals[i + 1] - vals[i]).abs() < 1e-10 {
                    continue;
                }

                let mut left_labels: Vec<usize> = Vec::new();
                let mut right_labels: Vec<usize> = Vec::new();
                for (si, s) in samples.iter().enumerate() {
                    if s[feat] < thresh {
                        left_labels.push(labels[si]);
                    } else {
                        right_labels.push(labels[si]);
                    }
                }

                let wl = left_labels.len() as f64 / n as f64;
                let wr = right_labels.len() as f64 / n as f64;
                let gain = parent_gini
                    - wl * gini_impurity(&left_labels)
                    - wr * gini_impurity(&right_labels);

                if gain > best_gain {
                    best_gain = gain;
                    best_split = Some((feat, thresh));
                }
            }
        }

        best_split
    }
}

impl Default for DecisionTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Gini 不纯度: 1 - Σ p_i²
fn gini_impurity(labels: &[usize]) -> f64 {
    if labels.is_empty() {
        return 0.0;
    }
    let n = labels.len() as f64;
    let mut counts = [0u32; LulcClass::NUM_CLASSES];
    for &l in labels {
        if l < LulcClass::NUM_CLASSES {
            counts[l] += 1;
        }
    }
    let sum_sq: f64 = counts.iter().map(|&c| (c as f64 / n).powi(2)).sum();
    1.0 - sum_sq
}

/// 多数表决，返回最多出现的类别。
fn majority_vote(labels: &[usize]) -> Option<usize> {
    if labels.is_empty() {
        return None;
    }
    let mut counts = [0u32; LulcClass::NUM_CLASSES];
    for &l in labels {
        if l < LulcClass::NUM_CLASSES {
            counts[l] += 1;
        }
    }
    counts
        .iter()
        .enumerate()
        .max_by_key(|(_, &c)| c)
        .map(|(i, _)| i)
}

// ════════════════════════════════════════════════════════════════
//  RandomForest — 随机森林
// ════════════════════════════════════════════════════════════════

/// 随机森林分类器。
pub struct RandomForest {
    trees: Vec<DecisionTree>,
    num_classes: usize,
}

impl RandomForest {
    /// 训练随机森林。
    ///
    /// # Arguments
    /// * `samples` - 训练样本，每行一个样本
    /// * `labels` - 对应的类别标签
    /// * `num_trees` - 树的数量（默认 10）
    /// * `max_depth` - 每棵树的最大深度（默认 5）
    pub fn train(
        samples: &[Vec<f64>],
        labels: &[usize],
        num_trees: usize,
        max_depth: usize,
    ) -> Self {
        let num_classes = labels.iter().max().map(|&m| m + 1).unwrap_or(1);
        let mut trees = Vec::with_capacity(num_trees);

        for _ in 0..num_trees {
            let (bs_samples, bs_labels) = bootstrap_sample(samples, labels);
            let mut tree = DecisionTree::new();
            if !bs_samples.is_empty() {
                tree.train(&bs_samples, &bs_labels, max_depth);
            }
            trees.push(tree);
        }

        RandomForest { trees, num_classes }
    }

    /// 预测单个样本的类别及各类概率。
    ///
    /// 返回 `(majority_class_index, class_probabilities)`。
    pub fn predict(&self, features: &[f64]) -> (usize, Vec<f64>) {
        let mut votes = vec![0u32; self.num_classes];
        for tree in &self.trees {
            if let Some(class) = tree.predict_one(features) {
                if class < self.num_classes {
                    votes[class] += 1;
                }
            }
        }

        let total = votes.iter().sum::<u32>().max(1) as f64;
        let probs: Vec<f64> = votes.iter().map(|&v| v as f64 / total).collect();
        let best = votes
            .iter()
            .enumerate()
            .max_by_key(|(_, &v)| v)
            .map(|(i, _)| i)
            .unwrap_or(0);

        (best, probs)
    }

    /// 批量预测多像素。
    pub fn predict_batch(&self, features_batch: &[Vec<f64>]) -> Vec<(usize, Vec<f64>)> {
        features_batch.iter().map(|f| self.predict(f)).collect()
    }
}

/// Bootstrap 采样（有放回）。
fn bootstrap_sample(samples: &[Vec<f64>], labels: &[usize]) -> (Vec<Vec<f64>>, Vec<usize>) {
    let n = samples.len();
    if n == 0 {
        return (vec![], vec![]);
    }

    // 简单确定性采样：循环取模 + 偏移，模拟 bootstrap
    let mut bs_samples = Vec::with_capacity(n);
    let mut bs_labels = Vec::with_capacity(n);
    for i in 0..n {
        let idx = (i * 3 + 7) % n;
        bs_samples.push(samples[idx].clone());
        bs_labels.push(labels[idx]);
    }
    (bs_samples, bs_labels)
}

// ════════════════════════════════════════════════════════════════
//  TypicaL spectral centroids — 用于默认模型训练
// ════════════════════════════════════════════════════════════════

/// 典型光谱指数中心（NDVI, NDWI, NDBI）。
const CLASS_CENTROIDS: [(f64, f64, f64); 6] = [
    (0.70, -0.10, -0.40), // 林地 Forest
    (0.40, -0.05, -0.20), // 草地 Grassland
    (0.50, 0.00, -0.10),  // 耕地 Cropland
    (-0.30, 0.50, -0.50), // 水域 Water
    (0.10, -0.20, 0.30),  // 建设用地 BuiltUp
    (0.05, -0.40, -0.10), // 裸地 Bare
];

/// 生成默认训练数据（每个类别 100 个带噪声样本）。
pub fn default_training_data(per_class: usize) -> (Vec<Vec<f64>>, Vec<usize>) {
    let mut samples = Vec::with_capacity(per_class * 6);
    let mut labels = Vec::with_capacity(per_class * 6);

    for (class, &(c_ndvi, c_ndwi, c_ndbi)) in CLASS_CENTROIDS.iter().enumerate() {
        for i in 0..per_class {
            // 确定性"噪声"：使用正弦调制避免随机种子
            let noise = (i as f64 * 0.07).sin();
            let ndvi = (c_ndvi + noise * 0.15).clamp(-1.0, 1.0);
            let ndwi = (c_ndwi + noise * 0.15).clamp(-1.0, 1.0);
            let ndbi = (c_ndbi + noise * 0.15).clamp(-1.0, 1.0);
            samples.push(vec![ndvi, ndwi, ndbi]);
            labels.push(class);
        }
    }

    (samples, labels)
}

/// 训练一个默认的随机森林模型（使用合成训练数据）。
pub fn default_model() -> RandomForest {
    let (samples, labels) = default_training_data(100);
    RandomForest::train(&samples, &labels, 10, 5)
}

// ════════════════════════════════════════════════════════════════
//  测试
// ════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lulc_class_display() {
        assert_eq!(LulcClass::Forest.to_string(), "林地");
        assert_eq!(LulcClass::Grassland.to_string(), "草地");
        assert_eq!(LulcClass::Cropland.to_string(), "耕地");
        assert_eq!(LulcClass::Water.to_string(), "水域");
        assert_eq!(LulcClass::BuiltUp.to_string(), "建设用地");
        assert_eq!(LulcClass::Bare.to_string(), "裸地");
    }

    #[test]
    fn test_lulc_class_roundtrip() {
        for i in 0..6 {
            let cls = LulcClass::from_usize(i);
            assert_eq!(cls.to_usize(), i);
        }
    }

    #[test]
    fn test_gini_pure() {
        // 纯节点：Gini = 0
        assert!((gini_impurity(&[0, 0, 0]) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_gini_impure() {
        // 二分类均匀：Gini = 1 - 2*(0.5²) = 0.5
        assert!((gini_impurity(&[0, 1, 0, 1]) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_majority_vote() {
        assert_eq!(majority_vote(&[0, 1, 0]), Some(0));
        assert_eq!(majority_vote(&[1, 1, 2, 2, 1]), Some(1));
        assert_eq!(majority_vote(&[]), None);
    }

    #[test]
    fn test_decision_tree_overfit_small() {
        // 简单可分离数据：训练决策树应完美拟合
        let samples = vec![
            vec![0.8, -0.2, -0.5], // Forest-like
            vec![0.8, -0.2, -0.5],
            vec![0.1, -0.3, 0.4], // BuiltUp-like
            vec![0.1, -0.3, 0.4],
        ];
        let labels = vec![0, 0, 4, 4];

        let mut tree = DecisionTree::new();
        tree.train(&samples, &labels, 5);

        // 应该正确分类训练样本
        assert_eq!(tree.predict_one(&[0.8, -0.2, -0.5]), Some(0));
        assert_eq!(tree.predict_one(&[0.1, -0.3, 0.4]), Some(4));
    }

    #[test]
    fn test_random_forest_default() {
        let model = default_model();
        assert_eq!(model.trees.len(), 10);

        // 林地像元应被分类为林地
        let (class, _) = model.predict(&[0.7, -0.1, -0.4]);
        assert_eq!(class, 0); // Forest

        // 水域像元
        let (class, _) = model.predict(&[-0.3, 0.5, -0.5]);
        assert_eq!(class, 3); // Water
    }

    #[test]
    fn test_random_forest_probabilities() {
        let model = default_model();
        let (class, probs) = model.predict(&[0.7, -0.1, -0.4]);
        assert_eq!(probs.len(), 6);
        // 概率应在 [0,1] 且和为 1
        let sum: f64 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
        // 最大概率应指向 class
        let max_prob = probs.iter().cloned().fold(0.0_f64, f64::max);
        assert!((probs[class] - max_prob).abs() < 1e-10);
    }

    #[test]
    fn test_default_model_all_classes() {
        let model = default_model();
        // 测试 6 个类别的典型光谱
        let test_cases: [(f64, f64, f64, usize); 6] = [
            (0.70, -0.10, -0.40, 0), // Forest
            (0.40, -0.05, -0.20, 1), // Grassland
            (0.50, 0.00, -0.10, 2),  // Cropland
            (-0.30, 0.50, -0.50, 3), // Water
            (0.10, -0.20, 0.30, 4),  // BuiltUp
            (0.05, -0.40, -0.10, 5), // Bare
        ];

        let mut correct = 0;
        for (ndvi, ndwi, ndbi, expected) in &test_cases {
            let (class, _) = model.predict(&[*ndvi, *ndwi, *ndbi]);
            if class == *expected {
                correct += 1;
            }
        }
        // 至少大部分正确
        assert!(correct >= 4, "expected at least 4 correct, got {correct}");
    }

    #[test]
    fn test_predict_batch() {
        let model = default_model();
        let batch = vec![
            vec![0.7, -0.1, -0.4],
            vec![-0.3, 0.5, -0.5],
            vec![0.1, -0.2, 0.3],
        ];
        let results = model.predict_batch(&batch);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].1.len(), 6);
    }

    #[test]
    fn test_spectral_features_to_vec() {
        let sf = SpectralFeatures {
            ndvi: 0.5,
            ndwi: -0.1,
            ndbi: 0.2,
        };
        assert_eq!(sf.to_vec(), vec![0.5, -0.1, 0.2]);
    }

    #[test]
    fn test_bootstrap_reproducible() {
        let samples = vec![vec![0.1], vec![0.2], vec![0.3]];
        let labels = vec![0, 1, 2];
        let (bs1, bl1) = bootstrap_sample(&samples, &labels);
        let (bs2, bl2) = bootstrap_sample(&samples, &labels);
        // Deterministic bootstrap should be reproducible
        assert_eq!(bs1.len(), bs2.len());
        assert_eq!(bl1, bl2);
    }
}
