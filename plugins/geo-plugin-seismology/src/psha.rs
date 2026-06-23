//! 概率地震危险性分析 (PSHA) — 地震源模型、超越概率曲线、危险性分解。
use serde::{Deserialize, Serialize};

/// 地震源。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeismicSource {
    /// 源名称
    pub name: String,
    /// 中心经度
    pub longitude: f64,
    /// 中心纬度
    pub latitude: f64,
    /// 年平均发生率 (≥ M_min)
    pub annual_rate: f64,
    /// b 值 (G-R 关系)
    pub b_value: f64,
    /// 最小震级
    pub m_min: f64,
    /// 最大震级
    pub m_max: f64,
}

/// PSHA 计算结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PshaResult {
    /// 场地位置
    pub site_lon: f64,
    pub site_lat: f64,
    /// 超越概率曲线 [(pga_return_period, exceedance_probability)]
    pub hazard_curve: Vec<HazardPoint>,
    /// 均匀危险谱
    pub uniform_hazard_spectrum: Vec<SpectrumHazardPoint>,
    /// 危险性分解 (最大贡献源)
    pub deaggregation: Vec<DeaggBin>,
}

/// 超越概率点。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HazardPoint {
    pub return_period_years: f64,
    pub pga_g: f64,
    pub exceedance_probability: f64,
}

/// 均匀危险谱点。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectrumHazardPoint {
    pub period_s: f64,
    pub sa_g: f64,
    pub return_period_years: f64,
}

/// 危险性分解 bin。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeaggBin {
    pub source_name: String,
    pub mag_bin: String,
    pub distance_km: f64,
    pub contribution_pct: f64,
}

/// Gutenberg-Richter 累积发生率: N(M≥m) = 10^(a - b*m)
pub fn gutenberg_richter_rate(a: f64, b: f64, m: f64) -> f64 {
    10.0_f64.powf(a - b * m)
}

/// 从发生率计算 a 值。
pub fn a_from_rate(annual_rate: f64, b: f64, m_min: f64) -> f64 {
    annual_rate.log10() + b * m_min
}

/// 给定震级区间内的年发生率。
pub fn annual_rate_in_interval(annual_rate: f64, b: f64, m_min: f64, m1: f64, m2: f64) -> f64 {
    if m2 <= m_min { return 0.0; }
    let m1c = m1.max(m_min);
    if m1c >= m2 { return 0.0; }
    let beta = b * 2.302585; // ln(10)
    annual_rate * ((-beta * (m1c - m_min)).exp() - (-beta * (m2 - m_min)).exp())
}

/// PSHA 超越概率 (简化: 单源+衰减方差)。
///
/// 给定场地到源的距离，计算 PGA 超越 im_value 的年超越概率。
pub fn psha_exceedance(
    source: &SeismicSource,
    distance_km: f64,
    im_value: f64,
    site_class: &str,
) -> f64 {
    if distance_km <= 0.0 { return 0.0; }
    let beta = source.b_value * 2.302585;
    // 对震级区间求和 (M_min ~ M_max, 步长 0.1)
    let n_bins = ((source.m_max - source.m_min) / 0.1) as usize;
    let mut total_rate = 0.0;
    for i in 0..n_bins {
        let m1 = source.m_min + i as f64 * 0.1;
        let m2 = m1 + 0.1;
        let rate_m = annual_rate_in_interval(source.annual_rate, source.b_value, source.m_min, m1, m2);
        if rate_m <= 0.0 { continue; }
        let median_pga = crate::ground_motion::pga_from_mag_distance((m1 + m2) / 2.0, distance_km, site_class);
        if median_pga <= 0.0 { continue; }
        // 对数正态超越概率 (方差 σ_lnPGA ≈ 0.7)
        let sigma = 0.7;
        let ln_ratio = (im_value / median_pga).ln();
        let exceed = 1.0 - gaussian_cdf(ln_ratio / sigma);
        total_rate += rate_m * exceed;
    }
    total_rate.min(1.0)
}

/// 标准正态 CDF (近似, 非 stable erf)。
fn gaussian_cdf(x: f64) -> f64 {
    0.5 * (1.0 + x.signum() * (1.0 - (-2.0 / std::f64::consts::PI * x * x).exp()).sqrt().min(1.0).max(-1.0))
}

/// 构建完整 PSHA  hazard curve。
pub fn psha_hazard_curve(
    sources: &[SeismicSource],
    site_lon: f64,
    site_lat: f64,
    return_periods: &[f64],
    site_class: &str,
) -> Vec<HazardPoint> {
    let mut curve = Vec::new();
    for &rp in return_periods {
        // 目标年超越概率 = 1 / 重现期
        let target_prob = 1.0 / rp;
        // 二分法搜索 PGA
        let pga = binary_search_pga(sources, site_lon, site_lat, target_prob, site_class);
        let exceed = if pga > 0.0 { 1.0 / rp } else { 0.0 };
        curve.push(HazardPoint { return_period_years: rp, pga_g: pga, exceedance_probability: exceed });
    }
    curve
}

