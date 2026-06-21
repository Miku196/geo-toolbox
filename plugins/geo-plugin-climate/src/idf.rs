use serde::{Deserialize, Serialize};

/// IDF curve parameters from Sherman formula: i = a / (t + b)^c
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdfParams {
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

/// IDF curve result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdfResult {
    pub return_period_yr: f64,
    pub durations_min: Vec<f64>,
    pub intensities_mmh: Vec<f64>,
    pub depths_mm: Vec<f64>,
}

/// Compute intensity for given duration: i = a / (t + b)^c
pub fn idf_intensity(duration_min: f64, params: &IdfParams) -> f64 {
    params.a / (duration_min + params.b).powf(params.c)
}

/// Generate IDF curve for a list of durations.
pub fn idf_curve(durations_min: &[f64], params: &IdfParams) -> IdfResult {
    let intensities: Vec<f64> = durations_min.iter().map(|&t| idf_intensity(t, params)).collect();
    let depths: Vec<f64> = intensities.iter().zip(durations_min.iter()).map(|(&i, &t)| i * t / 60.0).collect();
    IdfResult {
        return_period_yr: 10.0,
        durations_min: durations_min.to_vec(),
        intensities_mmh: intensities,
        depths_mm: depths,
    }
}

/// Fit IDF params from data using log-transform linear regression with grid search on b.
pub fn idf_fit_params(durations_min: &[f64], intensities_mmh: &[f64]) -> Option<IdfParams> {
    let n = durations_min.len();
    if n < 3 {
        return None;
    }
    let mut best_b = 0.0f64;
    let mut best_r2 = -1.0f64;
    let mut best_a = 1.0;
    let mut best_c = 1.0;

    for b_guess in (0..500).map(|i| i as f64 * 0.1) {
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xx = 0.0;
        let mut sum_xy = 0.0;
        for i in 0..n {
            let t = durations_min[i] + b_guess;
            if t <= 0.0 || intensities_mmh[i] <= 0.0 {
                continue;
            }
            let x = t.ln();
            let y = intensities_mmh[i].ln();
            sum_x += x;
            sum_y += y;
            sum_xx += x * x;
            sum_xy += x * y;
        }
        let nn = n as f64;
        let denom = nn * sum_xx - sum_x * sum_x;
        if denom.abs() < 1e-12 {
            continue;
        }
        // Regression: y = ln(i) = ln(a) - c*ln(t+b) = intercept + slope*ln(t+b)
        // slope = -c, intercept = ln(a)
        let slope = (nn * sum_xy - sum_x * sum_y) / denom;
        let intercept = (sum_y - slope * sum_x) / nn;
        let a_hat = intercept.exp();
        let c_true = -slope;

        // Compute R²
        let mut ss_res = 0.0;
        let mut ss_tot = 0.0;
        let mean_y = sum_y / nn;
        for i in 0..n {
            let t = durations_min[i] + b_guess;
            if t <= 0.0 || intensities_mmh[i] <= 0.0 {
                continue;
            }
            let y_pred = intercept + slope * t.ln();
            let y_obs = intensities_mmh[i].ln();
            ss_res += (y_obs - y_pred).powi(2);
            ss_tot += (y_obs - mean_y).powi(2);
        }
        let r2 = 1.0 - ss_res / ss_tot;
        if r2 > best_r2 {
            best_r2 = r2;
            best_b = b_guess;
            best_a = a_hat;
            best_c = c_true;
        }
    }
    if best_r2 < 0.0 {
        return None;
    }
    Some(IdfParams { a: best_a, b: best_b, c: best_c })
}

/// Scale IDF to different return period.
pub fn idf_return_period(
    base_params: &IdfParams,
    base_return_yr: f64,
    target_return_yr: f64,
    coef_a: f64,
    coef_b: f64,
) -> IdfParams {
    let scale = (coef_a + coef_b * target_return_yr.ln()) / (coef_a + coef_b * base_return_yr.ln());
    IdfParams {
        a: base_params.a * scale,
        b: base_params.b,
        c: base_params.c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idf_intensity() {
        let p = IdfParams { a: 1000.0, b: 10.0, c: 0.8 };
        let i = idf_intensity(30.0, &p);
        assert!(i > 0.0);
    }

    #[test]
    fn test_idf_curve() {
        let p = IdfParams { a: 1000.0, b: 10.0, c: 0.8 };
        let result = idf_curve(&[10.0, 30.0, 60.0], &p);
        assert_eq!(result.intensities_mmh.len(), 3);
        assert_eq!(result.depths_mm.len(), 3);
    }

    #[test]
    fn test_idf_fit_params() {
        let p = IdfParams { a: 500.0, b: 5.0, c: 0.7 };
        let durations = vec![5.0, 10.0, 20.0, 30.0, 60.0];
        let intensities: Vec<f64> = durations.iter().map(|&t| idf_intensity(t, &p)).collect();
        let fitted = idf_fit_params(&durations, &intensities);
        assert!(fitted.is_some());
        let f = fitted.unwrap();
        assert!((f.a / p.a - 1.0).abs() < 0.3);
    }

    #[test]
    fn test_idf_return_period() {
        let p = IdfParams { a: 500.0, b: 5.0, c: 0.7 };
        let scaled = idf_return_period(&p, 10.0, 100.0, 0.546, 0.459);
        assert!(scaled.a > p.a);
    }
}

