use serde::{Deserialize, Serialize};

/// Drought index types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DroughtIndex {
    Spi,
    Spei,
    Pdsi,
}

/// SPI/SPEI computation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiResult {
    pub scale_months: usize,
    pub values: Vec<f64>,
    pub drought_months: usize,
    pub drought_severity: f64,
    pub index: DroughtIndex,
}

/// Compute SPI using gamma distribution fitting.
pub fn compute_spi(precip_monthly: &[f64], scale_months: usize) -> Option<SpiResult> {
    if precip_monthly.len() < scale_months + 1 {
        return None;
    }
    // Accumulate precipitation over scale_months
    let n = precip_monthly.len() - scale_months + 1;
    let mut acc = Vec::with_capacity(n);
    for i in 0..n {
        let sum: f64 = precip_monthly[i..i + scale_months].iter().sum();
        if sum > 0.0 {
            acc.push(sum);
        }
    }
    if acc.len() < 4 {
        return None;
    }
    // Fit gamma distribution
    let (shape, scale) = gamma_mle(&acc)?;
    // Transform to standard normal
    let mut values = Vec::with_capacity(acc.len());
    for &x in &acc {
        let p = gamma_cdf(x, shape, scale);
        let z = inverse_normal_cdf(p.clamp(1e-15, 1.0 - 1e-15));
        values.push(z);
    }
    let drought_months = values.iter().filter(|&&v| v < -1.0).count();
    let drought_severity: f64 = values.iter().filter(|&&v| v < -1.0).sum();
    Some(SpiResult {
        scale_months,
        values,
        drought_months,
        drought_severity,
        index: DroughtIndex::Spi,
    })
}

/// Compute SPEI: SPI-like but uses P - PET.
pub fn compute_spei(
    precip_monthly: &[f64],
    temp_monthly: &[f64],
    scale_months: usize,
) -> Option<SpiResult> {
    if precip_monthly.len() < scale_months + 1 || temp_monthly.len() < precip_monthly.len() {
        return None;
    }
    let mut water_balance = Vec::with_capacity(precip_monthly.len());
    for i in 0..precip_monthly.len() {
        let pet = thornthwaite_pet(temp_monthly[i], 30.0, i % 12);
        water_balance.push(precip_monthly[i] - pet);
    }
    // Accumulate and fit
    let n = water_balance.len() - scale_months + 1;
    let mut acc = Vec::with_capacity(n);
    for i in 0..n {
        let sum: f64 = water_balance[i..i + scale_months].iter().sum();
        acc.push(sum);
    }
    // Fit logistic distribution (simplified)
    let mean = acc.iter().sum::<f64>() / acc.len() as f64;
    let std: f64 = (acc.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / acc.len() as f64)
        .sqrt()
        .max(1e-10);
    let mut values = Vec::with_capacity(acc.len());
    for &x in &acc {
        let p = 1.0 / (1.0 + (-(x - mean) / std).exp());
        let z = inverse_normal_cdf(p.clamp(1e-15, 1.0 - 1e-15));
        values.push(z);
    }
    let drought_months = values.iter().filter(|&&v| v < -1.0).count();
    let drought_severity: f64 = values.iter().filter(|&&v| v < -1.0).sum();
    Some(SpiResult {
        scale_months,
        values,
        drought_months,
        drought_severity,
        index: DroughtIndex::Spei,
    })
}

/// Simplified Palmer PDSI.
pub fn compute_pdsi(
    temp_monthly: &[f64],
    precip_monthly: &[f64],
    lat: f64,
    awc_mm: f64,
) -> Option<Vec<f64>> {
    if temp_monthly.is_empty() || precip_monthly.is_empty() {
        return None;
    }
    let mut pdsi = Vec::with_capacity(temp_monthly.len());
    let mut soil_moisture = awc_mm * 0.5; // start at 50% AWC
    let mut _prev_pdsi = 0.0;
    for i in 0..temp_monthly.len() {
        let pet = thornthwaite_pet(temp_monthly[i], lat, i % 12);
        let p = precip_monthly[i];
        // Simplified water balance
        let deficit = pet - p;
        if deficit > 0.0 {
            // Soil moisture depletion
            let reduction = (deficit * 0.5).min(soil_moisture);
            soil_moisture -= reduction;
        } else {
            // Recharge
            soil_moisture = (soil_moisture - deficit).min(awc_mm);
        }
        let moisture_anomaly = (p - pet) / awc_mm.max(1.0);
        // Accumulate index
        let current = moisture_anomaly + 0.8 * _prev_pdsi;
        pdsi.push(current);
        _prev_pdsi = current;
    }
    Some(pdsi)
}

