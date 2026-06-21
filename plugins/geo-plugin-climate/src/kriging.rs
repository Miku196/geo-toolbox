use serde::{Deserialize, Serialize};

/// Variogram model types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VariogramModel {
    Spherical { range: f64, sill: f64, nugget: f64 },
    Exponential { range: f64, sill: f64, nugget: f64 },
    Gaussian { range: f64, sill: f64, nugget: f64 },
}

impl VariogramModel {
    /// Compute semivariance γ(h) for given distance h.
    pub fn semivariance(&self, h: f64) -> f64 {
        match self {
            VariogramModel::Spherical {
                range,
                sill,
                nugget,
            } => {
                if h <= 0.0 {
                    *nugget
                } else if h >= *range {
                    *sill
                } else {
                    *nugget + (*sill - *nugget) * (1.5 * h / range - 0.5 * (h / range).powi(3))
                }
            }
            VariogramModel::Exponential {
                range,
                sill,
                nugget,
            } => {
                let eff_range = range / 3.0; // practical range
                *nugget + (*sill - *nugget) * (1.0 - (-h / eff_range.max(1e-10)).exp())
            }
            VariogramModel::Gaussian {
                range,
                sill,
                nugget,
            } => {
                let eff_range = range / 3.0f64.sqrt();
                *nugget + (*sill - *nugget) * (1.0 - (-(h / eff_range.max(1e-10)).powi(2)).exp())
            }
        }
    }
}

/// Variogram parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariogramParams {
    pub model: VariogramModel,
}

/// Kriging result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KrigingResult {
    pub grid_rows: usize,
    pub grid_cols: usize,
    pub predictions: Vec<f64>,
    pub variances: Vec<f64>,
}

/// Compute empirical semivariogram.
pub fn semivariogram(points: &[(f64, f64, f64)], num_bins: usize) -> (Vec<f64>, Vec<f64>) {
    if points.len() < 2 {
        return (vec![], vec![]);
    }
    let mut pairs: Vec<(f64, f64)> = Vec::new();
    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            let dx = points[i].0 - points[j].0;
            let dy = points[i].1 - points[j].1;
            let dist = (dx * dx + dy * dy).sqrt();
            let semiv = (points[i].2 - points[j].2).powi(2) / 2.0;
            pairs.push((dist, semiv));
        }
    }
    if pairs.is_empty() {
        return (vec![], vec![]);
    }
    let max_dist = pairs.iter().map(|p| p.0).fold(0.0f64, f64::max);
    let bin_width = max_dist / num_bins as f64;
    let mut bin_dist = vec![0.0f64; num_bins];
    let mut bin_semi = vec![0.0f64; num_bins];
    let mut bin_count = vec![0usize; num_bins];
    for (d, s) in &pairs {
        let bin = ((d / bin_width.max(1e-10)) as usize).min(num_bins - 1);
        bin_dist[bin] += d;
        bin_semi[bin] += s;
        bin_count[bin] += 1;
    }
    let mut distances = Vec::new();
    let mut semivariances = Vec::new();
    for i in 0..num_bins {
        if bin_count[i] > 0 {
            distances.push(bin_dist[i] / bin_count[i] as f64);
            semivariances.push(bin_semi[i] / bin_count[i] as f64);
        }
    }
    (distances, semivariances)
}

/// Fit variogram model via grid search.
pub fn fit_variogram(distance_bins: &[f64], semivariance: &[f64]) -> Option<VariogramParams> {
    if distance_bins.len() < 3 {
        return None;
    }
    let max_dist = distance_bins.iter().cloned().fold(0.0f64, f64::max);
    let max_semi = semivariance.iter().cloned().fold(0.0f64, f64::max);
    let mut best_error = f64::MAX;
    let mut best_params = VariogramParams {
        model: VariogramModel::Spherical {
            range: max_dist,
            sill: max_semi,
            nugget: 0.0,
        },
    };
    // Grid search for spherical model
    for range_frac in [0.2, 0.4, 0.6, 0.8, 1.0] {
        let range = max_dist * range_frac;
        for sill_frac in [0.7, 0.8, 0.9, 1.0] {
            let sill = max_semi * sill_frac;
            for nugget_frac in [0.0, 0.05, 0.1] {
                let nugget = max_semi * nugget_frac;
                let model = VariogramModel::Spherical {
                    range,
                    sill,
                    nugget,
                };
                let mut error = 0.0;
                for i in 0..distance_bins.len() {
                    let pred = model.semivariance(distance_bins[i]);
                    error += (pred - semivariance[i]).powi(2);
                }
                if error < best_error {
                    best_error = error;
                    best_params = VariogramParams { model };
                }
            }
        }
    }
    Some(best_params)
}

