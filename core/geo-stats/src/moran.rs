//! Spatial autocorrelation — Moran's I.
//!
//! Measures the degree to which nearby spatial units are similar (positive)
//! or dissimilar (negative) in value.
//!
//! # Formula
//!
//! ```text
//! I = (N / W) * (Σ_i Σ_j wᵢⱼ (xᵢ - x̄)(xⱼ - x̄)) / (Σ_i (xᵢ - x̄)²)
//! ```
//!
//! Where:
//! - N = number of spatial units
//! - W = sum of all spatial weights
//! - wᵢⱼ = spatial weight between i and j
//! - xᵢ = value at unit i
//! - x̄ = mean of all values
//!
//! # Interpretation
//!
//! | I range | Interpretation |
//! |---------|----------------|
//! | +1.0    | Perfect positive spatial clustering |
//! | 0.0     | Random spatial pattern |
//! | -1.0    | Perfect negative spatial clustering (dispersion) |
//!
//! ## Statistical significance
//!
//! Z-score = (I - E[I]) / sqrt(Var[I]).
//! p-value from normal CDF (two-tailed).

/// Result of a Moran's I computation.
#[derive(Debug, Clone)]
pub struct MoranI {
    /// Moran's I statistic (~ -1 to +1).
    pub i: f64,
    /// Expected I under the null hypothesis of spatial randomness.
    pub expected_i: f64,
    /// Variance of I under the null hypothesis.
    pub variance_i: f64,
    /// Z-score.
    pub z_score: f64,
    /// Two-tailed p-value.
    pub p_value: f64,
    /// Number of spatial units.
    pub n: usize,
    /// Total spatial weight sum.
    pub weight_sum: f64,
}

/// Compute Moran's I from a distance matrix and z-values.
///
/// # Arguments
/// * `values` — attribute values at each spatial unit (zᵢ).
/// * `weights` — N×N spatial weight matrix (wᵢⱼ, entry i*N + j).
///   Use 1.0 for neighbors and 0.0 for non-neighbors.
///
/// Returns `None` if variance is zero or inputs are empty.
pub fn morans_i(values: &[f64], weights: &[f64]) -> Option<MoranI> {
    let n = values.len();
    if n == 0 || weights.len() != n * n {
        return None;
    }

    let mean: f64 = values.iter().sum::<f64>() / n as f64;
    let deviations: Vec<f64> = values.iter().map(|v| v - mean).collect();
    let sse: f64 = deviations.iter().map(|d| d * d).sum(); // Σ (xᵢ - x̄)²

    if sse == 0.0 {
        return None;
    }

    let mut num = 0.0; // Σ_i Σ_j wᵢⱼ (xᵢ - x̄)(xⱼ - x̄)
    let mut w_sum = 0.0; // Σ_i Σ_j wᵢⱼ
    let mut w2_sum = 0.0; // Σ_i Σ_j wᵢⱼ²
    let mut row_sum: Vec<f64> = vec![0.0; n]; // Σ_j wᵢⱼ per row
    let mut col_sum: Vec<f64> = vec![0.0; n]; // Σ_i wᵢⱼ per col

    for i in 0..n {
        for j in 0..n {
            let wij = weights[i * n + j];
            if wij != 0.0 {
                num += wij * deviations[i] * deviations[j];
                w_sum += wij;
                w2_sum += wij * wij;
                row_sum[i] += wij;
                col_sum[j] += wij;
            }
        }
    }

    if w_sum == 0.0 {
        return None;
    }

    let i_stat = (n as f64 / w_sum) * (num / sse);

    // Expected I under spatial randomness (H₀)
    let expected_i: f64 = -1.0 / (n as f64 - 1.0);

    // Variance of I (normality assumption, Cliff & Ord 1973)
    let s1 = 0.5
        * (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| {
                        let wij = weights[i * n + j];
                        let wji = weights[j * n + i];
                        (wij + wji) * (wij + wji)
                    })
                    .sum::<f64>()
            })
            .sum::<f64>();
    let s2 = (0..n)
        .map(|i| {
            let r = row_sum[i];
            let c = col_sum[i];
            (r + c) * (r + c)
        })
        .sum::<f64>();
    let n_f64 = n as f64;
    let b2: f64 = (sse / n_f64) / ((sse / n_f64) * (sse / n_f64)); // kurtosis

    let var_i = ((n as f64 * n as f64 * s1 - n as f64 * s2 + 3.0 * w_sum * w_sum)
        / ((n as f64 - 1.0) * (n as f64 + 1.0) * w_sum * w_sum))
        - expected_i * expected_i
        + ((n as f64 - 2.0) * (n as f64 - 3.0) * w2_sum
            - 3.0 * b2 * ((n as f64 - 1.0) * (n as f64 - 2.0))
            - 6.0 * b2 * ((n as f64 - 1.0) / (n as f64 + 1.0)) * w2_sum)
            / ((n as f64 - 1.0) * (n as f64 + 1.0));

    let z_score = if var_i > 0.0 {
        (i_stat - expected_i) / var_i.sqrt()
    } else {
        0.0
    };
    let p_value = 2.0 * (1.0 - normal_cdf(z_score.abs()));

    Some(MoranI {
        i: i_stat,
        expected_i,
        variance_i: var_i,
        z_score,
        p_value,
        n,
        weight_sum: w_sum,
    })
}