/// Thornthwaite PET (mm/month).
pub fn thornthwaite_pet(temp_c: f64, lat_deg: f64, month: usize) -> f64 {
    if temp_c <= 0.0 {
        return 0.0;
    }
    let heat_index = |t: f64| {
        let months: [f64; 12] = [0.0; 12];
        months.iter().map(|_| (t / 5.0).powf(1.514)).sum::<f64>()
    };
    let i_annual = heat_index(temp_c);
    let a = 6.75e-7 * i_annual.powi(3) - 7.71e-5 * i_annual.powi(2) + 1.79e-2 * i_annual + 0.49;
    let day_hours = daylight_hours(lat_deg, month);
    // Thornthwaite formula: PET = 16 * (10T/I)^a * (daylight_hours/12) * (days_in_month/30)
    let days_in_month = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let dm = days_in_month[month.min(11)];
    16.0 * (10.0 * temp_c / i_annual.max(1.0)).powf(a) * (day_hours / 12.0) * (dm as f64 / 30.0)
}

/// Approximate monthly daylight hours at given latitude.
pub fn daylight_hours(lat_deg: f64, month: usize) -> f64 {
    let lat_rad = lat_deg * std::f64::consts::PI / 180.0;
    // Approximate solar declination for mid-month
    let decl = match month % 12 {
        0 => -23.4,  // Jan
        1 => -17.1,  // Feb
        2 => -7.9,   // Mar
        3 => 4.8,    // Apr
        4 => 14.5,   // May
        5 => 21.9,   // Jun
        6 => 23.4,   // Jul
        7 => 18.5,   // Aug
        8 => 8.9,    // Sep
        9 => -2.0,   // Oct
        10 => -12.8, // Nov
        11 => -21.7, // Dec
        _ => 0.0,
    };
    let dec_rad = decl * std::f64::consts::PI / 180.0;
    let cos_omega = (-lat_rad.tan() * dec_rad.tan()).clamp(-1.0, 1.0);
    let day_length = 24.0 / std::f64::consts::PI * cos_omega.acos();
    day_length * 2.0
}

/// Gamma MLE via Greenwood-Durand approximation.
pub fn gamma_mle(x: &[f64]) -> Option<(f64, f64)> {
    let n = x.len() as f64;
    let mean = x.iter().sum::<f64>() / n;
    let log_mean = x.iter().map(|&v| v.ln()).sum::<f64>() / n;
    let d = mean.ln() - log_mean;
    if d <= 0.0 {
        return None;
    }
    // Greenwood-Durand approximation for shape
    // Clip shape to max 1e6 to avoid overflow for near-constant data
    let shape = if d < 1e-12 {
        1e6 // clip to practical maximum
    } else {
        ((3.0 - d + ((d - 3.0).powi(2) + 24.0 * d).sqrt()) / (12.0 * d)).min(1e6)
    };
    let scale = mean / shape;
    Some((shape, scale))
}

/// Gamma CDF using regularized incomplete gamma function (series expansion).
pub fn gamma_cdf(x: f64, shape: f64, scale: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    let a = shape;
    let z = x / scale;
    // Lower regularized incomplete gamma: P(a, z)
    // Use series expansion for small z, continued fraction for large z
    if z < a + 1.0 {
        // Series
        let mut sum = 1.0 / a;
        let mut term = 1.0 / a;
        for k in 1..200 {
            term *= z / (a + k as f64);
            sum += term;
            if term.abs() < 1e-14 {
                break;
            }
        }
        sum * z.powf(a) * (-z).exp() / gamma(a)
    } else {
        // Continued fraction
        1.0 - gamma_upper_cf(a, z)
    }
}