/// 二分法搜索指定超越概率的 PGA 值。
fn binary_search_pga(
    sources: &[SeismicSource],
    site_lon: f64,
    site_lat: f64,
    target_prob: f64,
    site_class: &str,
) -> f64 {
    let mut lo = 0.001;
    let mut hi = 2.0;
    for _ in 0..30 {
        let mid = (lo + hi) / 2.0;
        let total_exceed: f64 = sources.iter().map(|src| {
            let dist_km = haversine_km(site_lon, site_lat, src.longitude, src.latitude);
            psha_exceedance(src, dist_km, mid, site_class)
        }).sum();
        if total_exceed > target_prob {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    (lo + hi) / 2.0
}

/// Haversine 距离 (km)。
fn haversine_km(lon1: f64, lat1: f64, lon2: f64, lat2: f64) -> f64 {
    let r = 6371.0;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2) + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    2.0 * r * a.sqrt().asin()
}

/// 均匀危险谱 (UHS): 对各周期重复 PSHA。
pub fn uniform_hazard_spectrum(
    sources: &[SeismicSource],
    site_lon: f64,
    site_lat: f64,
    return_periods: &[f64],
    periods: &[f64],
    damping: f64,
    site_class: &str,
) -> Vec<SpectrumHazardPoint> {
    let mut uhs = Vec::new();
    for &rp in return_periods {
        // 先算该重现期的 PGA hazard
        let target_prob = 1.0 / rp;
        let pga = binary_search_pga(sources, site_lon, site_lat, target_prob, site_class);
        for &period in periods {
            let sa = if period < 0.1 {
                pga * (1.0 + (1.5 - 1.0) * period / 0.1)
            } else if period < 0.4 {
                pga * 1.5
            } else {
                pga * 1.5 * 0.4 / period.max(0.4)
            };
            uhs.push(SpectrumHazardPoint { period_s: period, sa_g: sa.min(5.0), return_period_years: rp });
        }
    }
    uhs
}

/// 危险性分解: 各震源贡献占比。
pub fn deaggregation(
    sources: &[SeismicSource],
    site_lon: f64,
    site_lat: f64,
    pga_g: f64,
    site_class: &str,
) -> Vec<DeaggBin> {
    let mut bins = Vec::new();
    let total: f64 = sources.iter().map(|src| {
        let dist = haversine_km(site_lon, site_lat, src.longitude, src.latitude);
        psha_exceedance(src, dist, pga_g, site_class)
    }).sum();
    for src in sources {
        let dist = haversine_km(site_lon, site_lat, src.longitude, src.latitude);
        let exceed = psha_exceedance(src, dist, pga_g, site_class);
        let pct = if total > 0.0 { exceed / total * 100.0 } else { 0.0 };
        bins.push(DeaggBin {
            source_name: src.name.clone(),
            mag_bin: format!("{:.1}-{:.1}", src.m_min, src.m_max),
            distance_km: dist,
            contribution_pct: (pct * 100.0).round() / 100.0,
        });
    }
    bins.sort_by(|a, b| b.contribution_pct.partial_cmp(&a.contribution_pct).unwrap_or(std::cmp::Ordering::Equal));
    bins
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_source() -> SeismicSource {
        SeismicSource {
            name: "test_fault".into(),
            longitude: 104.0,
            latitude: 30.0,
            annual_rate: 0.05,
            b_value: 1.0,
            m_min: 4.5,
            m_max: 7.5,
        }
    }

    #[test]
    fn test_gutenberg_richter() {
        let rate = gutenberg_richter_rate(4.0, 1.0, 6.0);
        assert!(rate > 0.0 && rate < 100.0);
    }

    #[test]
    fn test_annual_rate_in_interval() {
        let rate = annual_rate_in_interval(0.05, 1.0, 4.5, 5.0, 6.0);
        assert!(rate > 0.0 && rate < 0.05);
    }

    #[test]
    fn test_psha_exceedance() {
        let src = make_test_source();
        let prob = psha_exceedance(&src, 30.0, 0.1, "II");
        assert!(prob >= 0.0 && prob <= 1.0);
    }

    #[test]
    fn test_psha_hazard_curve() {
        let sources = vec![make_test_source()];
        let curve = psha_hazard_curve(&sources, 104.5, 30.5, &[100.0, 475.0], "II");
        assert_eq!(curve.len(), 2);
        assert!(curve[0].pga_g > 0.001 || curve[1].pga_g > 0.001);
    }

    #[test]
    fn test_uniform_hazard_spectrum() {
        let sources = vec![make_test_source()];
        let periods = vec![0.1, 0.4, 1.0];
        let uhs = uniform_hazard_spectrum(&sources, 104.5, 30.5, &[475.0], &periods, 0.05, "II");
        assert_eq!(uhs.len(), 3);
    }

    #[test]
    fn test_deaggregation() {
        let sources = vec![make_test_source()];
        let bins = deaggregation(&sources, 104.5, 30.5, 0.1, "II");
        assert_eq!(bins.len(), 1);
        assert!(bins[0].contribution_pct > 0.0);
    }

    #[test]
    fn test_haversine() {
        let d = haversine_km(104.0, 30.0, 104.1, 30.05);
        assert!(d > 5.0 && d < 20.0);
    }
}
