//! 流域提取 — D8 流向 + 汇水区逆追溯
//!
//! Given a D8 flow direction grid and a pour-point, extracts
//! all upstream cells that contribute flow to that point via
//! BFS traversal of the reverse flow-direction graph.

use serde::{Deserialize, Serialize};

/// D8 流向 → (row_offset, col_offset)
/// 方向编码: 0=E, 1=SE, 2=S, 3=SW, 4=W, 5=NW, 6=N, 7=NE
const D8_DR: [isize; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
const D8_DC: [isize; 8] = [1, 1, 0, -1, -1, -1, 0, 1];

/// 流域提取结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatershedResult {
    /// 流域内格点索引（flat index）。
    pub cells: Vec<usize>,
    /// 流域格点数。
    pub num_cells: usize,
    /// 布尔遮罩（true = 在流域内）。
    pub mask: Vec<bool>,
    /// 集水面积 (ha)。
    pub area_ha: f64,
}

// ─── 辅助函数 ──────────────────────────────────────────────

/// 获取方向 d 的下游邻居 (dr, dc)。
pub fn d8_offset(dir: usize) -> (isize, isize) {
    if dir < 8 {
        (D8_DR[dir], D8_DC[dir])
    } else {
        (0, 0)
    }
}

// ─── 核心：汇水区提取 ──────────────────────────────────────

/// 从 D8 流向栅格提取汇水区（上游集水格点）。
///
/// # Arguments
/// * `flow_dir` - 每个格点的流向（0-7 或 None=Sink/Edge）
/// * `nrows` - 行数
/// * `ncols` - 列数
/// * `pour_row` - 出口行号（0-based）
/// * `pour_col` - 出口列号（0-based）
/// * `cell_size_m` - 格网边长 (m)，用于计算面积
///
/// # Returns
/// 所有流入出口点的格点集合。
pub fn extract_watershed(
    flow_dir: &[Option<usize>],
    nrows: usize,
    ncols: usize,
    pour_row: usize,
    pour_col: usize,
    cell_size_m: f64,
) -> WatershedResult {
    let n = nrows * ncols;
    if flow_dir.len() < n || pour_row >= nrows || pour_col >= ncols {
        return WatershedResult {
            cells: vec![],
            num_cells: 0,
            mask: vec![false; n],
            area_ha: 0.0,
        };
    }

    // 构建逆流向图：每个格点列出其上游邻居
    let mut upstream: Vec<Vec<usize>> = vec![Vec::new(); n];

    for r in 0..nrows {
        for c in 0..ncols {
            let idx = r * ncols + c;
            if let Some(dir) = flow_dir[idx] {
                if dir < 8 {
                    let nr = r as isize + D8_DR[dir];
                    let nc = c as isize + D8_DC[dir];
                    if nr >= 0 && nr < nrows as isize && nc >= 0 && nc < ncols as isize {
                        let nidx = (nr as usize) * ncols + (nc as usize);
                        upstream[nidx].push(idx);
                    }
                }
            }
        }
    }

    // BFS 从出口逆流而上
    let start = pour_row * ncols + pour_col;
    let mut cells: Vec<usize> = vec![start];
    let mut queue: Vec<usize> = vec![start];
    let mut visited = vec![false; n];
    visited[start] = true;
    let mut ptr = 0;

    while ptr < queue.len() {
        let cur = queue[ptr];
        ptr += 1;
        for &up in &upstream[cur] {
            if !visited[up] {
                visited[up] = true;
                cells.push(up);
                queue.push(up);
            }
        }
    }

    let mut mask = vec![false; n];
    let cell_area_m2 = cell_size_m * cell_size_m;
    for &idx in &cells {
        mask[idx] = true;
    }

    WatershedResult {
        num_cells: cells.len(),
        area_ha: cells.len() as f64 * cell_area_m2 / 10000.0,
        mask,
        cells,
    }
}

// ─── GeoJSON 导出 ──────────────────────────────────────────

