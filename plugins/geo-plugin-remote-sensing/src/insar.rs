//! InSAR 模块 — 相干性计算、相位差分、相位解缠（简化 Goldstein）、形变估计。
use serde::{Deserialize, Serialize};

/// InSAR 处理结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsarResult {
    /// 相干性图 (0-1)
    pub coherence: Vec<f64>,
    /// 缠绕相位 (-pi 到 pi)
    pub wrapped_phase: Vec<f64>,
    /// 解缠相位 (radians)
    pub unwrapped_phase: Vec<f64>,
    /// 视线向形变 (m) (正值 = 朝向卫星)
    pub los_displacement_m: Vec<f64>,
    /// 相干性均值
    pub mean_coherence: f64,
    /// 形变均值 (mm)
    pub mean_displacement_mm: f64,
    /// 是否有解缠异常区域
    pub unwrap_anomaly_regions: usize,
}

/// 计算两景 SLC 影像的相干性。
pub fn coherence(master: &[f64], slave: &[f64], window: usize) -> Vec<f64> {
    let n = master.len().min(slave.len());
    let half = window / 2;
    (0..n)
        .map(|i| {
            let mut sum_m2 = 0.0;
            let mut sum_s2 = 0.0;
            let mut sum_ms = 0.0;
            let mut count = 0;
            let start = if i >= half { i - half } else { 0 };
            let end = (i + half + 1).min(n);
            for j in start..end {
                let m = master[j];
                let s = slave[j];
                sum_m2 += m * m;
                sum_s2 += s * s;
                sum_ms += m * s;
                count += 1;
            }
            if count > 1 && sum_m2 > 0.0 && sum_s2 > 0.0 {
                let gamma = sum_ms / (sum_m2.sqrt() * sum_s2.sqrt());
                gamma.clamp(0.0, 1.0)
            } else {
                0.0
            }
        })
        .collect()
}

/// 计算缠绕相位。
pub fn wrapped_phase(master: &[f64], slave: &[f64], phase_diff: Option<&[f64]>) -> Vec<f64> {
    if let Some(pd) = phase_diff {
        pd.iter()
            .map(|&p| {
                let wrapped = p % (2.0 * std::f64::consts::PI);
                if wrapped > std::f64::consts::PI {
                    wrapped - 2.0 * std::f64::consts::PI
                } else if wrapped < -std::f64::consts::PI {
                    wrapped + 2.0 * std::f64::consts::PI
                } else {
                    wrapped
                }
            })
            .collect()
    } else {
        let n = master.len().min(slave.len());
        (0..n)
            .map(|i| {
                let m = master[i];
                let s = slave[i];
                if m > 0.0 && s > 0.0 {
                    let real = (m * s) / (m.max(s));
                    let imag = (m - s) / (m + s + 1e-10);
                    imag.atan2(real)
                } else {
                    0.0
                }
            })
            .collect()
    }
}

/// 简化 Goldstein 枝切法相位解缠。
pub fn unwrap_phase(
    wrapped: &[f64],
    coherence_values: &[f64],
    cols: usize,
    coherence_threshold: f64,
) -> (Vec<f64>, usize) {
    let n = wrapped.len();
    let rows = n / cols;
    if rows == 0 {
        return (wrapped.to_vec(), 0);
    }

    let mut unwrapped = wrapped.to_vec();
    let mut anomaly_count = 0;

    let mut branch_cut = vec![false; n];
    for i in 0..n {
        if coherence_values.get(i).copied().unwrap_or(0.0) < coherence_threshold {
            branch_cut[i] = true;
        }
    }

    // Row-wise integration
    for row in 0..rows {
        let row_start = row * cols;
        for col in 1..cols {
            let idx = row_start + col;
            let prev = idx - 1;
            if branch_cut[idx] || branch_cut[prev] {
                continue;
            }
            let diff = wrapped[idx] - wrapped[prev];
            let corrected = if diff > std::f64::consts::PI {
                diff - 2.0 * std::f64::consts::PI
            } else if diff < -std::f64::consts::PI {
                diff + 2.0 * std::f64::consts::PI
            } else {
                diff
            };
            unwrapped[idx] = unwrapped[prev] + corrected;
        }
    }

    // Column-wise refinement
    for col in 0..cols {
        for row in 1..rows {
            let idx = row * cols + col;
            let above = (row - 1) * cols + col;
            if branch_cut[idx] || branch_cut[above] {
                anomaly_count += 1;
                continue;
            }
            let diff = unwrapped[idx] - unwrapped[above];
            if diff.abs() > std::f64::consts::PI {
                let corrected = if diff > std::f64::consts::PI {
                    unwrapped[above] + diff - 2.0 * std::f64::consts::PI
                } else {
                    unwrapped[above] + diff + 2.0 * std::f64::consts::PI
                };
                unwrapped[idx] = corrected;
            }
        }
    }

    (unwrapped, anomaly_count)
}

/// 从解缠相位计算视线向形变: dr = lambda * dphi / (4*pi)
pub fn los_displacement(
    unwrapped_phase: &[f64],
    baseline_phase: Option<&[f64]>,
    wavelength_cm: f64,
) -> Vec<f64> {
    let lambda_m = wavelength_cm / 100.0;
    let factor = lambda_m / (4.0 * std::f64::consts::PI);

    if let Some(baseline) = baseline_phase {
        let n = unwrapped_phase.len().min(baseline.len());
        (0..n)
            .map(|i| (unwrapped_phase[i] - baseline[i]) * factor)
            .collect()
    } else {
        let mean_phase: f64 =
            unwrapped_phase.iter().copied().sum::<f64>() / unwrapped_phase.len().max(1) as f64;
        unwrapped_phase
            .iter()
            .map(|&p| (p - mean_phase) * factor)
            .collect()
    }
}

