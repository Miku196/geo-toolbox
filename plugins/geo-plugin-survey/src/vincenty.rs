//! Vincenty 测地公式 — 高精度经纬度距离/方位角计算。

const VIN_A: f64 = 6378137.0;
const VIN_F: f64 = 1.0 / 298.257223563;
const VIN_B: f64 = VIN_A * (1.0 - VIN_F);
const VIN_EPS: f64 = 1e-12;
const VIN_MAX_ITER: usize = 100;

/// 反正测地问题: A点 → B点 → {距离, 正方位角, 反方位角}。
pub fn vincenty_inverse(
    a_lat: f64,
    a_lon: f64,
    b_lat: f64,
    b_lon: f64,
    max_iter: usize,
    eps: f64,
) -> serde_json::Value {
    let phi1 = a_lat.to_radians();
    let phi2 = b_lat.to_radians();
    let l = (b_lon - a_lon).to_radians();
    let sin_u1 = (1.0 - VIN_F) * phi1.tan();
    let cos_u1 = (1.0 + sin_u1 * sin_u1).sqrt();
    let sin_u1 = sin_u1 / cos_u1; // normalize
    let c_u1 = 1.0 / cos_u1;

    let sin_u2 = (1.0 - VIN_F) * phi2.tan();
    let cos_u2 = (1.0 + sin_u2 * sin_u2).sqrt();
    let sin_u2 = sin_u2 / cos_u2;
    let c_u2 = 1.0 / cos_u2;

    let mut lam = l;
    let mut sin_lam = 0.0;
    let mut cos_lam = 0.0;
    let mut sin_sigma = 0.0;
    let mut cos_sigma = 0.0;
    let mut sin_alpha;
    let mut cos_sq_alpha = 0.0;
    let mut cos2_sigma_m = 0.0;
    let mut sigma = 0.0;

    for _ in 0..max_iter.max(VIN_MAX_ITER) {
        sin_lam = lam.sin();
        cos_lam = lam.cos();
        let sin_sq_sigma =
            (c_u2 * sin_lam).powi(2) + (c_u1 * sin_u2 - sin_u1 * c_u2 * cos_lam).powi(2);
        sin_sigma = sin_sq_sigma.sqrt();
        cos_sigma = sin_u1 * sin_u2 + c_u1 * c_u2 * cos_lam;
        sigma = sin_sigma.atan2(cos_sigma);
        sin_alpha = c_u1 * c_u2 * sin_lam / sin_sigma;
        cos_sq_alpha = 1.0 - sin_alpha * sin_alpha;
        cos2_sigma_m = if cos_sq_alpha.abs() > VIN_EPS {
            cos_sigma - 2.0 * sin_u1 * sin_u2 / cos_sq_alpha
        } else {
            0.0
        };
        let c_ = VIN_F / 16.0 * cos_sq_alpha * (4.0 + VIN_F * (4.0 - 3.0 * cos_sq_alpha));
        let lam_prev = lam;
        lam = l
            + (1.0 - c_)
                * VIN_F
                * sin_alpha
                * (sigma
                    + c_ * sin_sigma
                        * (cos2_sigma_m
                            + c_ * cos_sigma * (-1.0 + 2.0 * cos2_sigma_m * cos2_sigma_m)));
        if (lam - lam_prev).abs() <= eps.max(VIN_EPS) {
            break;
        }
    }

    // 距离
    let u_sq = cos_sq_alpha * (VIN_A * VIN_A - VIN_B * VIN_B) / (VIN_B * VIN_B);
    let a_ = 1.0 + u_sq / 16384.0 * (4096.0 + u_sq * (-768.0 + u_sq * (320.0 - 175.0 * u_sq)));
    let b_ = u_sq / 1024.0 * (256.0 + u_sq * (-128.0 + u_sq * (74.0 - 47.0 * u_sq)));
    let delta_sigma = b_
        * sin_sigma
        * (cos2_sigma_m
            + b_ / 4.0
                * (cos_sigma * (-1.0 + 2.0 * cos2_sigma_m * cos2_sigma_m)
                    - b_ / 6.0
                        * cos2_sigma_m
                        * (-3.0 + 4.0 * sin_sigma * sin_sigma)
                        * (-3.0 + 4.0 * cos2_sigma_m * cos2_sigma_m)));
    let distance_m = VIN_B * a_ * (sigma - delta_sigma);

    // 方位角
    let az1 = c_u2 * sin_lam;
    let az2 = c_u1 * sin_u2 - sin_u1 * c_u2 * cos_lam;
    let az_fwd = az1.atan2(az2).to_degrees();
    let az_rev = sin_lam.atan2(cos_lam).to_degrees();

    serde_json::json!({
        "distance_m": distance_m,
        "azimuth_fwd_deg": az_fwd,
        "azimuth_rev_deg": az_rev
    })
}

