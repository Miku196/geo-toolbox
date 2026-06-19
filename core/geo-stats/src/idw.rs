//! Inverse Distance Weighted (IDW) spatial interpolation.
//!
//! Estimates values at unobserved locations as a weighted average of
//! nearby observed points. Closer points have higher weight.
//!
//! # Formula
//!
//! ```text
//! ẑ(x) = Σᵢ (zᵢ / dᵢᵖ) / Σᵢ (1 / dᵢᵖ)
//! ```
//!
//! Where:
//! - zᵢ = observed value at point i
//! - dᵢ = Euclidean distance from target to point i
//! - p = power parameter (typically 2). Higher p → more local influence.
//!
//! # Variants
//! - `idw_point` — interpolate at a single point
//! - `idw_grid` — interpolate a regular 2D grid

use geo_core::types::BBox;

/// Result of an IDW interpolation.
#[derive(Debug, Clone)]
pub struct IdwResult {
    /// Number of source points used.
    pub point_count: usize,
    /// Power parameter used.
    pub power: f64,
    /// Maximum search radius in coordinate units.
    pub max_radius: f64,
}

/// Interpolate value at a target point using IDW.
///
/// # Arguments
/// * `x_target`, `y_target` — target location.
/// * `x_src`, `y_src` — parallel arrays of source points.
/// * `values_src` — observed values at source points.
/// * `power` — inverse distance power (default 2). Higher = sharper peaks.
/// * `max_radius` — only consider points within this distance (≤ 0 means all points).
/// * `min_neighbors` — minimum neighbors needed (returns None below).
///
/// Returns `None` if fewer than `min_neighbors` within `max_radius`.
#[allow(clippy::too_many_arguments)]
pub fn idw_point(
    x_target: f64,
    y_target: f64,
    x_src: &[f64],
    y_src: &[f64],
    values_src: &[f64],
    power: f64,
    max_radius: f64,
    min_neighbors: usize,
) -> Option<f64> {
    let n = x_src.len().min(y_src.len()).min(values_src.len());
    if n == 0 {
        return None;
    }

    let mut weighted_sum = 0.0;
    let mut weight_sum = 0.0;
    let mut neighbor_count = 0usize;

    for i in 0..n {
        let dx = x_target - x_src[i];
        let dy = y_target - y_src[i];
        let d = (dx * dx + dy * dy).sqrt();

        if max_radius > 0.0 && d > max_radius {
            continue;
        }

        // Avoid division by zero: if target is exactly at a source point, return its value
        if d < 1e-12 {
            return Some(values_src[i]);
        }

        let w = 1.0 / d.powf(power);
        weighted_sum += w * values_src[i];
        weight_sum += w;
        neighbor_count += 1;
    }

    if neighbor_count < min_neighbors || weight_sum == 0.0 {
        return None;
    }

    Some(weighted_sum / weight_sum)
}

/// Interpolate a regular 2D grid using IDW.
///
/// # Arguments
/// * `bbox` — bounding box of the target grid.
/// * `ncols`, `nrows` — grid dimensions.
/// * See `idw_point` for source data parameters.
///
/// # Returns
/// 2D array of interpolated values (row-major: row, col → r * ncols + c),
/// plus metadata. Cells with insufficient neighbors get `NaN`.
#[allow(clippy::too_many_arguments)]
pub fn idw_grid(
    bbox: &BBox,
    ncols: usize,
    nrows: usize,
    x_src: &[f64],
    y_src: &[f64],
    values_src: &[f64],
    power: f64,
    max_radius: f64,
    min_neighbors: usize,
) -> (Vec<f64>, IdwResult) {
    let cell_width = (bbox.max_x - bbox.min_x) / ncols as f64;
    let cell_height = (bbox.max_y - bbox.min_y) / nrows as f64;
    let n_cells = ncols * nrows;
    let mut grid = vec![f64::NAN; n_cells];

    for r in 0..nrows {
        let y = bbox.max_y - (r as f64 + 0.5) * cell_height; // pixel center y
        for c in 0..ncols {
            let x = bbox.min_x + (c as f64 + 0.5) * cell_width; // pixel center x
            grid[r * ncols + c] = idw_point(
                x,
                y,
                x_src,
                y_src,
                values_src,
                power,
                max_radius,
                min_neighbors,
            )
            .unwrap_or(f64::NAN);
        }
    }

    (
        grid,
        IdwResult {
            point_count: x_src.len(),
            power,
            max_radius,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idw_self() {
        // A single source point: target at the same location → returns that value
        let v = idw_point(10.0, 20.0, &[10.0], &[20.0], &[42.0], 2.0, 0.0, 1);
        assert_eq!(v, Some(42.0));
    }

    #[test]
    fn test_idw_average() {
        // Two points at equal distance: returns their mean
        let v = idw_point(
            0.0,
            0.0,
            &[-1.0, 1.0],
            &[0.0, 0.0],
            &[10.0, 20.0],
            2.0,
            0.0,
            1,
        );
        assert!(v.is_some());
        let val = v.unwrap();
        // Both at distance 1 → weight 1 each → average = 15.0
        assert!((val - 15.0).abs() < 0.01);
    }

    #[test]
    fn test_idw_radius_filter() {
        // Only one point within radius 2
        let v = idw_point(
            0.0,
            0.0,
            &[10.0, 1.0],
            &[0.0, 0.0],
            &[100.0, 5.0],
            2.0,
            2.0,
            1,
        );
        assert!(v.is_some());
        // Only (1.0, 0.0) at distance 1 is within radius
        assert!((v.unwrap() - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_idw_grid() {
        let bbox = BBox {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 10.0,
            max_y: 10.0,
        };
        let (grid, meta) = idw_grid(
            &bbox,
            10,
            10,
            &[5.0, 5.0],
            &[2.0, 8.0],
            &[10.0, 20.0],
            2.0,
            0.0,
            1,
        );
        assert_eq!(grid.len(), 100);
        assert_eq!(meta.point_count, 2);
    }
}