/// Standard normal CDF approximation (Abramowitz & Stegun 7.1.26).
fn normal_cdf(x: f64) -> f64 {
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs() / (2.0f64).sqrt();
    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - ((((a5 * t + a4) * t) + a3) * t + a2) * t + a1 * t * (-x * x).exp();
    0.5 * (1.0 + sign * y)
}

/// Build a rook-contiguity weight matrix for a regular grid.
///
/// Rook: neighbors are up/down/left/right (shares an edge).
pub fn rook_weights(nrows: usize, ncols: usize) -> Vec<f64> {
    let n = nrows * ncols;
    let mut w = vec![0.0; n * n];
    for r in 0..nrows {
        for c in 0..ncols {
            let i = r * ncols + c;
            if r > 0 {
                w[i * n + (r - 1) * ncols + c] = 1.0;
            }
            if r + 1 < nrows {
                w[i * n + (r + 1) * ncols + c] = 1.0;
            }
            if c > 0 {
                w[i * n + r * ncols + (c - 1)] = 1.0;
            }
            if c + 1 < ncols {
                w[i * n + r * ncols + (c + 1)] = 1.0;
            }
        }
    }
    w
}

/// Build a queen-contiguity weight matrix for a regular grid.
///
/// Queen: neighbors are all 8 surrounding cells (shares edge or corner).
pub fn queen_weights(nrows: usize, ncols: usize) -> Vec<f64> {
    let n = nrows * ncols;
    let mut w = vec![0.0; n * n];
    for r in 0..nrows {
        for c in 0..ncols {
            let i = r * ncols + c;
            for dr in [-1i32, 0, 1] {
                for dc in [-1i32, 0, 1] {
                    if dr == 0 && dc == 0 {
                        continue;
                    }
                    let nr = r as i32 + dr;
                    let nc = c as i32 + dc;
                    if nr >= 0 && nc >= 0 && (nr as usize) < nrows && (nc as usize) < ncols {
                        w[i * n + (nr as usize) * ncols + (nc as usize)] = 1.0;
                    }
                }
            }
        }
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_positive() {
        // All identical values: I → +1
        let values = vec![10.0, 10.0, 10.0, 10.0];
        let weights = queen_weights(2, 2);
        assert!(weights.len() == 16);
        // With identical values, I approaches 1
        // (formally NaN due to zero variance, so our function returns None)
        let result = morans_i(&values, &weights);
        assert!(result.is_none());
    }

    #[test]
    fn test_clustered_grid() {
        // 3x3 grid: high values clustered in top-left
        let values = vec![
            10.0, 9.0, 3.0, // top row: hot in NW
            8.0, 7.0, 4.0, // middle
            3.0, 4.0, 1.0, // bottom: cold in SE
        ];
        let weights = rook_weights(3, 3);
        let result = morans_i(&values, &weights).unwrap();
        // Positive spatial autocorrelation expected
        assert!(result.i > 0.0);
        assert!(result.z_score > 0.0);
    }

    #[test]
    fn test_rook_weights() {
        let w = rook_weights(2, 3);
        // 2×3 = 6 cells → 36-element weight matrix
        assert_eq!(w.len(), 36);
        // Cell 0 (r0,c0) should be neighbor to (r0,c1) and (r1,c0) only
        let idx = |r: usize, c: usize| r * 3 + c;
        assert!(w[0 * 6 + idx(0, 1)] > 0.0); // right
        assert!(w[0 * 6 + idx(1, 0)] > 0.0); // down
        assert!(w[0 * 6 + idx(0, 0)] == 0.0); // self
    }
}