/// 将流域格点导出为 GeoJSON（bounding-box 多边形近似）。
pub fn watershed_to_geojson(
    cells: &[usize],
    ncols: usize,
    cell_size: f64,
    xmin: f64,
    ymax: f64,
) -> String {
    if cells.is_empty() {
        return String::from(r#"{"type":"FeatureCollection","features":[]}"#);
    }

    let mut min_r = usize::MAX;
    let mut max_r = 0usize;
    let mut min_c = usize::MAX;
    let mut max_c = 0usize;

    for &idx in cells {
        let r = idx / ncols;
        let c = idx % ncols;
        min_r = min_r.min(r);
        max_r = max_r.max(r);
        min_c = min_c.min(c);
        max_c = max_c.max(c);
    }

    let ymin = ymax - (max_r as f64 + 1.0) * cell_size;
    let ymax_b = ymax - min_r as f64 * cell_size;
    let xmin_b = xmin + min_c as f64 * cell_size;
    let xmax_b = xmin + (max_c as f64 + 1.0) * cell_size;

    format!(
        r#"{{"type":"FeatureCollection","features":[{{"type":"Feature","properties":{{}},"geometry":{{"type":"Polygon","coordinates":[[[{xmin},{ymin}],[{xmax},{ymin}],[{xmax},{ymax}],[{xmin},{ymax}],[{xmin},{ymin}]]]}}}}]}}"#,
        xmin = xmin_b,
        ymin = ymin,
        xmax = xmax_b,
        ymax = ymax_b,
    )
}

// ─── 测试 ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_watershed_all_to_center_5x5() {
        // 5x5: 所有格点流向中心 (2,2)
        let nrows = 5;
        let ncols = 5;
        let n = nrows * ncols;
        let mut flow_dir: Vec<Option<usize>> = vec![None; n];
        for r in 0..nrows {
            for c in 0..ncols {
                let idx = r * ncols + c;
                if r == 2 && c == 2 {
                    flow_dir[idx] = None; // center sink
                } else if r < 2 {
                    flow_dir[idx] = Some(2); // flow S
                } else if r > 2 {
                    flow_dir[idx] = Some(6); // flow N
                } else if c < 2 {
                    flow_dir[idx] = Some(0); // flow E
                } else {
                    flow_dir[idx] = Some(4); // flow W
                }
            }
        }

        let result = extract_watershed(&flow_dir, nrows, ncols, 2, 2, 10.0);
        assert_eq!(result.num_cells, 25);
        assert_eq!(result.cells.len(), 25);
        assert!(result.area_ha > 0.0);
    }

    #[test]
    fn test_extract_watershed_partial_3x3() {
        // 3x3: 所有格点流向 (1,0)
        let nrows = 3;
        let ncols = 3;
        let n = nrows * ncols;
        let mut flow_dir: Vec<Option<usize>> = vec![None; n];

        // (0,0)→S, (2,0)→N, (1,1)→W, (0,1)→S, (2,1)→N,
        // (1,2)→W, (0,2)→S, (2,2)→N, (1,0)=sink
        for r in 0..nrows {
            for c in 0..ncols {
                let idx = r * ncols + c;
                if r == 1 && c == 0 {
                    flow_dir[idx] = None; // pour-point
                } else if c == 0 {
                    flow_dir[idx] = if r < 1 { Some(2) } else { Some(6) };
                } else if c == 1 {
                    // col 1 cells drain toward col 0
                    let down_r = if r == 1 {
                        1
                    } else if r == 0 {
                        1
                    } else {
                        1
                    };
                    // (0,1)→S(2)→(1,1), (1,1)→W(4)→(1,0), (2,1)→N(6)→(1,1)
                    flow_dir[idx] = if r == 0 {
                        Some(2)
                    } else if r == 1 {
                        Some(4)
                    } else {
                        Some(6)
                    };
                } else {
                    // col 2 → col 1 → col 0
                    // (0,2)→S(2)→(1,2), (1,2)→W(4)→(1,1), (2,2)→N(6)→(1,2)
                    flow_dir[idx] = if r == 0 {
                        Some(2)
                    } else if r == 1 {
                        Some(4)
                    } else {
                        Some(6)
                    };
                }
            }
        }

        let result = extract_watershed(&flow_dir, nrows, ncols, 1, 0, 10.0);
        assert_eq!(result.num_cells, 9);
    }

    #[test]
    fn test_extract_watershed_disconnected() {
        // 3x3: 只有左列流向 (1,0)，右列流向边界外
        let nrows = 3;
        let ncols = 3;
        let n = nrows * ncols;
        let mut flow_dir: Vec<Option<usize>> = vec![None; n];

        // Left column → pour-point (1,0)
        flow_dir[0] = Some(2); // (0,0)→S→(1,0)
        flow_dir[3] = None; // (1,0) sink
        flow_dir[6] = Some(6); // (2,0)→N→(1,0)

        // Middle column → drains to right (out of watershed of (1,0))
        flow_dir[1] = Some(0); // (0,1)→E→(0,2)
        flow_dir[4] = Some(0); // (1,1)→E→(1,2)
        flow_dir[7] = Some(0); // (2,1)→E→(2,2)

        // Right column → out of bounds (south)
        flow_dir[2] = Some(2); // (0,2)→S, boundary
        flow_dir[5] = Some(2); // (1,2)→S, boundary
        flow_dir[8] = Some(2); // (2,2)→S, boundary

        let result = extract_watershed(&flow_dir, nrows, ncols, 1, 0, 30.0);
        assert_eq!(result.num_cells, 3);
        assert!(result.mask[0]);
        assert!(result.mask[3]);
        assert!(result.mask[6]);
        let area_expected = 3.0 * 30.0 * 30.0 / 10000.0;
        assert!((result.area_ha - area_expected).abs() < 0.001);
    }

    #[test]
    fn test_watershed_to_geojson() {
        let cells = vec![0, 1, 5, 6]; // 2×2 block at top-left of 3×3 grid
        let geojson = watershed_to_geojson(&cells, 3, 10.0, 0.0, 100.0);
        assert!(geojson.contains("FeatureCollection"));
        assert!(geojson.contains("Polygon"));
        assert!(geojson.contains("coordinates"));
    }

    #[test]
    fn test_d8_offset() {
        assert_eq!(d8_offset(0), (0, 1)); // E
        assert_eq!(d8_offset(1), (1, 1)); // SE
        assert_eq!(d8_offset(2), (1, 0)); // S
        assert_eq!(d8_offset(3), (1, -1)); // SW
        assert_eq!(d8_offset(4), (0, -1)); // W
        assert_eq!(d8_offset(5), (-1, -1)); // NW
        assert_eq!(d8_offset(6), (-1, 0)); // N
        assert_eq!(d8_offset(7), (-1, 1)); // NE
        assert_eq!(d8_offset(8), (0, 0)); // invalid
    }
}