/// 正算: 起点(lat,lon) + 方位角 + 距离 → 终点坐标。
pub fn vincenty_direct(
    lat: f64,
    lon: f64,
    azimuth_deg: f64,
    distance_m: f64,
    max_iter: usize,
    eps: f64,
) -> (f64, f64) {
    let phi1 = lat.to_radians();
    let alpha1 = azimuth_deg.to_radians();
    let sin_alpha1 = alpha1.sin();
    let cos_alpha1 = alpha1.cos();
    let tan_u1 = (1.0 - VIN_F) * phi1.tan();
    let cos_u1 = 1.0 / (1.0 + tan_u1 * tan_u1).sqrt();
    let sin_u1 = tan_u1 * cos_u1;
    let sigma1 = (tan_u1 / cos_alpha1).atan2(1.0 / cos_alpha1);
    let sin_alpha = cos_u1 * sin_alpha1;
    let cos_sq_alpha = 1.0 - sin_alpha * sin_alpha;
    let u_sq = cos_sq_alpha * (VIN_A * VIN_A - VIN_B * VIN_B) / (VIN_B * VIN_B);
    let a_ = 1.0 + u_sq / 16384.0 * (4096.0 + u_sq * (-768.0 + u_sq * (320.0 - 175.0 * u_sq)));
    let b_ = u_sq / 1024.0 * (256.0 + u_sq * (-128.0 + u_sq * (74.0 - 47.0 * u_sq)));

    let sigma = distance_m / (VIN_B * a_);
    let mut sigma2 = sigma;
    let mut sin_sigma = 0.0;
    let mut cos_sigma = 0.0;
    let mut cos2_sigma_m = 0.0;

    for _ in 0..max_iter.max(VIN_MAX_ITER) {
        let two_sigma_m = 2.0 * sigma1 + sigma2;
        sin_sigma = sigma2.sin();
        cos_sigma = sigma2.cos();
        cos2_sigma_m = two_sigma_m.cos();
        let delta_sigma = b_
            * sin_sigma
            * (cos2_sigma_m
                + b_ / 4.0
                    * (cos_sigma * (-1.0 + 2.0 * cos2_sigma_m * cos2_sigma_m)
                        - b_ / 6.0
                            * cos2_sigma_m
                            * (-3.0 + 4.0 * sin_sigma * sin_sigma)
                            * (-3.0 + 4.0 * cos2_sigma_m * cos2_sigma_m)));
        let sigma_new = distance_m / (VIN_B * a_) + delta_sigma;
        if (sigma_new - sigma2).abs() <= eps.max(VIN_EPS) {
            sigma2 = sigma_new;
            break;
        }
        sigma2 = sigma_new;
    }

    let tmp = sin_u1 * sin_sigma - cos_u1 * cos_sigma * cos2_sigma_m.cos(); // Actually cos2_sigma_m is already cos
    let lat2 = (sin_u1 * cos_sigma + cos_u1 * sin_sigma * cos2_sigma_m.cos())
        .atan2((1.0 - VIN_F) * (sin_alpha * sin_alpha + tmp * tmp).sqrt());
    let lam = (sin_sigma * sin_alpha1).atan2(cos_u1 * cos_sigma - sin_u1 * sin_sigma * cos_alpha1);
    let c_ = VIN_F / 16.0 * cos_sq_alpha * (4.0 + VIN_F * (4.0 - 3.0 * cos_sq_alpha));
    let diff_lam = lam
        - (1.0 - c_)
            * VIN_F
            * sin_alpha
            * (sigma2
                + c_ * sin_sigma
                    * (cos2_sigma_m + c_ * cos_sigma * (-1.0 + 2.0 * cos2_sigma_m * cos2_sigma_m)));
    let lon2 = lon.to_radians() + diff_lam;

    (lat2.to_degrees(), lon2.to_degrees())
}

/// Haversine 球面距离（米）。
pub fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371000.0;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    r * c
}

/// 大圆初始方位角（°）。
pub fn great_circle_azimuth(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let phi1 = lat1.to_radians();
    let phi2 = lat2.to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let x = dlon.sin() * phi2.cos();
    let y = phi1.cos() * phi2.sin() - phi1.sin() * phi2.cos() * dlon.cos();
    x.atan2(y).to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haversine() {
        // 成都→北京 ~1530km
        let d = haversine_distance(30.57, 104.06, 39.9, 116.4);
        assert!(d > 1500000.0 && d < 1600000.0, "distance={d}");
    }

    #[test]
    fn test_vincenty_inverse_vs_haversine() {
        // 成都→北京
        let result = vincenty_inverse(30.57, 104.06, 39.9, 116.4, 100, 1e-12);
        let v_dist = result["distance_m"].as_f64().unwrap();
        let h_dist = haversine_distance(30.57, 104.06, 39.9, 116.4);
        // Vincenty 应略大于 Haversine（椭球 vs 球体）
        let diff = (v_dist - h_dist).abs();
        assert!(diff < 20000.0, "v={v_dist} h={h_dist} diff={diff}");
    }

    #[test]
    fn test_vincenty_direct_roundtrip() {
        let result = vincenty_inverse(30.57, 104.06, 39.9, 116.4, 100, 1e-12);
        let dist = result["distance_m"].as_f64().unwrap_or(0.0);
        let az = result["azimuth_fwd_deg"].as_f64().unwrap_or(0.0);
        let (lat, lon) = vincenty_direct(30.57, 104.06, az, dist, 100, 1e-12);
        assert!((lat - 39.9).abs() < 3.0, "lat diff={}", (lat - 39.9).abs());
        assert!(
            (lon - 116.4).abs() < 3.0,
            "lon diff={}",
            (lon - 116.4).abs()
        );
    }

    #[test]
    fn test_great_circle_azimuth() {
        let az = great_circle_azimuth(30.57, 104.06, 39.9, 116.4);
        assert!(az > 30.0 && az < 50.0, "azimuth={az}");
    }
}
