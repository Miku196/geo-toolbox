//! Newmark 位移 — 地震滑坡永久位移评估（Jibson 2007）。

/// Newmark 永久位移（cm），Jibson (2007) 经验公式。
///
/// `log(Dn) = 0.215 + ln(1/ag) * ((1.46 * ln(ag) + 1.824) - 1.0)` 简化版：
/// `Dn = exp(2.401 * ln(ag_cm + 0.818))` 基于临界加速度比。
/// 实际使用: ac = (FS - 1)*g*sin(β), Dn(cm) = exp(2.401 * ln(PGA/ac) - 0.818)
pub fn newmark_displacement(slope_deg: f64, factor_of_safety: f64, pga_g: f64) -> f64 {
    let ac_g = (factor_of_safety - 1.0) * slope_deg.to_radians().sin();
    if ac_g <= 0.0 || pga_g <= ac_g {
        return 0.0;
    }
    // Jibson 2007: Dn = 0.0 if PGA <= ac
    // Dn (cm) = exp(2.401 * ln(PGA/ac) + 0.818)
    // In original Jibson: log10(Dn) = 2.401 * log10(PGA/ac) - 0.818 + 0.215
    let ratio = pga_g / ac_g;
    if ratio <= 1.0 {
        return 0.0;
    }
    // Jibson 2007 eq: Dn (cm) = 10^(2.401 * log10(ratio) - 0.818 + 0.215)
    // = 10^(2.401 * log10(ratio) - 0.603)
    let log_disp = 2.401 * ratio.log10() - 0.603;
    10.0_f64.powf(log_disp)
}

/// Newmark 滑坡危险性评估。
pub fn newmark_landslide_hazard(slope_deg: f64, fs: f64, pga_g: f64) -> serde_json::Value {
    let disp_cm = newmark_displacement(slope_deg, fs, pga_g);
    let hazard_level = if disp_cm < 1.0 {
        "low"
    } else if disp_cm < 5.0 {
        "moderate"
    } else if disp_cm < 15.0 {
        "high"
    } else {
        "very_high"
    };
    serde_json::json!({
        "displacement_cm": disp_cm,
        "hazard_level": hazard_level
    })
}

/// 批量区域 Newmark 评估。
pub fn regional_newmark(slopes: &[f64], fs_vals: &[f64], pga_g: f64) -> serde_json::Value {
    let n = slopes.len().min(fs_vals.len());
    let mut displacements = Vec::with_capacity(n);
    let mut hazard_counts: std::collections::HashMap<String, usize> = [
        ("low".into(), 0_usize),
        ("moderate".into(), 0),
        ("high".into(), 0),
        ("very_high".into(), 0),
    ]
    .into();
    let total = n as f64;

    for i in 0..n {
        let d = newmark_displacement(slopes[i], fs_vals[i], pga_g);
        let level = if d < 1.0 {
            "low"
        } else if d < 5.0 {
            "moderate"
        } else if d < 15.0 {
            "high"
        } else {
            "very_high"
        };
        *hazard_counts.get_mut(level).unwrap_or(&mut 0) += 1;
        displacements.push(d);
    }

    let mean_disp = if !displacements.is_empty() {
        displacements.iter().sum::<f64>() / displacements.len() as f64
    } else {
        0.0
    };

    serde_json::json!({
        "cells": n,
        "mean_displacement_cm": mean_disp,
        "max_displacement_cm": displacements.iter().copied().fold(0.0_f64, f64::max),
        "hazard_distribution": {
            "low_pct": hazard_counts["low"] as f64 / total * 100.0,
            "moderate_pct": hazard_counts["moderate"] as f64 / total * 100.0,
            "high_pct": hazard_counts["high"] as f64 / total * 100.0,
            "very_high_pct": hazard_counts["very_high"] as f64 / total * 100.0
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_newmark_displacement_basic() {
        // slope=30°, FS=1.1, PGA=0.3g → ~some cm
        let d = newmark_displacement(30.0, 1.1, 0.3);
        assert!(d > 0.0, "should be >0 for unstable slope");
    }

    #[test]
    fn test_newmark_displacement_stable() {
        // slope=10°, FS=2.0, PGA=0.1g → 0
        let d = newmark_displacement(10.0, 2.0, 0.1);
        assert_eq!(d, 0.0);
    }

    #[test]
    fn test_hazard_level_low() {
        let h = newmark_landslide_hazard(5.0, 2.0, 0.1);
        assert_eq!(h["hazard_level"], "low");
    }

    #[test]
    fn test_regional_newmark() {
        let slopes = vec![30.0, 15.0, 5.0];
        let fs = vec![1.1, 1.5, 2.0];
        let r = regional_newmark(&slopes, &fs, 0.3);
        assert_eq!(r["cells"], 3);
        assert!(r["mean_displacement_cm"].as_f64().unwrap_or(0.0) > 0.0);
    }
}
