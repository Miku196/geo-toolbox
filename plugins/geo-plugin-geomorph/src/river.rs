use serde::{Deserialize, Serialize};

/// Strahler stream order result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrahlerResult {
    /// Stream order per cell (1=first order, 0=non-stream)
    pub order: Vec<u8>,
    pub rows: usize,
    pub cols: usize,
    /// Number of streams per order
    pub stream_count_per_order: Vec<usize>,
    /// Total stream length per order (cells × cell_size_m)
    pub stream_length_per_order: Vec<f64>,
}

/// Valley cross-section result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValleyCrossSection {
    /// Distance from origin (m) along the profile
    pub distance_m: Vec<f64>,
    /// Elevation at each point (m)
    pub elevation_m: Vec<f64>,
    /// Valley width (m) at a given height above channel floor
    pub valley_width_m: f64,
    /// Depth of valley at cross-section (max - min elevation)
    pub depth_m: f64,
    /// Index of channel (lowest point)
    pub channel_idx: usize,
}

/// Compute Strahler stream order from D8 flow directions and stream mask.
/// Algorithm: first-order = outer streams. When two same-order streams merge → order+1.
/// When different orders merge → max order.
pub fn strahler_order(
    flow_dir: &[u8],
    stream_mask: &[bool],
    rows: usize,
    cols: usize,
) -> StrahlerResult {
    let len = rows * cols;
    let mut order = vec![0u8; len];
    let mut in_degree = vec![0u32; len];

    // Compute in-degree: count of upstream neighbors that drain into this cell
    for r in 0..rows {
        for c in 0..cols {
            let idx = r * cols + c;
            if !stream_mask[idx] {
                continue;
            }
            let d = flow_dir[idx];
            if d >= 8 {
                continue;
            }
            let nr = (r as isize + crate::d8::D8_DR[d as usize]) as usize;
            let nc = (c as isize + crate::d8::D8_DC[d as usize]) as usize;
            if nr < rows && nc < cols {
                in_degree[nr * cols + nc] += 1;
            }
        }
    }

    // Topological sort: process headwater cells first (in_degree == 0)
    let mut queue: Vec<usize> = Vec::new();
    for i in 0..len {
        if stream_mask[i] && in_degree[i] == 0 {
            queue.push(i);
            order[i] = 1; // first order
        }
    }

    let mut local_indeg = in_degree.clone();

    while let Some(idx) = queue.pop() {
        let d = flow_dir[idx];
        if d >= 8 {
            continue;
        }
        let r = idx / cols;
        let c = idx % cols;
        let nr = (r as isize + crate::d8::D8_DR[d as usize]) as usize;
        let nc = (c as isize + crate::d8::D8_DC[d as usize]) as usize;
        if nr >= rows || nc >= cols {
            continue;
        }
        let nidx = nr * cols + nc;
        if !stream_mask[nidx] {
            continue;
        }

        // When merging: order = max(self_order, neighbor_order)
        // If both same order → order+1
        if order[nidx] == 0 {
            order[nidx] = order[idx];
        } else if order[idx] == order[nidx] {
            order[nidx] = order[idx] + 1;
        } else {
            order[nidx] = order[nidx].max(order[idx]);
        }

        local_indeg[nidx] = local_indeg[nidx].saturating_sub(1);
        if local_indeg[nidx] == 0 {
            queue.push(nidx);
        }
    }

    // Count streams per order
    let max_order = *order.iter().max().unwrap_or(&0) as usize;
    let mut count = vec![0usize; max_order + 1];
    let mut length = vec![0.0f64; max_order + 1];
    for i in 0..len {
        let o = order[i] as usize;
        if o > 0 && o <= max_order {
            count[o] = 1; // we count if at least one cell has this order
                          // Check if this is the start of a new stream segment
                          // (upstream cell has different order or is non-stream)
            let r = i / cols;
            let c = i % cols;
            let mut is_head = true;
            for d in 0..8usize {
                let pr = r as isize - crate::d8::D8_DR[d];
                let pc = c as isize - crate::d8::D8_DC[d];
                if pr >= 0 && pr < rows as isize && pc >= 0 && pc < cols as isize {
                    let pi = pr as usize * cols + pc as usize;
                    if stream_mask[pi] && flow_dir[pi] < 8 {
                        // Check if this neighbor flows into us
                        let nd = flow_dir[pi] as usize;
                        let npr = pr + crate::d8::D8_DR[nd];
                        let npc = pc + crate::d8::D8_DC[nd];
                        if npr as usize == r && npc as usize == c {
                            // This upstream neighbor has same order → not a head
                            if order[pi] == order[i] {
                                is_head = false;
                            }
                        }
                    }
                }
            }
            if is_head {
                count[o] += 1;
            }
            length[o] += 1.0; // accumulator for cell count per order
        }
    }

    StrahlerResult {
        order,
        rows,
        cols,
        stream_count_per_order: count,
        stream_length_per_order: length,
    }
}

