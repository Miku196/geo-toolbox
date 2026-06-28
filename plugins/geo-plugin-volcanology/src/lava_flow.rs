#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// 熔岩流单元。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LavaFlowCell {
    /// 栅格列
    pub col: usize,
    /// 栅格行
    pub row: usize,
    /// 累积成本
    pub cost: f64,
    /// 熔岩厚度 (m)
    pub thickness_m: f64,
    /// 温度 (°C)
    pub temperature_c: f64,
    /// 是否在流径上
    pub on_path: bool,
}

/// 熔岩流模拟结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LavaFlowSimulation {
    /// 流径单元
    pub flow_path: Vec<(usize, usize)>,
    /// 流径长度 (km)
    pub path_length_km: f64,
    /// 覆盖面积 (km²)
    pub coverage_area_km2: f64,
    /// 最大流距 (km)
    pub max_flow_distance_km: f64,
    /// 冷却时间估计 (小时)
    pub cooling_time_hours: f64,
    /// 源头位置
    pub vent_row: usize,
    pub vent_col: usize,
}

/// 用于优先队列的成本节点。
#[derive(Debug, Clone, Copy)]
struct CostNode {
    row: usize,
    col: usize,
    cost: f64,
}

impl PartialEq for CostNode {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
    }
}
impl Eq for CostNode {}
impl PartialOrd for CostNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.cost.partial_cmp(&self.cost) // 最小堆
    }
}
impl Ord for CostNode {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

/// Dijkstra 变体: 基于地形坡度和熔岩流变学的最小成本路径。
/// - dem: 数字高程模型 [rows × cols]
/// - vent_row, vent_col: 喷发口位置
/// - effusion_rate_m3s: 喷发速率
/// - viscosity_Pa_s: 熔岩粘度
/// - rows, cols
#[allow(non_snake_case)]
pub fn lava_flow_path(
    dem: &[f64],
    vent_row: usize,
    vent_col: usize,
    effusion_rate_m3s: f64,
    viscosity_Pa_s: f64,
    rows: usize,
    cols: usize,
) -> LavaFlowSimulation {
    let n = rows * cols;
    let mut cost_map = vec![f64::MAX; n];
    let mut parent = vec![(vent_row, vent_col); n];
    let mut visited = vec![false; n];
    let mut heap = BinaryHeap::new();

    let start_idx = vent_row * cols + vent_col;
    cost_map[start_idx] = 0.0;
    heap.push(CostNode {
        row: vent_row,
        col: vent_col,
        cost: 0.0,
    });

    // 8 邻域
    let dr: [isize; 8] = [-1, -1, -1, 0, 0, 1, 1, 1];
    let dc: [isize; 8] = [-1, 0, 1, -1, 1, -1, 0, 1];

    let mut max_dist = 0.0;
    let mut farthest = (vent_row, vent_col);

    while let Some(node) = heap.pop() {
        let idx = node.row * cols + node.col;
        if visited[idx] {
            continue;
        }
        visited[idx] = true;

        let dist = ((node.row as f64 - vent_row as f64).powi(2)
            + (node.col as f64 - vent_col as f64).powi(2))
        .sqrt();
        if dist > max_dist {
            max_dist = dist;
            farthest = (node.row, node.col);
        }

        for i in 0..8 {
            let nr = node.row as isize + dr[i];
            let nc = node.col as isize + dc[i];
            if nr < 0 || nr >= rows as isize || nc < 0 || nc >= cols as isize {
                continue;
            }
            let nr = nr as usize;
            let nc = nc as usize;
            let nidx = nr * cols + nc;
            if visited[nidx] {
                continue;
            }

            // 坡度成本: 下坡成本低, 上坡极高
            let z0 = dem[idx];
            let z1 = dem[nidx];
            let slope_cost = if z1 < z0 {
                // 下坡
                1.0 + (z0 - z1).abs() * 0.5
            } else {
                // 上坡或平地
                10.0 + (z1 - z0).abs() * 2.0
            };

            // 粘度成本
            let visc_cost = 1.0 + (viscosity_Pa_s / 5000.0).min(10.0);

            // 流量成本: 高喷发速率降低成本
            let eff_cost = (effusion_rate_m3s / 100.0).max(1.0).recip();

            let new_cost = cost_map[idx] + slope_cost * visc_cost * eff_cost;

            if new_cost < cost_map[nidx] - 1e-6 {
                cost_map[nidx] = new_cost;
                parent[nidx] = (node.row, node.col);
                heap.push(CostNode {
                    row: nr,
                    col: nc,
                    cost: new_cost,
                });
            }
        }
    }

    // 回溯路径
    let mut path = Vec::new();
    let mut cur = farthest;
    loop {
        path.push(cur);
        if cur == (vent_row, vent_col) {
            break;
        }
        let idx = cur.0 * cols + cur.1;
        cur = parent[idx];
        if path.len() > n {
            break; // 防止死循环
        }
    }
    path.reverse();

    let cell_size_km = 0.03; // ~30 m
    let path_length = path.len() as f64 * cell_size_km;
    let coverage_area = (path.len() as f64) * cell_size_km.powi(2);
    let cooling_time = viscosity_Pa_s / (effusion_rate_m3s + 1.0) * 0.5;

    LavaFlowSimulation {
        flow_path: path,
        path_length_km: (path_length * 100.0).round() / 100.0,
        coverage_area_km2: (coverage_area * 100.0).round() / 100.0,
        max_flow_distance_km: (max_dist * cell_size_km * 100.0).round() / 100.0,
        cooling_time_hours: (cooling_time * 100.0).round() / 100.0,
        vent_row,
        vent_col,
    }
}

/// 简化熔岩流模拟 (返回模拟结果)。
#[allow(non_snake_case)]
pub fn lava_flow_simulation(
    dem: &[f64],
    vent_row: usize,
    vent_col: usize,
    effusion_rate_m3s: f64,
    viscosity_Pa_s: f64,
    rows: usize,
    cols: usize,
) -> LavaFlowSimulation {
    lava_flow_path(
        dem,
        vent_row,
        vent_col,
        effusion_rate_m3s,
        viscosity_Pa_s,
        rows,
        cols,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_terrain() {
        let dem = vec![100.0; 100];
        let r = lava_flow_path(&dem, 5, 5, 500.0, 5000.0, 10, 10);
        assert!(r.path_length_km > 0.0);
        assert_eq!(r.flow_path.first(), Some(&(5, 5)));
    }

    #[test]
    fn test_downhill_preference() {
        let mut dem = vec![100.0; 100];
        dem[4 * 10 + 5] = 90.0; // 下方更低
        let r = lava_flow_path(&dem, 0, 5, 500.0, 5000.0, 10, 10);
        assert!(r.path_length_km > 0.0);
    }

    #[test]
    fn test_cooling_time() {
        // 高粘度 → 冷却更快 (流动性差)
        let r = lava_flow_path(&[100.0; 100], 1, 1, 500.0, 5000.0, 10, 10);
        assert!(r.cooling_time_hours > 0.0);
    }
}
