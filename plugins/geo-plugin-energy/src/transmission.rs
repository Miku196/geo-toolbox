//! 输电走廊最小成本路径 (LCP) 模块。
//!
//! ## 方法
//!
//! 基于多因子成本面 (cost surface)，使用简化 Dijkstra 寻找从起点到终点的
//! 最小累积成本路径。走廊宽度由两侧缓冲距离确定。
//!
//! ## 成本因子 (归一化到 0-100)
//!
//! | 因子 | 低成本 | 高成本 |
//! |------|--------|--------|
//! | 坡度 (°) | <10 | >30 |
//! | 距保护区 (m) | >5000 | 0 |
//! | 距居民区 (m) | >1000 | 0 |
//! | 土地利用 | 裸地/草地 | 建成区/水体 |
//! | 高程变异性 (std) | <50m | >200m |
//!
//! LCP 累加成本 = Σ(cost_i × cell_distance)，走廊 = buffer(lcp, width/2)

use serde::Serialize;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// 成本因子配置。
#[derive(Debug, Clone, Copy)]
pub struct CostFactors {
    /// 坡度权重 (default: 0.3)
    pub slope_weight: f64,
    /// 保护区距离权重 (default: 0.2)
    pub protected_area_weight: f64,
    /// 居民区距离权重 (default: 0.25)
    pub residential_weight: f64,
    /// 土地利用权重 (default: 0.15)
    pub land_use_weight: f64,
    /// 高程变异性权重 (default: 0.1)
    pub elevation_std_weight: f64,
}

impl Default for CostFactors {
    fn default() -> Self {
        Self {
            slope_weight: 0.3,
            protected_area_weight: 0.2,
            residential_weight: 0.25,
            land_use_weight: 0.15,
            elevation_std_weight: 0.1,
        }
    }
}

/// 土地利用成本 (0-100)。
/// 低值 = 低成本, 高值 = 高成本。
pub fn land_use_cost(lu_class: u8) -> f64 {
    match lu_class {
        0 => 10.0,  // 裸地
        1 => 15.0,  // 草地
        2 => 20.0,  // 灌木
        3 => 25.0,  // 耕地
        4 => 60.0,  // 森林
        5 => 90.0,  // 建成区
        6 => 100.0, // 水体/湿地
        _ => 50.0,  // 未知
    }
}

/// 坡度成本 (0-100)。
pub fn slope_cost(slope_deg: f64) -> f64 {
    if slope_deg < 5.0 {
        5.0
    } else if slope_deg < 10.0 {
        slope_deg * 1.5
    } else if slope_deg < 20.0 {
        slope_deg * 2.0
    } else if slope_deg < 30.0 {
        slope_deg * 3.0
    } else {
        100.0
    } // > 30° 不适宜
}

/// 距离衰减成本。
/// 距离越近成本越高。
pub fn proximity_cost(distance_m: f64, threshold_m: f64) -> f64 {
    if distance_m >= threshold_m {
        0.0
    } else {
        (1.0 - distance_m / threshold_m) * 100.0
    }
}

/// 综合成本面 (cols × rows, 单通道)。
pub fn build_cost_surface(
    slope: &[f64],
    lu_class: &[u8],
    dist_to_protected: &[f64],
    dist_to_residential: &[f64],
    factors: &CostFactors,
) -> Vec<f64> {
    let n = slope
        .len()
        .min(lu_class.len())
        .min(dist_to_protected.len())
        .min(dist_to_residential.len());

    let mut cost = vec![0.0; n];
    for i in 0..n {
        let s = slope_cost(slope[i]);
        let l = land_use_cost(lu_class[i]);
        let p = proximity_cost(dist_to_protected[i], 5000.0);
        let r = proximity_cost(dist_to_residential[i], 1000.0);

        // elevation variability handled per-cell (would need neighborhood)
        let e_std = 0.0;

        cost[i] = s * factors.slope_weight
            + l * factors.land_use_weight
            + p * factors.protected_area_weight
            + r * factors.residential_weight
            + e_std * factors.elevation_std_weight;
    }
    cost
}

/// 网格节点状态 (Dijkstra)。
#[derive(Clone)]
struct DijkstraNode {
    idx: usize,
    cost: f64,
}

impl PartialEq for DijkstraNode {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
    }
}

