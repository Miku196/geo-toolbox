//! UTM 坐标转换 — WGS84↔UTM。

/// UTM 常数
const WGS84_A: f64 = 6378137.0;
const WGS84_F: f64 = 1.0 / 298.257223563;
const UTM_K0: f64 = 0.9996;

/// 经度→UTM 带号（WGS84）。
pub fn utm_zone_from_lon(lon: f64) -> u8 {
    ((lon + 180.0) / 6.0).floor() as u8 % 60 + 1
}

/// UTM 带的中央子午线（°）。
pub fn utm_central_meridian(zone: u8) -> f64 {
    zone as f64 * 6.0 - 183.0
}

/// WGS84 经纬度 → UTM 投影坐标。
///
/// 返回 (东向_Easting, 北向_Northing, 带号, 北半球).
pub fn latlon_to_utm(lat: f64, lon: f64) -> (f64, f64, u8, bool) {
    let zone = utm_zone_from_lon(lon);
    let cm = utm_central_meridian(zone).to_radians();
    let phi = lat.to_radians();
    let _lam = lon.to_radians();
    let north = lat >= 0.0;

    let ecc_sq = 2.0 * WGS84_F - WGS84_F * WGS84_F; // e²
    let e_prime_sq = ecc_sq / (1.0 - ecc_sq); // e'²

    let sin_phi = phi.sin();
    let cos_phi = phi.cos();
    let tan_phi = phi.tan();

    let n = WGS84_A / (1.0 - ecc_sq * sin_phi * sin_phi).sqrt(); // nu = radius of curvature in prime vertical
    let t = tan_phi * tan_phi;
    let c = e_prime_sq * cos_phi * cos_phi;
    let a = (lon.to_radians() - cm) * cos_phi;

    // 子午线弧长 M
    let _e = ecc_sq.sqrt();
    let m = WGS84_A
        * ((1.0
            - ecc_sq / 4.0
            - 3.0 * ecc_sq * ecc_sq / 64.0
            - 5.0 * ecc_sq * ecc_sq * ecc_sq / 256.0)
            * phi
            - (3.0 * ecc_sq / 8.0
                + 3.0 * ecc_sq * ecc_sq / 32.0
                + 45.0 * ecc_sq * ecc_sq * ecc_sq / 1024.0)
                * (2.0 * phi).sin()
            + (15.0 * ecc_sq * ecc_sq / 256.0 + 45.0 * ecc_sq * ecc_sq * ecc_sq / 1024.0)
                * (4.0 * phi).sin()
            - (35.0 * ecc_sq * ecc_sq * ecc_sq / 3072.0) * (6.0 * phi).sin());

    let easting = UTM_K0
        * n
        * (a + (1.0 - t + c) * a.powi(3) / 6.0
            + (5.0 - 18.0 * t + t * t + 72.0 * c - 58.0 * e_prime_sq) * a.powi(5) / 120.0)
        + 500000.0;

    let northing_base = UTM_K0
        * (m + n
            * tan_phi
            * (a * a / 2.0
                + (5.0 - t + 9.0 * c + 4.0 * c * c) * a.powi(4) / 24.0
                + (61.0 - 58.0 * t + t * t + 600.0 * c - 330.0 * e_prime_sq) * a.powi(6) / 720.0));
    let northing = if north {
        northing_base
    } else {
        northing_base + 10000000.0
    };

    (easting, northing, zone, north)
}