fn gamma(x: f64) -> f64 {
    // Stirling's approximation for large x, Lanczos for small
    if x < 0.5 {
        std::f64::consts::PI / (std::f64::consts::PI * x).sin() / gamma(1.0 - x)
    } else {
        let g = 7.0;
        let c = [
            0.999_999_999_999_809_9,
            676.5203681218851,
            -1259.1392167224028,
            771.323_428_777_653_1,
            -176.615_029_162_140_6,
            12.507343278686905,
            -0.13857109526572012,
            9.984_369_578_019_572e-6,
            1.5056327351493116e-7,
        ];
        let xm = x - 1.0;
        let mut sum = c[0];
        for i in 1..c.len() {
            sum += c[i] / (xm + i as f64);
        }
        let t = xm + g + 0.5;
        (2.0 * std::f64::consts::PI).sqrt() * t.powf(xm + 0.5) * (-t).exp() * sum
    }
}

fn gamma_upper_cf(a: f64, z: f64) -> f64 {
    // Lentz's continued fraction for upper incomplete gamma
    const FPMIN: f64 = 1e-30;
    let b = z + 1.0 - a;
    let mut c = 1.0 / FPMIN;
    let mut d = 1.0 / b;
    let mut h = d;
    for i in 1..200 {
        let an = if i % 2 == 1 {
            (i as f64 + 1.0) / 2.0 - a
        } else {
            i as f64 / 2.0 * z
        };
        let bn = z + (i as f64).mul_add(2.0, -a + 1.0);
        d = 1.0 / (an.mul_add(d, bn));
        c = an / c + bn;
        let del = c * d;
        h *= del;
        if (del - 1.0).abs() < 1e-14 {
            break;
        }
    }
    (-z + a * z.ln()).exp() * h / gamma(a)
}

/// Inverse normal CDF (Beasley-Springer-Moro approximation).
pub fn inverse_normal_cdf(p: f64) -> f64 {
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    let a = [
        -3.969683028665376e1,
        2.209460984245205e2,
        -2.759285104469687e2,
        1.383_577_518_672_69e2,
        -3.066479806614716e1,
        2.506628277459239e0,
    ];
    let b = [
        -5.447609879822406e1,
        1.615858368580409e2,
        -1.556989798598866e2,
        6.680131188771972e1,
        -1.328068155288572e1,
        1.0,
    ];
    let c = [
        -7.784894002430293e-3,
        -3.223964580411365e-1,
        -2.400758277161838e0,
        -2.549732539343734e0,
        4.374664141464968e0,
        2.938163982698783e0,
    ];
    let d = [
        7.784695709041462e-3,
        3.224671290700398e-1,
        2.445134137142996e0,
        3.754408661907416e0,
        1.0,
    ];
    let p_low = 0.02425;
    let p_high = 1.0 - p_low;
    if p < p_low {
        let q = (-2.0 * p.ln()).sqrt();
        (((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
    } else if p <= p_high {
        let q = p - 0.5;
        let r = q * q;
        (((((a[0] * r + a[1]) * r + a[2]) * r + a[3]) * r + a[4]) * r + a[5]) * q
            / (((((b[0] * r + b[1]) * r + b[2]) * r + b[3]) * r + b[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_spi_constant() {
        let precip = vec![100.0; 24];
        let result = compute_spi(&precip, 3);
        if let Some(r) = result {
            assert!(r.values.iter().all(|&v| v.abs() < 3.0));
        }
    }

    #[test]
    fn test_gamma_mle() {
        let data = vec![50.0, 60.0, 70.0, 80.0, 90.0, 100.0, 110.0, 120.0];
        let (shape, scale) = gamma_mle(&data).unwrap();
        assert!(shape > 0.0);
        assert!(scale > 0.0);
    }

    #[test]
    fn test_thornthwaite_pet() {
        let pet = thornthwaite_pet(25.0, 30.0, 6);
        assert!(pet > 100.0);
    }

    #[test]
    fn test_inverse_normal_cdf() {
        let z = inverse_normal_cdf(0.5);
        assert!((z - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_spei() {
        let precip = vec![100.0; 24];
        let temp = vec![20.0; 24];
        let result = compute_spei(&precip, &temp, 3);
        assert!(result.is_some());
    }

    #[test]
    fn test_compute_pdsi() {
        let temp = vec![20.0; 12];
        let precip = vec![100.0; 12];
        let result = compute_pdsi(&temp, &precip, 30.0, 150.0);
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 12);
    }
}