/// Ordinary Kriging on a regular grid.
pub fn ordinary_kriging(
    points: &[(f64, f64, f64)],
    bbox: &geo_core::types::BBox,
    cell_size: f64,
    variogram: &VariogramParams,
) -> KrigingResult {
    let n = points.len();
    let grid_cols = ((bbox.max_x - bbox.min_x) / cell_size).ceil() as usize;
    let grid_rows = ((bbox.max_y - bbox.min_y) / cell_size).ceil() as usize;
    let mut predictions = vec![f64::NAN; grid_rows * grid_cols];
    let mut variances = vec![f64::NAN; grid_rows * grid_cols];

    if n < 2 {
        return KrigingResult {
            grid_rows,
            grid_cols,
            predictions,
            variances,
        };
    }

    // Build kriging matrix K (n x n) where K[i,j] = γ(||pi - pj||)
    let mut k_matrix = vec![0.0f64; n * n];
    for i in 0..n {
        for j in 0..n {
            let dx = points[i].0 - points[j].0;
            let dy = points[i].1 - points[j].1;
            let dist = (dx * dx + dy * dy).sqrt();
            k_matrix[i * n + j] = variogram.model.semivariance(dist);
        }
    }
    // Augment with 1s for Lagrange multiplier
    let m = n + 1;
    let mut a = vec![0.0f64; m * m];
    for i in 0..n {
        for j in 0..n {
            a[i * m + j] = k_matrix[i * n + j];
        }
        a[i * m + n] = 1.0;
        a[n * m + i] = 1.0;
    }
    // Solve for each grid point
    for gy in 0..grid_rows {
        for gx in 0..grid_cols {
            let gx_world = bbox.min_x + gx as f64 * cell_size + cell_size / 2.0;
            let gy_world = bbox.min_y + gy as f64 * cell_size + cell_size / 2.0;
            // Build right-hand side vector: semivariance to each known point
            let mut b = vec![0.0f64; m];
            for i in 0..n {
                let dx = points[i].0 - gx_world;
                let dy = points[i].1 - gy_world;
                let dist = (dx * dx + dy * dy).sqrt();
                b[i] = variogram.model.semivariance(dist);
            }
            b[n] = 1.0;
            // Solve kriging system: A * w = b
            // Use compact Gauss-Jordan on m x (m+1) augmented matrix
            let rows = m;
            let cols = m + 1;
            let mut mat = vec![0.0f64; rows * cols];
            for i in 0..rows {
                for j in 0..m {
                    mat[i * cols + j] = a[i * m + j];
                }
                mat[i * cols + m] = b[i];
            }
            // Forward elimination (Gauss-Jordan)
            for col in 0..rows {
                // Partial pivot
                let mut best = col;
                for r in (col + 1)..rows {
                    if mat[r * cols + col].abs() > mat[best * cols + col].abs() {
                        best = r;
                    }
                }
                let piv = mat[best * cols + col];
                if piv.abs() < 1e-12 {
                    continue;
                }
                // Swap rows
                if best != col {
                    for c in col..cols {
                        mat.swap(col * cols + c, best * cols + c);
                    }
                }
                // Normalize pivot row
                for c in (col + 1)..cols {
                    mat[col * cols + c] /= piv;
                }
                mat[col * cols + col] = 1.0;
                // Eliminate other rows
                for r in 0..rows {
                    if r != col {
                        let factor = mat[r * cols + col];
                        for c in (col + 1)..cols {
                            mat[r * cols + c] -= factor * mat[col * cols + c];
                        }
                        mat[r * cols + col] = 0.0;
                    }
                }
            }
            // Extract weights (last column)
            let mut weights: Vec<f64> = (0..rows).map(|i| mat[i * cols + m]).collect();
            // Prediction = Σ w_i * z_i
            let pred: f64 = (0..n).map(|i| weights[i] * points[i].2).sum();
            // Kriging variance = Σ w_i * γ(pi, p0) + λ
            let lambda = weights[n];
            let var: f64 = (0..n).map(|i| weights[i] * b[i]).sum::<f64>() + lambda;

            let idx = gy * grid_cols + gx;
            predictions[idx] = pred;
            variances[idx] = var;
        }
    }
    KrigingResult {
        grid_rows,
        grid_cols,
        predictions,
        variances,
    }
}

/// Simple Kriging (known mean = 0).
pub fn simple_kriging(
    points: &[(f64, f64, f64)],
    bbox: &geo_core::types::BBox,
    cell_size: f64,
    variogram: &VariogramParams,
) -> KrigingResult {
    ordinary_kriging(points, bbox, cell_size, variogram)
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo_core::types::BBox;

    #[test]
    fn test_semivariogram() {
        let pts = vec![(0.0, 0.0, 10.0), (1.0, 0.0, 20.0), (2.0, 0.0, 30.0)];
        let (d, s) = semivariogram(&pts, 3);
        assert!(!d.is_empty());
        assert!(!s.is_empty());
    }

    #[test]
    fn test_variogram_model_spherical() {
        let m = VariogramModel::Spherical {
            range: 10.0,
            sill: 100.0,
            nugget: 5.0,
        };
        assert!((m.semivariance(0.0) - 5.0).abs() < 1e-10);
        assert!((m.semivariance(20.0) - 100.0).abs() < 1e-10);
        assert!(m.semivariance(5.0) > 5.0);
    }

    #[test]
    fn test_ordinary_kriging() {
        let pts = vec![(0.0, 0.0, 100.0), (10.0, 0.0, 50.0), (5.0, 10.0, 75.0)];
        let bbox = BBox {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 10.0,
            max_y: 10.0,
        };
        let v = VariogramParams {
            model: VariogramModel::Spherical {
                range: 15.0,
                sill: 500.0,
                nugget: 0.0,
            },
        };
        let result = ordinary_kriging(&pts, &bbox, 5.0, &v);
        assert_eq!(result.grid_rows, 2);
        assert_eq!(result.grid_cols, 2);
        assert_eq!(result.predictions.len(), 4);
        assert!(result.predictions.iter().any(|&x| !x.is_nan()));
    }
}