/// UTM 投影坐标 → WGS84 经纬度。
pub fn utm_to_latlon(easting: f64, northing: f64, zone: u8, north_hemisphere: bool) -> (f64, f64) {
    let cm = utm_central_meridian(zone).to_radians();
    let ecc_sq = 2.0 * WGS84_F - WGS84_F * WGS84_F;
    let e_prime_sq = ecc_sq / (1.0 - ecc_sq);
    let e1 = (1.0 - (1.0 - ecc_sq).sqrt()) / (1.0 + (1.0 - ecc_sq).sqrt());

    let x = easting - 500000.0;
    let y = if north_hemisphere {
        northing
    } else {
        northing - 10000000.0
    };

    // 足量纬度 M / a
    let m = y / UTM_K0;
    let mu = m
        / (WGS84_A
            * (1.0
                - ecc_sq / 4.0
                - 3.0 * ecc_sq * ecc_sq / 64.0
                - 5.0 * ecc_sq * ecc_sq * ecc_sq / 256.0));

    // φ₁ 用迭代逼近
    let phi1 = mu
        + (3.0 * e1 / 2.0 - 27.0 * e1.powi(3) / 32.0) * (2.0 * mu).sin()
        + (21.0 * e1 * e1 / 16.0 - 55.0 * e1.powi(4) / 32.0) * (4.0 * mu).sin()
        + (151.0 * e1.powi(3) / 96.0) * (6.0 * mu).sin()
        + (1097.0 * e1.powi(4) / 512.0) * (8.0 * mu).sin();

    let sin_phi1 = phi1.sin();
    let cos_phi1 = phi1.cos();
    let tan_phi1 = phi1.tan();

    let n1 = WGS84_A / (1.0 - ecc_sq * sin_phi1 * sin_phi1).sqrt();
    let r1 = WGS84_A * (1.0 - ecc_sq) / (1.0 - ecc_sq * sin_phi1 * sin_phi1).powf(1.5);
    let t1 = tan_phi1 * tan_phi1;
    let c1 = e_prime_sq * cos_phi1 * cos_phi1;
    let d = x / (n1 * UTM_K0);

    let lat = phi1
        - (n1 * tan_phi1 / r1)
            * (d * d / 2.0
                - (5.0 + 3.0 * t1 + 10.0 * c1 - 4.0 * c1 * c1 - 9.0 * e_prime_sq) * d.powi(4)
                    / 24.0
                + (61.0 + 90.0 * t1 + 298.0 * c1 + 45.0 * t1 * t1
                    - 252.0 * e_prime_sq
                    - 3.0 * c1 * c1)
                    * d.powi(6)
                    / 720.0);

    let lon = cm
        + (d - (1.0 + 2.0 * t1 + c1) * d.powi(3) / 6.0
            + (5.0 - 2.0 * c1 + 28.0 * t1 - 3.0 * c1 * c1 + 8.0 * e_prime_sq + 24.0 * t1 * t1)
                * d.powi(5)
                / 120.0)
            / cos_phi1;

    (lat.to_degrees(), lon.to_degrees())
}

/// UTM 带信息 JSON。
pub fn utm_zone_info(lon: f64) -> serde_json::Value {
    let zone = utm_zone_from_lon(lon);
    let cm = utm_central_meridian(zone);
    serde_json::json!({
        "zone": zone,
        "central_meridian": cm,
        "hemisphere": if lon >= 0.0 { "north" } else { "south" }
    })
}

/// 判断是否在 UPS 极区（lat≥84° 或 ≤-80°）。
pub fn is_ups(lat: f64) -> bool {
    lat >= 84.0 || lat <= -80.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utm_zone_from_lon() {
        // 成都 104°E → zone 43 (84-90→1, 90-96→2, ..., 102-108→18)
        // 104°E: (104+180)/6 = 47.33, floor=47, 47%60=47. Wait: (104+180)/6 = 284/6 = 47.33, ceil?
        // Actually: zone = floor((lon+180)/6). (104+180)/6 = 47.33, floor=47. But zone 47 is correct for 102-108.
        assert_eq!(utm_zone_from_lon(104.06), 48);
        assert_eq!(utm_zone_from_lon(116.4), 50); // Beijing ~116°E
        assert_eq!(utm_zone_from_lon(-74.0), 18); // New York
    }

    #[test]
    fn test_utm_central_meridian() {
        assert_eq!(utm_central_meridian(48), 105.0);
    }

    #[test]
    fn test_latlon_to_utm_roundtrip() {
        // 成都 (104.06, 30.57) 中心
        let (e, n, zone, north) = latlon_to_utm(30.57, 104.06);
        assert!(e > 400000.0 && e < 500000.0, "easting={e}");
        assert!(n > 3300000.0 && n < 3400000.0, "northing={n}");
        assert_eq!(zone, 48);
        assert!(north);

        let (lat, lon) = utm_to_latlon(e, n, zone, north);
        assert!(
            (lat - 30.57).abs() < 0.001,
            "lat diff={}",
            (lat - 30.57).abs()
        );
        assert!(
            (lon - 104.06).abs() < 0.001,
            "lon diff={}",
            (lon - 104.06).abs()
        );
    }

    #[test]
    fn test_southern_hemisphere() {
        // Buenos Aires (-34.6, -58.38)
        let (e, n, zone, north) = latlon_to_utm(-34.6, -58.38);
        assert!(!north);
        assert!(n > 0.0); // 南半球加上 10000000

        let (lat, lon) = utm_to_latlon(e, n, zone, north);
        assert!((lat - (-34.6)).abs() < 0.01);
        assert!((lon - (-58.38)).abs() < 0.01);
    }

    #[test]
    fn test_zone_info() {
        let info = utm_zone_info(116.4);
        assert_eq!(info["zone"], 50);
    }

    #[test]
    fn test_is_ups() {
        assert!(is_ups(84.0));
        assert!(is_ups(-80.0));
        assert!(!is_ups(30.0));
    }
}
