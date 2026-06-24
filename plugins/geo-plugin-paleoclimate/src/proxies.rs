/// δ¹⁸O → 海表温度 (SST) 转换。斜率约为 -4.5 °C/‰。
pub fn d18o_to_sst(d18o_permil: f64, slope: f64, intercept: f64) -> f64 {
    slope * d18o_permil + intercept
}

/// 冰芯 CH₄ 浓度 → 温度距平。
/// 梯度约 0.055 °C/ppb (简化线性)。
pub fn ice_core_temp_anomaly(ch4_ppb: f64, baseline_ppb: f64, gradient: f64) -> f64 {
    (ch4_ppb - baseline_ppb) * gradient
}

/// 综合代用指标反演温度。
/// proxies: [(proxy_type, value)]
///   "d18o": δ¹⁸O (‰)
///   "ch4": CH4 (ppb)
///   "pollen": 孢粉温带组分占比 (0-1)
///   "alkenone": U^k_37 不饱和指数
pub fn proxy_temperature(
    proxies: &[(&str, f64)],
    d18o_slope: f64,
    d18o_intercept: f64,
    ch4_baseline: f64,
    ch4_gradient: f64,
) -> f64 {
    let mut temps = Vec::new();

    for &(ptype, value) in proxies {
        match ptype {
            "d18o" => temps.push(d18o_to_sst(value, d18o_slope, d18o_intercept)),
            "ch4" => temps.push(ice_core_temp_anomaly(value, ch4_baseline, ch4_gradient)),
            "pollen" => {
                // 孢粉温带组分占比 → 温度距平
                let anom = (value - 0.5) * 6.0; // 0→-3°C, 1→+3°C
                temps.push(anom);
            }
            "alkenone" => {
                // U^k_37 → SST: T = (U^k_37 - 0.039) / 0.034 (Prahl et al.)
                let sst = (value - 0.039) / 0.034;
                temps.push(sst);
            }
            _ => {}
        }
    }

    if temps.is_empty() {
        return f64::NAN;
    }
    temps.iter().sum::<f64>() / temps.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_d18o_sst() {
        // 冰期 δ¹⁸O 更高 → SST 更冷
        let interglacial = d18o_to_sst(0.0, -4.5, 20.0);
        let glacial = d18o_to_sst(3.0, -4.5, 20.0);
        assert!(interglacial > glacial);
    }

    #[test]
    fn test_ch4_anomaly() {
        let a = ice_core_temp_anomaly(800.0, 700.0, 0.055);
        assert!((a - 5.5).abs() < 0.01); // 100 ppb × 0.055
    }

    #[test]
    fn test_proxy_combined() {
        let t = proxy_temperature(&[("d18o", 0.5), ("ch4", 750.0)], -4.5, 20.0, 700.0, 0.055);
        assert!(!t.is_nan());
        let expected_d18o = -4.5 * 0.5 + 20.0;
        let expected_ch4 = (750.0 - 700.0) * 0.055;
        let expected = (expected_d18o + expected_ch4) / 2.0;
        assert!((t - expected).abs() < 0.01);
    }

    #[test]
    fn test_pollen_proxy() {
        let t = proxy_temperature(&[("pollen", 0.8)], -4.5, 20.0, 700.0, 0.055);
        assert!((t - (0.8 - 0.5) * 6.0).abs() < 0.01);
    }

    #[test]
    fn test_alkenone_sst() {
        let t = proxy_temperature(&[("alkenone", 0.5)], -4.5, 20.0, 700.0, 0.055);
        let expected = (0.5 - 0.039) / 0.034;
        assert!((t - expected).abs() < 0.1);
    }

    #[test]
    fn test_empty_proxies() {
        let t = proxy_temperature(&[], -4.5, 20.0, 700.0, 0.055);
        assert!(t.is_nan());
    }
}
