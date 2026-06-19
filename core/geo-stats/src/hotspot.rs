//! Getis-Ord Gi* hotspot analysis.
//!
//! Identifies statistically significant spatial clusters of high
//! (hot spots) and low (cold spots) attribute values.
//!
//! # Formula
//!
//! ```text
//! Gᵢ* = (Σⱼ wᵢⱼ xⱼ - x̄ × Σⱼ wᵢⱼ) /
//!        (s × sqrt( (N × Σⱼ wᵢⱼ² - (Σⱼ wᵢⱼ)²) / (N - 1) ))
//! ```
//!
//! Where:
//! - wᵢⱼ = spatial weight between i and j (i included)
//! - xⱼ = value at location j
//! - x̄ = global mean of all x
//! - s = standard deviation of all x
//! - N = number of spatial units
//!
//! # Interpretation
//!
//! | Gi* z-score | Interpretation |
//! |-------------|----------------|
//! | > 2.58      | Hot spot (99% confidence) |
//! | > 1.96      | Hot spot (95% confidence) |
//! | > 1.65      | Hot spot (90% confidence) |
//! | -1.65 ~ 1.65 | Not significant |
//! | < -1.65     | Cold spot (90% confidence) |
//! | < -1.96     | Cold spot (95% confidence) |
//! | < -2.58     | Cold spot (99% confidence) |

/// Result for a single spatial unit in the hotspot analysis.
#[derive(Debug, Clone)]
pub struct GiStar {
    /// Original index/position.
    pub index: usize,
    /// Getis-Ord Gi* z-score.
    pub z_score: f64,
    /// p-value (two-tailed).
    pub p_value: f64,
    /// Whether this location is a hot spot.
    pub is_hotspot: bool,
    /// Whether this location is a cold spot.
    pub is_coldspot: bool,
    /// Sum of weighted neighbor values (Σ wᵢⱼ xⱼ).
    pub local_sum: f64,
    /// Number of neighbors (including self).
    pub neighbor_count: usize,
}

/// Compute Getis-Ord Gi* for all spatial units.
///
/// # Arguments
/// * `values` — attribute values at each spatial unit.
/// * `weights` — N×N spatial weight matrix (entry i*N + j).
///   Include self-weight (wᵢᵢ) — typically set to 1.0.
/// * `confidence` — statistical confidence threshold for significance:
///   - 0.10 → 90% (|z| > 1.65)
///   - 0.05 → 95% (|z| > 1.96)
///   - 0.01 → 99% (|z| > 2.58)
///
/// Returns `None` if inputs are empty or have zero variance.
pub fn gistar(values: &[f64], weights: &[f64], confidence: f64) -> Option<Vec<GiStar>> {
    let n = values.len();
    if n == 0 || weights.len() != n * n {
        return None;
    }

    let mean: f64 = values.iter().sum::<f64>() / n as f64;
    let variance: f64 = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    let std_dev = variance.sqrt();
    if std_dev == 0.0 {
        return None;
    }

    let z_threshold = match confidence {
        c if c <= 0.01 => 2.5758,
        c if c <= 0.05 => 1.96,
        c if c <= 0.10 => 1.6449,
        _ => 1.96,
    };

    let n_f64 = n as f64;

    let mut results = Vec::with_capacity(n);
    for i in 0..n {
        let mut w_sum = 0.0; // Σⱼ wᵢⱼ
        let mut w2_sum = 0.0; // Σⱼ wᵢⱼ²
        let mut wx_sum = 0.0; // Σⱼ wᵢⱼ xⱼ
        let mut nbr_count = 0usize;

        for j in 0..n {
            let wij = weights[i * n + j];
            if wij != 0.0 {
                w_sum += wij;
                w2_sum += wij * wij;
                wx_sum += wij * values[j];
                nbr_count += 1;
            }
        }

        if w_sum == 0.0 {
            results.push(GiStar {
                index: i,
                z_score: 0.0,
                p_value: 1.0,
                is_hotspot: false,
                is_coldspot: false,
                local_sum: 0.0,
                neighbor_count: 0,
            });
            continue;
        }

        let num = wx_sum - mean * w_sum;
        let denom = std_dev
            * ((n_f64 * w2_sum - w_sum * w_sum) / (n_f64 - 1.0))
                .max(0.0)
                .sqrt();

        let z_score = if denom > 0.0 { num / denom } else { 0.0 };

        // Two-tailed p-value via error function approximation
        let p_value = 2.0 * (1.0 - normal_cdf(z_score.abs()));
        let is_hotspot = z_score > z_threshold;
        let is_coldspot = z_score < -z_threshold;

        results.push(GiStar {
            index: i,
            z_score,
            p_value,
            is_hotspot,
            is_coldspot,
            local_sum: wx_sum,
            neighbor_count: nbr_count,
        });
    }

    Some(results)
}

/// Standard normal CDF (Abramowitz & Stegun 7.1.26).
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

/// Build a queen-contiguity weight matrix with self-inclusion.
///
/// Each cell has weight 1.0 for itself and all 8 neighbors.
pub fn queen_weights_self(nrows: usize, ncols: usize) -> Vec<f64> {
    let n = nrows * ncols;
    let mut w = vec![0.0; n * n];
    for r in 0..nrows {
        for c in 0..ncols {
            let i = r * ncols + c;
            for dr in [-1i32, 0, 1] {
                for dc in [-1i32, 0, 1] {
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
    fn test_hotspot_clustered() {
        // 3x3: hot cluster in the top-left
        let values = vec![10.0, 9.0, 1.0, 8.0, 7.0, 2.0, 1.0, 2.0, 0.0];
        let weights = queen_weights_self(3, 3);
        let result = gistar(&values, &weights, 0.05).unwrap();
        assert_eq!(result.len(), 9);

        // Top-left cell should be a hot spot
        assert!(result[0].z_score > 0.0);
        // Bottom-right should be a cold spot
        assert!(result[8].z_score < 0.0);
    }

    #[test]
    fn test_uniform_data() {
        let values = vec![5.0; 9];
        let weights = queen_weights_self(3, 3);
        let result = gistar(&values, &weights, 0.05);
        assert!(result.is_none()); // zero variance
    }

    #[test]
    fn test_weights_self_include() {
        let w = queen_weights_self(1, 3);
        // Each of 3 cells should be neighbor to itself and its adjacent
        assert!(w[0 * 3 + 0] > 0.0); // self
        assert!(w[0 * 3 + 1] > 0.0); // right neighbor
        assert!(w[0 * 3 + 2] == 0.0); // non-neighbor
    }
}