impl Eq for DijkstraNode {}

impl Ord for DijkstraNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap 是最大堆，取反实现最小堆
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for DijkstraNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// 输电走廊评估结果。
#[derive(Debug, Clone, Serialize)]
pub struct TransmissionCorridor {
    /// 名称
    pub name: String,
    /// 起点名
    pub source_name: String,
    /// 终点名
    pub sink_name: String,
    /// 路径格点索引序列
    pub path_indices: Vec<usize>,
    /// 路径长度 (km)
    pub path_length_km: f64,
    /// 累积成本
    pub cumulative_cost: f64,
    /// 平均成本
    pub mean_cost: f64,
    /// 最大成本
    pub max_cost: f64,
    /// 走廊宽度 (m)
    pub corridor_width_m: f64,
    /// 走廊面积 (km²)
    pub corridor_area_km2: f64,
    /// 总成本 (USD) 估算
    pub estimated_cost_usd: f64,
}

/// 最小成本路径搜索。
///
/// 使用 Dijkstra 在成本面上从 start_idx 到 end_idx 搜索。
///
/// # Arguments
/// * `cost_surface` - 综合成本面 (flat array, 长度 = nrows × ncols)
/// * `nrows` - 行数
/// * `ncols` - 列数
/// * `start_idx` - 起点 flat 索引
/// * `end_idx` - 终点 flat 索引
/// * `cell_size_m` - 格网边长 (m)
pub fn least_cost_path(
    cost_surface: &[f64],
    nrows: usize,
    ncols: usize,
    start_idx: usize,
    end_idx: usize,
    cell_size_m: f64,
) -> Option<(Vec<usize>, f64)> {
    let n = nrows * ncols;
    if start_idx >= n || end_idx >= n || n == 0 {
        return None;
    }

    let mut dist = vec![f64::INFINITY; n];
    let mut prev = vec![usize::MAX; n];
    let mut heap = BinaryHeap::new();

    dist[start_idx] = 0.0;
    heap.push(DijkstraNode {
        idx: start_idx,
        cost: 0.0,
    });

    let diag_cost = (2.0_f64).sqrt() * cell_size_m; // 对角距离 ~1.414 × cell

    while let Some(node) = heap.pop() {
        let u = node.idx;
        if u == end_idx {
            break;
        }
        if node.cost > dist[u] {
            continue; // 过期节点
        }

        let row = u / ncols;
        let col = u % ncols;

        // 8 邻域
        const DR: [isize; 8] = [-1, -1, -1, 0, 0, 1, 1, 1];
        const DC: [isize; 8] = [-1, 0, 1, -1, 1, -1, 0, 1];
        const DIAG: [bool; 8] = [true, false, true, false, false, true, false, true];

        for d in 0..8 {
            let nr = row as isize + DR[d];
            let nc = col as isize + DC[d];
            if nr < 0 || nr >= nrows as isize || nc < 0 || nc >= ncols as isize {
                continue;
            }
            let v = (nr as usize) * ncols + (nc as usize);

            // 边权重 = (cost_u + cost_v)/2 × 距离
            let edge_dist = if DIAG[d] { diag_cost } else { cell_size_m };
            let avg_cost = (cost_surface[u] + cost_surface[v]) / 200.0; // 归一化到 0-1
            let weight = edge_dist * (1.0 + avg_cost * 5.0); // 成本加权

            let new_cost = dist[u] + weight;
            if new_cost < dist[v] {
                dist[v] = new_cost;
                prev[v] = u;
                heap.push(DijkstraNode {
                    idx: v,
                    cost: new_cost,
                });
            }
        }
    }

    // 回溯路径
    if !dist[end_idx].is_finite() {
        return None; // 不可达
    }

    let mut path = Vec::new();
    let mut cur = end_idx;
    while cur != usize::MAX {
        path.push(cur);
        if cur == start_idx {
            break;
        }
        cur = prev[cur];
    }
    path.reverse();

    if path.is_empty() || path[0] != start_idx {
        return None;
    }

    let path_len_km = dist[end_idx] / 1000.0;
    Some((path, path_len_km))
}