/// 完整 InSAR 处理管线。
pub fn full_insar_pipeline(
    master: &[f64],
    slave: &[f64],
    window: usize,
    cols: usize,
    coherence_threshold: f64,
    wavelength_cm: f64,
    phase_diff: Option<&[f64]>,
) -> InsarResult {
    let coh = coherence(master, slave, window);
    let wp = wrapped_phase(master, slave, phase_diff);
    let (uwp, anomalies) = unwrap_phase(&wp, &coh, cols, coherence_threshold);
    let disp = los_displacement(&uwp, None, wavelength_cm);

    let mean_coh = if coh.is_empty() {
        0.0
    } else {
        coh.iter().sum::<f64>() / coh.len() as f64
    };
    let mean_disp_mm = if disp.is_empty() {
        0.0
    } else {
        disp.iter().sum::<f64>() / disp.len() as f64 * 1000.0
    };

    InsarResult {
        coherence: coh,
        wrapped_phase: wp,
        unwrapped_phase: uwp,
        los_displacement_m: disp,
        mean_coherence: mean_coh,
        mean_displacement_mm: mean_disp_mm,
        unwrap_anomaly_regions: anomalies,
    }
}

/// 将形变数组转为形变等级。
pub fn displacement_class(disp_mm: f64) -> &'static str {
    if disp_mm.abs() < 5.0 {
        "稳定"
    } else if disp_mm.abs() < 15.0 {
        "轻微"
    } else if disp_mm.abs() < 30.0 {
        "中等"
    } else if disp_mm.abs() < 50.0 {
        "显著"
    } else {
        "剧烈"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coherence_uniform() {
        let m = vec![1.0; 100];
        let s = vec![1.0; 100];
        let coh = coherence(&m, &s, 3);
        assert!((coh[50] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_coherence_anti_correlated() {
        let m = vec![1.0; 100];
        let s: Vec<f64> = (0..100)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let coh = coherence(&m, &s, 3);
        assert!(coh[50] < 0.3);
    }

    #[test]
    fn test_wrapped_phase() {
        let m = vec![1.0; 10];
        let s = vec![0.5; 10];
        let wp = wrapped_phase(&m, &s, None);
        assert_eq!(wp.len(), 10);
        for &p in &wp {
            assert!(p >= -std::f64::consts::PI && p <= std::f64::consts::PI);
        }
    }

    #[test]
    fn test_wrapped_phase_with_diff() {
        let pd = vec![0.1, 3.2, 6.5, -3.0];
        let wp = wrapped_phase(&[], &[], Some(&pd));
        assert!((wp[0] - 0.1).abs() < 1e-6);
        // 3.2 rad -> wrapped to 3.2 - 2pi = 3.2 - 6.283 = -3.083
        assert!((wp[1] + 3.083).abs() < 0.01);
    }

    #[test]
    fn test_unwrap_phase() {
        let mut wrapped = Vec::new();
        for i in 0..100 {
            let phase = (i as f64) * 0.2;
            let w = phase % (2.0 * std::f64::consts::PI);
            wrapped.push(if w > std::f64::consts::PI {
                w - 2.0 * std::f64::consts::PI
            } else {
                w
            });
        }
        let coh = vec![0.8; 100];
        let (unwrapped, anomalies) = unwrap_phase(&wrapped, &coh, 10, 0.3);
        assert!(anomalies < 10);
        // Overall monotonic: no more than 10 non-increasing steps
        let decreasing: i32 = (1..unwrapped.len())
            .map(|i| {
                if unwrapped[i] < unwrapped[i - 1] - 0.5 {
                    1
                } else {
                    0
                }
            })
            .sum();
        assert!(decreasing < 5);
        let end_val = unwrapped.last().copied().unwrap_or(0.0);
        assert!(end_val > 5.0);
    }

    #[test]
    fn test_los_displacement() {
        let phase = vec![0.0, std::f64::consts::PI / 2.0, std::f64::consts::PI];
        let disp = los_displacement(&phase, None, 5.6);
        let wavelength_m = 0.056;
        let factor = wavelength_m / (4.0 * std::f64::consts::PI);
        let mean = (0.0 + std::f64::consts::PI / 2.0 + std::f64::consts::PI) / 3.0;
        let expected_disp0 = (0.0 - mean) * factor;
        assert!((disp[0] - expected_disp0).abs() < 1e-10);
        // disp[2] = (pi - pi/2) * lambda / 4pi = pi/2 * lambda/4pi = lambda/8
        let expected_disp2 = (std::f64::consts::PI - mean) * factor;
        assert!((disp[2] - expected_disp2).abs() < 1e-10);
    }

    #[test]
    fn test_full_pipeline() {
        let master: Vec<f64> = (0..400)
            .map(|i| (i as f64 * 0.1).sin().abs() + 0.5)
            .collect();
        let slave: Vec<f64> = (0..400)
            .map(|i| ((i as f64 * 0.1) + 0.5).sin().abs() + 0.5)
            .collect();
        let result = full_insar_pipeline(&master, &slave, 5, 20, 0.3, 5.6, None);
        assert_eq!(result.coherence.len(), 400);
        assert_eq!(result.los_displacement_m.len(), 400);
        assert!(result.mean_coherence > 0.0);
    }

    #[test]
    fn test_displacement_class() {
        assert_eq!(displacement_class(0.0), "稳定");
        assert_eq!(displacement_class(10.0), "轻微");
        assert_eq!(displacement_class(20.0), "中等");
        assert_eq!(displacement_class(40.0), "显著");
        assert_eq!(displacement_class(60.0), "剧烈");
    }
}
