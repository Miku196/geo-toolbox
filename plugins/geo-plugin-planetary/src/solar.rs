use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// 太阳位置结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolarPosition {
    /// 太阳高度角 (度)
    pub elevation_deg: f64,
    /// 太阳方位角 (度，正北顺时针)
    pub azimuth_deg: f64,
    /// 赤纬 (度)
    pub declination_deg: f64,
    /// 时角 (度)
    pub hour_angle_deg: f64,
    /// 地外辐射 (W/m²)
    pub extraterrestrial_radiation_wm2: f64,
}

/// 计算太阳赤纬 (度)。
/// day_of_year: 1-365/366
pub fn declination(day_of_year: u16) -> f64 {
    let gamma = 2.0 * PI * (day_of_year as f64 - 1.0) / 365.0;
    (0.006918 - 0.399912 * gamma.cos() + 0.070257 * gamma.sin() - 0.006758 * (2.0 * gamma).cos()
        + 0.000907 * (2.0 * gamma).sin()
        - 0.002697 * (3.0 * gamma).cos()
        + 0.001480 * (3.0 * gamma).sin())
    .to_degrees()
}

/// 计算时角 (度)。
pub fn hour_angle(longitude_deg: f64, utc_hour: f64) -> f64 {
    let solar_time = utc_hour + longitude_deg / 15.0;
    (solar_time - 12.0) * 15.0
}

/// 计算太阳高度角和方位角。
/// - lat_deg: 纬度
/// - lon_deg: 经度
/// - day_of_year: 年积日
/// - utc_hour: UTC 小时 (小数)
pub fn solar_elevation_azimuth(
    lat_deg: f64,
    lon_deg: f64,
    day_of_year: u16,
    utc_hour: f64,
) -> SolarPosition {
    let lat = lat_deg.to_radians();
    let dec = declination(day_of_year).to_radians();
    let h = hour_angle(lon_deg, utc_hour).to_radians();

    let sin_alt = lat.sin() * dec.sin() + lat.cos() * dec.cos() * h.cos();
    let elevation = sin_alt.asin();

    let _cos_az = (dec.sin() - lat.sin() * elevation.sin()) / (lat.cos() * elevation.cos());
    let azimuth = (-h)
        .sin()
        .atan2(h.cos() * lat.sin() - dec.cos().recip() * lat.cos() * dec.sin());

    let el_deg = elevation.to_degrees();
    let az_deg = (azimuth.to_degrees() + 360.0) % 360.0;
    let dec_deg = declination(day_of_year);
    let ha_deg = hour_angle(lon_deg, utc_hour);

    let etr = extraterrestrial_radiation(day_of_year);

    SolarPosition {
        elevation_deg: (el_deg * 100.0).round() / 100.0,
        azimuth_deg: (az_deg * 100.0).round() / 100.0,
        declination_deg: (dec_deg * 100.0).round() / 100.0,
        hour_angle_deg: (ha_deg * 100.0).round() / 100.0,
        extraterrestrial_radiation_wm2: (etr * 100.0).round() / 100.0,
    }
}

/// 地外辐射 (W/m²) — 太阳常数 × 日地距离修正。
pub fn extraterrestrial_radiation(day_of_year: u16) -> f64 {
    let d_r = 1.0 + 0.033 * (2.0 * PI * day_of_year as f64 / 365.0).cos();
    1361.0 * d_r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_declination_equinox() {
        let d80 = declination(80); // 春分附近
        let d266 = declination(266); // 秋分附近
        assert!(d80.abs() < 5.0);
        assert!(d266.abs() < 5.0);
    }

    #[test]
    fn test_declination_solstice() {
        let d172 = declination(172); // 夏至附近
        let d355 = declination(355); // 冬至附近
        assert!(d172 > 20.0);
        assert!(d355 < -20.0);
    }

    #[test]
    fn test_solar_elevation_noon_equator_equinox() {
        let sp = solar_elevation_azimuth(0.0, 0.0, 80, 12.0);
        assert!(sp.elevation_deg > 80.0);
        assert!(sp.elevation_deg <= 90.0);
    }

    #[test]
    fn test_solar_nighttime() {
        let sp = solar_elevation_azimuth(60.0, 0.0, 355, 0.0);
        // 高纬度冬季午夜 - 很可能低于地平线
        assert!(sp.elevation_deg < 0.0);
    }

    #[test]
    fn test_extraterrestrial_radiation() {
        let etr = extraterrestrial_radiation(1);
        assert!((etr - 1361.0 * (1.0 + 0.033)).abs() < 0.01);
    }

    #[test]
    fn test_hour_angle_noon() {
        let ha = hour_angle(0.0, 12.0);
        assert!(ha.abs() < 0.01);
    }
}