/// 完整输电走廊评估。
#[allow(clippy::too_many_arguments)]
pub fn assess_corridor(
    name: &str,
    source_name: &str,
    sink_name: &str,
    cost_surface: &[f64],
    nrows: usize,
    ncols: usize,
    start_idx: usize,
    end_idx: usize,
    cell_size_m: f64,
    corridor_width_m: f64,
    cost_per_km_usd: f64,
) -> Option<TransmissionCorridor> {
    let (path, length_km) =
        least_cost_path(cost_surface, nrows, ncols, start_idx, end_idx, cell_size_m)?;

    let cum: f64 = path.iter().map(|&idx| cost_surface[idx]).sum();
    let mean = if !path.is_empty() {
        cum / path.len() as f64
    } else {
        0.0
    };
    let max = path
        .iter()
        .map(|&idx| cost_surface[idx])
        .fold(0.0, f64::max);

    let area_km2 = length_km * corridor_width_m / 1000.0;
    let est_cost = length_km * cost_per_km_usd;

    Some(TransmissionCorridor {
        name: name.to_string(),
        source_name: source_name.to_string(),
        sink_name: sink_name.to_string(),
        path_indices: path,
        path_length_km: length_km,
        cumulative_cost: cum,
        mean_cost: mean,
        max_cost: max,
        corridor_width_m,
        corridor_area_km2: area_km2,
        estimated_cost_usd: est_cost,
    })
}

/// 默认输电成本 (USD/km)，基于中国 500kV 交流线路。
pub const DEFAULT_COST_PER_KM: f64 = 1_200_000.0;

#[cfg(test)]
mod tests {
    use super::*;

    /// 平坦、均质面——LCP 应走对角线，路径长度为成本加权距离。
    #[test]
    fn test_flat_homogeneous() {
        let nrows = 5;
        let ncols = 5;
        let cost = vec![50.0; nrows * ncols];
        let (path, len) = least_cost_path(&cost, nrows, ncols, 0, 24, 1000.0).unwrap();
        assert!(!path.is_empty());
        // 对角线 (0,0)→(1,1)→(2,2)→(3,3)→(4,4), 4 步。
        // 每步权重 = sqrt(2)*1000 * (1 + (50+50)/200*5) = 1414.21 * 3.5 = 4949.75
        // 总长 = 4 * 4949.75 / 1000 = 19.799 km
        let expected = 4.0 * (2.0_f64).sqrt() * (1.0 + 0.5 * 5.0);
        assert!(
            (len - expected).abs() < 0.01,
            "len={len}, expected={expected}"
        );
    }

    /// 单个高成本格点迫使 LCP 绕行。
    #[test]
    fn test_barrier_deviation() {
        let nrows = 5;
        let ncols = 7;
        // 单个高成本格点 (0,2)，路径可绕行
        let mut cost = vec![50.0; nrows * ncols];
        cost[2] = 5000.0;

        // start (0,0)=idx0, end (0,6)=idx6
        let (path, len) = least_cost_path(&cost, nrows, ncols, 0, 6, 1000.0).unwrap();
        assert!(!path.is_empty());
        // 应绕过高成本格点
        assert!(
            !path.contains(&2),
            "Path should avoid expensive cell at index 2. Got: {path:?}"
        );
        assert!(
            len > 6.0,
            "Path must be longer than direct route. len={len}"
        );
    }

    #[test]
    fn test_slope_cost_values() {
        assert_eq!(slope_cost(3.0), 5.0);
        assert!(slope_cost(15.0) > 20.0);
        assert_eq!(slope_cost(35.0), 100.0);
    }

    #[test]
    fn test_proximity_cost() {
        assert!(proximity_cost(0.0, 1000.0) > 95.0);
        assert!((proximity_cost(500.0, 1000.0) - 50.0).abs() < 1.0);
        assert_eq!(proximity_cost(2000.0, 1000.0), 0.0);
    }

    #[test]
    fn test_full_corridor() {
        let nrows = 4;
        let ncols = 4;
        let cost = vec![20.0; 16];
        let result = assess_corridor(
            "test_corridor",
            "WindFarm",
            "Substation",
            &cost,
            nrows,
            ncols,
            0,
            15,
            1000.0,
            100.0,
            1_000_000.0,
        )
        .unwrap();
        assert!(!result.path_indices.is_empty());
        assert!(result.path_length_km > 0.0);
        assert!(result.corridor_area_km2 > 0.0);
        assert!(result.estimated_cost_usd > 0.0);
    }
}