/// Compute valley cross-section elevation profile.
/// Extracts a transect across a DEM at given channel location, perpendicular to flow.
pub fn valley_cross_section(
    dem: &[f64],
    _dem_rows: usize,
    dem_cols: usize,
    channel_row: usize,
    channel_col: usize,
    half_width: usize,
) -> ValleyCrossSection {
    let mut dist = Vec::new();
    let mut elev = Vec::new();

    let mut channel_idx = half_width; // default: center

    // Extract E-W transect through the channel point
    let left = channel_col.saturating_sub(half_width);
    let right = (channel_col + half_width).min(dem_cols - 1);

    let mut min_elev = f64::MAX;
    for c in left..=right {
        let idx = channel_row * dem_cols + c;
        let z = dem[idx];
        dist.push(c as f64 - channel_col as f64);
        elev.push(z);
        if z < min_elev {
            min_elev = z;
            channel_idx = c - left;
        }
    }

    let max_elev = elev.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let depth_m = max_elev - min_elev;

    // Valley width at 20% above channel floor
    let threshold_elev = min_elev + depth_m * 0.2;
    let mut valley_width_m = 0.0;
    let mut in_valley = false;
    let mut valley_start: Option<f64> = None;
    for (i, &z) in elev.iter().enumerate() {
        if z > threshold_elev && z.is_finite() {
            if !in_valley {
                valley_start = Some(dist[i]);
                in_valley = true;
            }
        } else {
            if in_valley {
                if let Some(start) = valley_start {
                    valley_width_m += dist[i] - start;
                }
                in_valley = false;
                valley_start = None;
            }
        }
    }
    if in_valley {
        if let Some(start) = valley_start {
            valley_width_m += dist.last().unwrap_or(&dist[0]) - start;
        }
    }

    ValleyCrossSection {
        distance_m: dist,
        elevation_m: elev,
        valley_width_m,
        depth_m,
        channel_idx,
    }
}

