/// 可达性 / 市场潜力模块 — 出行时间、市场潜力。
use serde::{Deserialize, Serialize};

/// 可达性结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityResult {
    /// 到最近城市的出行时间 (minutes)
    pub travel_time_min: Vec<Option<f64>>,
    /// 可达性得分 (市场潜力)
    pub accessibility_score: Vec<f64>,
    /// 平均出行时间 (可由区域中非 None 的值计算)
    pub mean_travel_time: Option<f64>,
}

/// 计算到最近城市的出行时间（基于成本面栅格的累积成本）。
/// `origin` — 起点索引
/// `cost_surface` — 每栅格通行成本 (如 travel_time per cell)
/// `max_cost` — 最大搜索成本（超出=不可达）
/// `cols` — 栅格列数
pub fn travel_time_to_city(
    origin: usize,
    cost_surface: &[f64],
    max_cost: f64,
    cols: usize,
) -> Vec<Option<f64>> {
    let n = cost_surface.len();
    let rows = n / cols;
    if rows == 0 || origin >= n {
        return vec![None; n];
    }
    // Dijkstra 简单实现（适合小栅格）
    let mut dist = vec![f64::MAX; n];
    let mut visited = vec![false; n];
    dist[origin] = 0.0;

    for _ in 0..n {
        // 找未访问的最小距离节点
        let mut u = None;
        let mut min_d = f64::MAX;
        for i in 0..n {
            if !visited[i] && dist[i] < min_d {
                min_d = dist[i];
                u = Some(i);
            }
        }
        let u = match u { Some(u) => u, None => break };
        if min_d >= max_cost { break; }
        visited[u] = true;

        // 4-邻域
        let u_col = u % cols;
        let u_row = u / cols;
        for (dr, dc) in &[(0isize, 1), (0, -1), (1, 0), (-1, 0)] {
            let nr = u_row as isize + dr;
            let nc = u_col as isize + dc;
            if nr >= 0 && nr < rows as isize && nc >= 0 && nc < cols as isize {
                let v = (nr * cols as isize + nc) as usize;
                if !visited[v] {
                    let new_d = dist[u] + cost_surface[v];
                    if new_d < dist[v] {
                        dist[v] = new_d;
                    }
                }
            }
        }
    }

    dist.iter().map(|&d| {
        if d >= max_cost { None } else { Some(d) }
    }).collect()
}

/// 市场潜力（重力模型）:
/// potential_i = sum_j(P_j / (travel_time_ij^beta))
/// 简化版: 给定到每个目的点的出行时间列表
pub fn market_potential(
    population: &[f64],
    travel_time: &[f64],
    decay_parameter: f64,
) -> f64 {
    population.iter().zip(travel_time.iter()).map(|(&pop, &tt)| {
        if tt > 0.0 {
            pop / tt.powf(decay_parameter)
        } else if tt == 0.0 {
            pop * 10.0 // 自身市场 = 人口×10
        } else {
            0.0
        }
    }).sum()
}

/// 多起点可达性（如到多个城市的出行时间取最小值）。
pub fn multi_city_accessibility(
    origins: &[usize],
    cost_surface: &[f64],
    max_cost: f64,
    cols: usize,
    decay: f64,
    city_populations: &[f64],
) -> AccessibilityResult {
    let n = cost_surface.len();
    let mut min_travel = vec![None; n];
    for (i, &origin) in origins.iter().enumerate() {
        let tt = travel_time_to_city(origin, cost_surface, max_cost, cols);
        // 只优化起点自身及周围的出行时间
        let pop = city_populations.get(i).copied().unwrap_or(1.0);
        for j in 0..n {
            if let Some(t) = tt[j] {
                match min_travel[j] {
                    Some(cur) if t < cur => min_travel[j] = Some(t),
                    None => min_travel[j] = Some(t),
                    _ => {}
                }
            }
        }
    }

    // 市场潜力 = sum over destinations
    let scores: Vec<f64> = (0..n).map(|i| {
        let mut potential = 0.0;
        for (j, &origin) in origins.iter().enumerate() {
            let tt = travel_time_to_city(origin, cost_surface, max_cost, cols);
            let pop = city_populations.get(j).copied().unwrap_or(1.0);
            if let Some(t) = tt.get(i).copied().flatten() {
                potential += if t > 0.0 { pop / t.powf(decay) } else { pop * 10.0 };
            }
        }
        potential
    }).collect();

    let valid_tt: Vec<f64> = min_travel.iter().filter_map(|&t| t).collect();
    let mean_tt = if valid_tt.is_empty() { None } else {
        Some(valid_tt.iter().sum::<f64>() / valid_tt.len() as f64)
    };

    AccessibilityResult {
        travel_time_min: min_travel,
        accessibility_score: scores,
        mean_travel_time: mean_tt,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_travel_time_simple() {
        // 3x3 uniform cost = 1 per cell
        let cost = vec![1.0; 9];
        let result = travel_time_to_city(0, &cost, 10.0, 3);
        assert!(result[0].unwrap() < 0.01); // origin = 0
        assert!(result[8].unwrap() > 3.0); // diagonal = 4 steps
    }

    #[test]
    fn test_travel_time_unreachable() {
        let cost = vec![1.0, 1000.0, 1.0, 1.0];
        let result = travel_time_to_city(0, &cost, 5.0, 2);
        assert!(result[0].is_some());
        assert!(result[1].is_none()); // cost too high
    }

    #[test]
    fn test_market_potential() {
        let pop = vec![1000.0, 2000.0];
        let tt = vec![10.0, 20.0];
        let potential = market_potential(&pop, &tt, 0.5);
        assert!(potential > 0.0);
        // origin pop contribution
        let expected = 1000.0 / 10.0_f64.powf(0.5) + 2000.0 / 20.0_f64.powf(0.5);
        assert!((potential - expected).abs() < 0.1);
    }

    #[test]
    fn test_multi_city_accessibility() {
        let cost = vec![1.0; 16];
        let result = multi_city_accessibility(
            &[0, 15], &cost, 10.0, 4, 0.5,
            &[5000.0, 3000.0],
        );
        assert_eq!(result.travel_time_min.len(), 16);
        assert!(result.mean_travel_time.unwrap() > 0.0);
        assert!(!result.accessibility_score.is_empty());
    }
}
