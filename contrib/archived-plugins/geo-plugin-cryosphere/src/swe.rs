//! 冰雪水当量 (SWE) — 融雪模型
use serde::{Deserialize, Serialize};

/// 融雪类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MeltType {
    DegreeDay,
    EnergyBalance,
}

/// SWE 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweResult {
    pub swe_mm: Vec<f64>,
    pub melt_mm: Vec<f64>,
    pub total_melt_mm: f64,
    pub max_swe_mm: f64,
    pub melt_days: usize,
}

/// 温度指数法融雪 (degree-day): M = DDF × T
pub fn snowmelt_degree_day(temp_c: &[f64], dd_factor_mm_c_day: f64) -> Vec<f64> {
    temp_c
        .iter()
        .map(|&t| if t > 0.0 { t * dd_factor_mm_c_day } else { 0.0 })
        .collect()
}

/// 简化解能量平衡融雪
pub fn snowmelt_energy_balance(
    sw_net: &[f64],
    lw_net: &[f64],
    sensible: &[f64],
    latent: &[f64],
) -> Vec<f64> {
    let lf = 334.0; // latent heat of fusion kJ/kg
    sw_net
        .iter()
        .zip(lw_net.iter())
        .zip(sensible.iter())
        .zip(latent.iter())
        .map(|(((sw, lw), sh), lh)| {
            let total_w_m2 = sw + lw + sh + lh;
            if total_w_m2 > 0.0 {
                total_w_m2 / lf * 3600.0 * 24.0 / 1000.0
            } else {
                0.0
            }
        })
        .collect()
}

/// SWE 累积: 降水 > 阈值时降雪累积
pub fn swe_accumulation(precip_mm: &[f64], temp_c: &[f64], rain_snow_threshold: f64) -> Vec<f64> {
    let mut swe = 0.0;
    precip_mm
        .iter()
        .zip(temp_c.iter())
        .map(|(&p, &t)| {
            if t <= rain_snow_threshold {
                swe += p;
            } else {
                swe = (swe - p * 0.5).max(0.0);
            }
            swe
        })
        .collect()
}

/// 完整 SWE + 融雪模拟
pub fn simulate_swe(
    precip_mm: &[f64],
    temp_c: &[f64],
    dd_factor_mm_c_day: f64,
    rain_snow_c: f64,
) -> SweResult {
    let swe = swe_accumulation(precip_mm, temp_c, rain_snow_c);
    let melt = snowmelt_degree_day(temp_c, dd_factor_mm_c_day);
    let total_melt: f64 = melt.iter().sum();
    let max_swe = swe.iter().cloned().fold(0.0_f64, f64::max);
    let melt_days = melt.iter().filter(|&&m| m > 0.0).count();
    SweResult {
        swe_mm: swe,
        melt_mm: melt,
        total_melt_mm: total_melt,
        max_swe_mm: max_swe,
        melt_days,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_degree_day_melt() {
        let temps = vec![-5.0, 0.0, 5.0, 10.0];
        let melt = snowmelt_degree_day(&temps, 3.0);
        assert_eq!(melt[0], 0.0);
        assert!((melt[2] - 15.0).abs() < 1e-6);
    }

    #[test]
    fn test_energy_balance() {
        let sw = vec![200.0, 0.0];
        let lw = vec![-50.0, -80.0];
        let sh = vec![30.0, 10.0];
        let lh = vec![20.0, 5.0];
        let melt = snowmelt_energy_balance(&sw, &lw, &sh, &lh);
        assert!(melt[0] > 0.0);
        assert_eq!(melt[1], 0.0);
    }

    #[test]
    fn test_swe_accumulation() {
        let p = vec![10.0, 10.0, 10.0, 10.0];
        let t = vec![-5.0, -2.0, 2.0, 5.0];
        let swe = swe_accumulation(&p, &t, 0.0);
        assert!((swe[1] - 20.0).abs() < 1e-6);
        assert!(swe[3] < swe[2]);
    }

    #[test]
    fn test_simulate_swe() {
        let p = vec![5.0; 30];
        let t: Vec<f64> = (0..30).map(|i| if i < 15 { -3.0 } else { 5.0 }).collect();
        let r = simulate_swe(&p, &t, 2.0, 0.0);
        assert!(r.max_swe_mm > 0.0);
        assert!(r.melt_days > 0);
    }
}