/// Extract stream segments from flow direction + stream mask.
/// Returns list of segments, each as list of cell indices (row-major).
pub fn extract_stream_segments(
    flow_dir: &[u8],
    stream_mask: &[bool],
    order: &[u8],
    rows: usize,
    cols: usize,
) -> Vec<Vec<usize>> {
    let len = rows * cols;
    let mut visited = vec![false; len];
    let mut segments = Vec::new();

    // Find headwater cells (stream cell with no upstream stream neighbor)
    for i in 0..len {
        if !stream_mask[i] || visited[i] {
            continue;
        }
        // Check if any upstream neighbor is also a stream
        let r = i / cols;
        let c = i % cols;
        let mut has_upstream = false;
        for d in 0..8usize {
            let pr = r as isize - crate::d8::D8_DR[d];
            let pc = c as isize - crate::d8::D8_DC[d];
            if pr >= 0 && pr < rows as isize && pc >= 0 && pc < cols as isize {
                let pi = pr as usize * cols + pc as usize;
                if stream_mask[pi] && flow_dir[pi] < 8 {
                    let nd = flow_dir[pi] as usize;
                    let npr = pr + crate::d8::D8_DR[nd];
                    let npc = pc + crate::d8::D8_DC[nd];
                    if npr as usize == r && npc as usize == c {
                        has_upstream = true;
                        break;
                    }
                }
            }
        }
        if has_upstream {
            continue; // not a headwater
        }

        // Walk downstream building segment
        let mut segment = Vec::new();
        let mut cr = r;
        let mut cc = c;
        loop {
            let ci = cr * cols + cc;
            if !stream_mask[ci] || visited[ci] {
                break;
            }
            visited[ci] = true;
            segment.push(ci);

            let d = flow_dir[ci];
            if d >= 8 {
                break;
            }
            let nr = (cr as isize + crate::d8::D8_DR[d as usize]) as usize;
            let nc = (cc as isize + crate::d8::D8_DC[d as usize]) as usize;
            if nr >= rows || nc >= cols {
                break;
            }
            let ni = nr * cols + nc;
            if !stream_mask[ni] {
                break;
            }
            // Stop at confluence (out-degree > 1 not possible in D8, but we stop at junctions)
            if order[ni] > order[ci] {
                // Order change means junction reached
                visited[ni] = true; // Mark junction as visited
                break;
            }
            cr = nr;
            cc = nc;
        }
        if !segment.is_empty() {
            segments.push(segment);
        }
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::d8::{d8_flow_accumulation_fast, d8_flow_direction, extract_streams};

    fn make_dem_flat() -> Vec<f64> {
        vec![
            10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0,
        ]
    }

    fn make_dem_slope() -> Vec<f64> {
        vec![
            20.0, 20.0, 20.0, 20.0, 15.0, 15.0, 15.0, 15.0, 10.0, 10.0, 10.0, 10.0,
        ]
    }

    #[test]
    fn test_strahler_flat() {
        let dem = make_dem_flat();
        let fd = d8_flow_direction(&dem, 3, 4);
        let acc = d8_flow_accumulation_fast(&fd.directions, 3, 4);
        let stream = extract_streams(&acc.accumulation, 3, 4, 3);
        let result = strahler_order(&fd.directions, &stream, 3, 4);
        // Flat DEM → no flow → all pits → no streams
        assert!(result.order.iter().all(|&o| o == 0));
    }

    #[test]
    fn test_strahler_slope() {
        let dem = make_dem_slope();
        let fd = d8_flow_direction(&dem, 3, 4);
        let acc = d8_flow_accumulation_fast(&fd.directions, 3, 4);
        // Low threshold to mark most cells as stream
        let stream = extract_streams(&acc.accumulation, 3, 4, 1);
        let result = strahler_order(&fd.directions, &stream, 3, 4);
        // Top row should be first order
        for c in 0..4 {
            let idx = 0 * 4 + c;
            if stream[idx] {
                assert!(
                    result.order[idx] >= 1,
                    "top row cell should have order >= 1"
                );
            }
        }
    }

    #[test]
    fn test_valley_cross_section() {
        // V-shaped valley: center is lowest
        let mut dem = vec![10.0f64; 5 * 5];
        for i in 0..5 {
            for j in 0..5 {
                dem[i * 5 + j] = (j as f64 - 2.0).abs() * 5.0;
            }
        }
        // Channel at row=2, col=2 → elevation should be 0
        let section = valley_cross_section(&dem, 5, 5, 2, 2, 2);
        assert!(section.depth_m > 0.0);
        assert!(section.valley_width_m > 0.0);
        assert_eq!(section.channel_idx, 2); // center
    }

    #[test]
    fn test_extract_stream_segments_simple() {
        // 3 rows, 4 cols, all flowing south
        let rows = 3;
        let cols = 4;
        let mut dirs = vec![2u8; rows * cols]; // south
        let mask = vec![true; rows * cols];
        let order = vec![1u8; rows * cols];
        let segments = extract_stream_segments(&dirs, &mask, &order, rows, cols);
        assert!(!segments.is_empty());
    }

    #[test]
    fn test_valley_cross_section_v_shape() {
        // Classic V-shaped valley: center low, sides high
        let size = 9;
        let mut dem = vec![0.0f64; size * size];
        for i in 0..size {
            for j in 0..size {
                let dist_from_center = ((j as f64) - 4.0).abs();
                dem[i * size + j] = dist_from_center * 10.0; // V-shape
            }
        }
        let section = valley_cross_section(&dem, size, size, 4, 4, 4);
        // Valley width should be > 0
        assert!(section.valley_width_m > 0.0);
        // Center should be channel
        assert_eq!(section.channel_idx, 4);
        // Depth should be (0..4)*10 = 40
        assert!((section.depth_m - 40.0).abs() < 10.0);
    }
}
