//! Jenks Natural Breaks classification.
//!
//! Also known as "Fisher-Jenks" or "Jenks Optimization" — finds natural
//! break points in data by minimizing within-class variance.
//!
//! Used for choropleth map classification and raster reclassification.

/// Result of a Jenks Natural Breaks classification.
#[derive(Debug, Clone)]
pub struct JenksResult {
    /// The computed break points (k-1 values).
    pub breaks: Vec<f64>,
    /// Class assignments for each input value.
    pub classes: Vec<usize>,
    /// Goodness of Variance Fit (0..1). Higher = better separation.
    pub gvf: f64,
    /// Number of classes.
    pub k: usize,
}

/// Compute Jenks Natural Breaks for given data and number of classes.
///
/// # Arguments
/// * `data` — input values (unsorted, will be sorted internally).
/// * `k` — number of desired classes (2..=10).
///
/// Returns `None` if data is too short or k < 2.
pub fn jenks(data: &[f64], k: usize) -> Option<JenksResult> {
    let n = data.len();
    if n < k || !(2..=10).contains(&k) {
        return None;
    }

    // Sort data
    let mut sorted: Vec<f64> = data.to_vec();
    sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // Remove duplicates at extremes (Jenks requires unique values for break placement)
    if sorted[0] == sorted[n - 1] {
        // All same value — single class
        return Some(JenksResult {
            breaks: vec![sorted[0]],
            classes: vec![0; n],
            gvf: 1.0,
            k,
        });
    }

    // Fisher-Jenks: dynamic programming
    // mat[i][j] = error sum of squares for clustering points 0..i into j classes
    // Using Vec<Vec<Option<f64>>> for sparser computation
    let mut sse = vec![vec![0.0f64; k + 1]; n + 1];
    let mut breaks = vec![vec![0usize; k + 1]; n + 1];

    for i in 1..=n {
        sse[i][1] = variance(&sorted[..i]) * i as f64;
        breaks[i][1] = 1;
    }

    for j in 2..=k {
        for i in j..=n {
            sse[i][j] = f64::MAX;
            for p in (j - 1)..i {
                let var = variance(&sorted[p..i]) * (i - p) as f64;
                let candidate = sse[p][j - 1] + var;
                if candidate < sse[i][j] {
                    sse[i][j] = candidate;
                    breaks[i][j] = p;
                }
            }
        }
    }

    // Extract break positions
    let total_variance = variance(&sorted) * n as f64;
    let gvf = if total_variance > 0.0 {
        1.0 - sse[n][k] / total_variance
    } else {
        1.0
    };

    let mut break_positions = Vec::with_capacity(k);
    let mut current = n;
    for j in (1..=k).rev() {
        let p = breaks[current][j];
        if j > 1 {
            break_positions.push(p);
        }
        current = p;
    }
    break_positions.reverse();

    // Convert positions to actual break values (upper bound of each class)
    let mut class_breaks: Vec<f64> = Vec::with_capacity(k - 1);
    for &pos in &break_positions[..break_positions.len().saturating_sub(1)] {
        class_breaks.push(sorted[pos]);
    }

    // Assign classes to original data
    let classes: Vec<usize> = data
        .iter()
        .map(|&v| {
            let mut cls = k - 1;
            for (i, &b) in class_breaks.iter().enumerate() {
                if v <= b {
                    cls = i;
                    break;
                }
            }
            cls
        })
        .collect();

    Some(JenksResult {
        breaks: class_breaks,
        classes,
        gvf,
        k,
    })
}

/// Compute variance of a slice (biased, divide by n).
fn variance(slice: &[f64]) -> f64 {
    let n = slice.len() as f64;
    if n <= 1.0 {
        return 0.0;
    }
    let mean = slice.iter().sum::<f64>() / n;
    slice.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n
}

/// Quantile-based classification (equal count per class).
pub fn quantile_breaks(data: &[f64], k: usize) -> Option<Vec<f64>> {
    let n = data.len();
    if n < k || k < 2 {
        return None;
    }
    let mut sorted: Vec<f64> = data.to_vec();
    sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mut breaks = Vec::with_capacity(k - 1);
    for i in 1..k {
        let idx = (i as f64 * n as f64 / k as f64).round() as usize;
        let idx = idx.min(n - 1);
        breaks.push(sorted[idx]);
    }
    Some(breaks)
}

/// Equal interval classification (evenly spaced breaks).
pub fn equal_interval_breaks(data: &[f64], k: usize) -> Option<Vec<f64>> {
    if data.is_empty() || k < 2 {
        return None;
    }
    let min = data.iter().cloned().fold(f64::MAX, f64::min);
    let max = data.iter().cloned().fold(f64::MIN, f64::max);
    let step = (max - min) / k as f64;

    let breaks: Vec<f64> = (1..k).map(|i| min + i as f64 * step).collect();
    Some(breaks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jenks_basic() {
        let data = vec![
            1.0, 1.5, 2.0, 2.5, 3.0, // cluster low
            50.0, 55.0, 60.0, // cluster mid
            200.0, 220.0, // cluster high
        ];
        let result = jenks(&data, 3).unwrap();
        assert!(
            result.gvf > 0.8,
            "GVF should be high for well-separated data"
        );
        assert_eq!(result.breaks.len(), 2);
    }

    #[test]
    fn test_quantile() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let breaks = quantile_breaks(&data, 3).unwrap();
        assert_eq!(breaks.len(), 2);
    }

    #[test]
    fn test_equal_interval() {
        let data = vec![0.0, 10.0, 20.0, 30.0, 40.0, 50.0];
        let breaks = equal_interval_breaks(&data, 5).unwrap();
        assert_eq!(breaks.len(), 4);
        // Each break is 10 units apart (step = 10)
        assert!((breaks[0] - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_uniform_data() {
        let data = vec![5.0; 10];
        let result = jenks(&data, 3).unwrap();
        assert!((result.gvf - 1.0).abs() < 1e-6);
    }
}
