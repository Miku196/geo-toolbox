use serde::{Deserialize, Serialize};

/// GCM projection data for a single grid point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcmProjection {
    pub model: String,
    pub scenario: String,
    pub variable: String,
    pub lat: f64,
    pub lon: f64,
    /// Monthly means (12 values) for historical period
    pub historical: [f64; 12],
    /// Monthly means (12 values) for projection period
    pub projected: [f64; 12],
}

/// Downscaling result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownscaleResult {
    pub variable: String,
    pub monthly_delta: [f64; 12],
    pub downscaled_monthly: [f64; 12],
    pub bias_corrected_monthly: Option<[f64; 12]>,
    pub method: String,
}

/// Delta method: add temp delta / multiply precip ratio.
pub fn delta_downscale(
    obs: &[f64; 12],
    gcm_hist: &[f64; 12],
    gcm_proj: &[f64; 12],
    variable: &str,
) -> DownscaleResult {
    let mut monthly_delta = [0.0f64; 12];
    let mut downscaled = [0.0f64; 12];
    let is_precip = variable == "pr" || variable == "precip" || variable == "precipitation";
    for i in 0..12 {
        if is_precip {
            monthly_delta[i] = if gcm_hist[i] != 0.0 {
                gcm_proj[i] / gcm_hist[i]
            } else {
                1.0
            };
            downscaled[i] = obs[i] * monthly_delta[i];
        } else {
            monthly_delta[i] = gcm_proj[i] - gcm_hist[i];
            downscaled[i] = obs[i] + monthly_delta[i];
        }
    }
    DownscaleResult {
        variable: variable.to_string(),
        monthly_delta,
        downscaled_monthly: downscaled,
        bias_corrected_monthly: None,
        method: "delta".into(),
    }
}

/// Quantile mapping bias correction.
pub fn quantile_mapping(obs: &[f64], gcm_hist: &[f64], gcm_proj: &[f64]) -> Vec<f64> {
    if obs.is_empty() || gcm_hist.is_empty() || gcm_proj.is_empty() {
        return Vec::new();
    }
    // Build sorted obs and gcm_hist
    let mut obs_sorted: Vec<(usize, f64)> = obs.iter().copied().enumerate().collect();
    obs_sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let obs_vals: Vec<f64> = obs_sorted.iter().map(|x| x.1).collect();

    let mut hist_sorted: Vec<(usize, f64)> = gcm_hist.iter().copied().enumerate().collect();
    hist_sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let hist_vals: Vec<f64> = hist_sorted.iter().map(|x| x.1).collect();

    // For each projection value, find its quantile in gcm_hist, map to obs
    let mut corrected = Vec::with_capacity(gcm_proj.len());
    for &proj_val in gcm_proj {
        // Find position in hist_vals via binary search
        let pos = match hist_vals
            .binary_search_by(|v| v.partial_cmp(&proj_val).unwrap_or(std::cmp::Ordering::Less))
        {
            Ok(p) => p,
            Err(p) => p.min(hist_vals.len().saturating_sub(1)),
        };
        let frac = pos as f64 / (hist_vals.len().saturating_sub(1)).max(1) as f64;
        let obs_pos = (frac * (obs_vals.len().saturating_sub(1)) as f64).round() as usize;
        corrected.push(obs_vals[obs_pos.min(obs_vals.len().saturating_sub(1))]);
    }
    corrected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_downscale_temp() {
        let obs = [20.0; 12];
        let hist = [20.0; 12];
        let proj = [22.0; 12];
        let result = delta_downscale(&obs, &hist, &proj, "tas");
        assert!((result.monthly_delta[0] - 2.0).abs() < 1e-10);
        assert!((result.downscaled_monthly[0] - 22.0).abs() < 1e-10);
    }

    #[test]
    fn test_delta_downscale_precip() {
        let obs = [100.0; 12];
        let hist = [100.0; 12];
        let proj = [120.0; 12];
        let result = delta_downscale(&obs, &hist, &proj, "pr");
        assert!((result.monthly_delta[0] - 1.2).abs() < 1e-10);
    }

    #[test]
    fn test_quantile_mapping() {
        let obs = vec![10.0, 20.0, 30.0, 40.0];
        let hist = vec![12.0, 18.0, 28.0, 38.0];
        let proj = vec![15.0, 25.0];
        let result = quantile_mapping(&obs, &hist, &proj);
        assert_eq!(result.len(), 2);
        assert!(result[0] > 0.0);
    }
}
