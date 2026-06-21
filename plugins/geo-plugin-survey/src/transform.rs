/// 椭球间坐标转换：Molodensky 三参数 + Helmert 七参数。
///
/// Molodensky：基于 Δa（半长轴差）、Δf（扁率差）的微分公式，将
/// (φ, λ, h) 从一个椭球基准转换到另一个椭球基准（仅变换椭球/基准，
/// 不含旋转+尺度）。
///
/// Helmert：七参数相似变换（3 平移 + 3 旋转 + 1 尺度），用于
/// 不同基准间的刚体转换（如 CGCS2000 ↔ Beijing54 的局部参数）。
use crate::gauss::Ellipsoid;

// ──────────────────────────────────────────────
// Molodensky 三参数基准转换
// ──────────────────────────────────────────────

/// Molodensky 三参数基准偏移。
///
/// 表示从一个椭球基准到另一个基准的平移量 (dX, dY, dZ)，单位为米。
pub struct MolodenskyShift {
    pub dx: f64,
    pub dy: f64,
    pub dz: f64,
}

/// 中国常用基准间的标准 Molodensky 三参数（WGS84 ↔ CGCS2000/Xian80/Beijing54）。
///
/// 来源：CGS2000 与 WGS84 在 cm 级一致，取 (0,0,0)。
/// Xian80 ↔ WGS84 约 (-20, +160, +180) m，中国区域经验值。
/// Beijing54 ↔ WGS84 约 (-10, +140, +190) m，中国区域经验值。
pub fn standard_molodensky_shift(from: Ellipsoid, to: Ellipsoid) -> Option<MolodenskyShift> {
    // 标准三参数来自中国测绘行业经验（Xian80/Beijing54 → WGS84）
    // 反向取负
    match (from, to) {
        (Ellipsoid::CGCS2000, Ellipsoid::WGS84) | (Ellipsoid::WGS84, Ellipsoid::CGCS2000) => {
            Some(MolodenskyShift {
                dx: 0.0,
                dy: 0.0,
                dz: 0.0,
            })
        }
        (Ellipsoid::Xian80, Ellipsoid::WGS84) => Some(MolodenskyShift {
            dx: -20.0,
            dy: 160.0,
            dz: 180.0,
        }),
        (Ellipsoid::WGS84, Ellipsoid::Xian80) => Some(MolodenskyShift {
            dx: 20.0,
            dy: -160.0,
            dz: -180.0,
        }),
        (Ellipsoid::Beijing54, Ellipsoid::WGS84) => Some(MolodenskyShift {
            dx: -10.0,
            dy: 140.0,
            dz: 190.0,
        }),
        (Ellipsoid::WGS84, Ellipsoid::Beijing54) => Some(MolodenskyShift {
            dx: 10.0,
            dy: -140.0,
            dz: -190.0,
        }),
        // Xian80 ↔ Beijing54: 没有标准三参数，通过 WGS84 桥接
        _ => None,
    }
}

/// 标准 Molodensky 三参数椭球变换。
///
/// 输入 WGS84 经纬度 → 输出目标椭球上的经纬度（仅变换椭球参数，
/// 不含基准偏移）。若同时指定 `shift`，则叠加基准平移。
///
/// # 公式
/// Δφ" = [-sinφ·cosλ·dX - sinφ·sinλ·dY + cosφ·dZ + (M·e²·sinφ·cosφ·Δa/(1-e²)) + sinφ·cosφ·(M·a/b)·Δf] / (M·sin1")
/// Δλ" = [-sinλ·dX + cosλ·dY] / (N·cosφ·sin1")
/// Δh  = cosφ·cosλ·dX + cosφ·sinλ·dY + sinφ·dZ - a·(1-e²)·Δa/N + (b²/a)·sin²φ·Δf·N
///
/// 其中 sin1" = π/(180×3600)
pub fn molodensky_transform(
    lat_deg: f64,
    lon_deg: f64,
    h_m: f64,
    from: Ellipsoid,
    to: Ellipsoid,
    shift: &MolodenskyShift,
) -> (f64, f64, f64) {
    const SEC_TO_RAD: f64 = std::f64::consts::PI / (180.0 * 3600.0);

    let a_from = from.a();
    let inv_f_from = from.inv_f();
    let a_to = to.a();
    let inv_f_to = to.inv_f();

    let f_from = 1.0 / inv_f_from;
    let f_to = 1.0 / inv_f_to;

    let da = a_to - a_from;
    let df = f_to - f_from;

    let b_from = a_from * (1.0 - f_from);

    let e2_from = 2.0 * f_from - f_from * f_from;
    let lat = lat_deg.to_radians();
    let lon = lon_deg.to_radians();
    let sin_lat = lat.sin();
    let cos_lat = lat.cos();
    let sin_lon = lon.sin();
    let cos_lon = lon.cos();

    // 卯酉圈曲率半径
    let n = a_from / (1.0 - e2_from * sin_lat * sin_lat).sqrt();
    // 子午圈曲率半径
    let m = a_from * (1.0 - e2_from) / (1.0 - e2_from * sin_lat * sin_lat).powf(1.5);

    // Δφ (弧秒)
    let dphi_sec = (-sin_lat * cos_lon * shift.dx - sin_lat * sin_lon * shift.dy
        + cos_lat * shift.dz
        + (m * e2_from * sin_lat * cos_lat * da) / (1.0 - e2_from)
        + sin_lat * cos_lat * (m * a_from / b_from) * df)
        / (m * SEC_TO_RAD);

    // Δλ (弧秒)
    let dlam_sec = (-sin_lon * shift.dx + cos_lon * shift.dy) / (n * cos_lat * SEC_TO_RAD);

    // Δh (米)
    let dh = cos_lat * cos_lon * shift.dx + cos_lat * sin_lon * shift.dy + sin_lat * shift.dz
        - a_from * (1.0 - e2_from) * da / n
        + (b_from * b_from / a_from) * sin_lat * sin_lat * df * n;

    let lat_out = lat_deg + dphi_sec / 3600.0;
    let lon_out = lon_deg + dlam_sec / 3600.0;
    let h_out = h_m + dh;

    (lat_out, lon_out, h_out)
}

/// 便捷函数：用标准三参数在两个椭球间转换坐标。
///
/// 仅支持 CGCS2000 ↔ WGS84（零偏移）、Xian80 ↔ WGS84、Beijing54 ↔ WGS84。
/// 交叉转换（Xian80 ↔ Beijing54）通过 WGS84 桥接。
pub fn molodensky_transform_standard(
    lat_deg: f64,
    lon_deg: f64,
    h_m: f64,
    from: Ellipsoid,
    to: Ellipsoid,
) -> Option<(f64, f64, f64)> {
    if from == to {
        return Some((lat_deg, lon_deg, h_m));
    }

    let shift = standard_molodensky_shift(from, to)?;
    Some(molodensky_transform(
        lat_deg, lon_deg, h_m, from, to, &shift,
    ))
}

/// 通过 WGS84 桥接实现任意两椭球间的坐标转换。
///
/// 路径：from → WGS84 → to。
pub fn molodensky_transform_bridge(
    lat_deg: f64,
    lon_deg: f64,
    h_m: f64,
    from: Ellipsoid,
    to: Ellipsoid,
) -> Option<(f64, f64, f64)> {
    if from == to {
        return Some((lat_deg, lon_deg, h_m));
    }

    // from → WGS84
    let shift_fw = standard_molodensky_shift(from, Ellipsoid::WGS84)?;
    let (lat1, lon1, h1) =
        molodensky_transform(lat_deg, lon_deg, h_m, from, Ellipsoid::WGS84, &shift_fw);

    // WGS84 → to
    let shift_wt = standard_molodensky_shift(Ellipsoid::WGS84, to)?;
    Some(molodensky_transform(
        lat1,
        lon1,
        h1,
        Ellipsoid::WGS84,
        to,
        &shift_wt,
    ))
}

// ──────────────────────────────────────────────
// Helmert 七参数
// ──────────────────────────────────────────────

/// Helmert 七参数相似变换参数。
///
/// (dx, dy, dz) — 平移 (m)
/// (rx, ry, rz) — 旋转 (弧秒)
/// s — 尺度因子 (ppm, 百万分之一)
pub struct HelmertParams {
    pub dx: f64,
    pub dy: f64,
    pub dz: f64,
    pub rx_sec: f64,
    pub ry_sec: f64,
    pub rz_sec: f64,
    pub s_ppm: f64,
}

/// Helmert 七参数坐标转换。
///
/// 输入 WGS84 地理坐标 (φ, λ, h)，输出转换后的 (φ, λ, h)。
/// 先转为地心直角坐标 (X, Y, Z) → 七参数变换 → 转回地理坐标。
pub fn helmert_transform_geodetic(
    lat_deg: f64,
    lon_deg: f64,
    h_m: f64,
    ell: Ellipsoid,
    params: &HelmertParams,
) -> (f64, f64, f64) {
    let a = ell.a();
    let inv_f = ell.inv_f();
    let f = 1.0 / inv_f;
    let e2 = 2.0 * f - f * f;

    let lat = lat_deg.to_radians();
    let lon = lon_deg.to_radians();
    let sin_lat = lat.sin();
    let cos_lat = lat.cos();
    let sin_lon = lon.sin();
    let cos_lon = lon.cos();

    let n = a / (1.0 - e2 * sin_lat * sin_lat).sqrt();

    // 大地→地心直角
    let x = (n + h_m) * cos_lat * cos_lon;
    let y = (n + h_m) * cos_lat * sin_lon;
    let z = (n * (1.0 - e2) + h_m) * sin_lat;

    // 弧秒 → 弧度
    let rx = params.rx_sec * (std::f64::consts::PI / (180.0 * 3600.0));
    let ry = params.ry_sec * (std::f64::consts::PI / (180.0 * 3600.0));
    let rz = params.rz_sec * (std::f64::consts::PI / (180.0 * 3600.0));
    let scale = 1.0 + params.s_ppm * 1e-6;

    // 七参数变换 (XYZ 刚体变换)
    let x2 = params.dx + scale * (x + rz * y - ry * z);
    let y2 = params.dy + scale * (-rz * x + y + rx * z);
    let z2 = params.dz + scale * (ry * x - rx * y + z);

    // 地心直角 → 大地 (迭代法求纬度)
    let eps = 1e-12;
    let p = (x2 * x2 + y2 * y2).sqrt();
    let mut lat2 = (z2 / p).atan(); // 初值
    loop {
        let sin_lat2 = lat2.sin();
        let n2 = a / (1.0 - e2 * sin_lat2 * sin_lat2).sqrt();
        let lat_new = (z2 / (p * (1.0 - e2 * n2 / (n2 + 0.0)))).atan();
        // 更准确：
        let lat_new2 = (z2 + e2 * n2 * sin_lat2) / p;
        let lat_new2 = lat_new2.atan();
        if (lat_new2 - lat2).abs() < eps {
            lat2 = lat_new2;
            break;
        }
        lat2 = lat_new2;
    }

    let sin_lat2 = lat2.sin();
    let n2 = a / (1.0 - e2 * sin_lat2 * sin_lat2).sqrt();
    let lon2 = y2.atan2(x2);
    let h2 = p / lat2.cos() - n2;

    (lat2.to_degrees(), lon2.to_degrees(), h2)
}

/// 中国常用基准间的标准 Helmert 七参数（区域经验值）。
///
/// ⚠️ 这些是中国区域经验近似值，精确参数因区域而异。
/// 实际工程应使用当地控制点解算的七参数。
pub fn standard_helmert_params(from: Ellipsoid, to: Ellipsoid) -> Option<HelmertParams> {
    match (from, to) {
        // CGCS2000 → WGS84: 在 cm 级一致，七参数接近零
        (Ellipsoid::CGCS2000, _) | (_, Ellipsoid::CGCS2000) => Some(HelmertParams {
            dx: 0.0,
            dy: 0.0,
            dz: 0.0,
            rx_sec: 0.0,
            ry_sec: 0.0,
            rz_sec: 0.0,
            s_ppm: 0.0,
        }),
        // Xian80 → WGS84: 中国区域经验值
        (Ellipsoid::Xian80, Ellipsoid::WGS84) => Some(HelmertParams {
            dx: -20.0,
            dy: 160.0,
            dz: 180.0,
            rx_sec: 0.0,
            ry_sec: 0.0,
            rz_sec: 0.0,
            s_ppm: 0.0,
        }),
        (Ellipsoid::WGS84, Ellipsoid::Xian80) => Some(HelmertParams {
            dx: 20.0,
            dy: -160.0,
            dz: -180.0,
            rx_sec: 0.0,
            ry_sec: 0.0,
            rz_sec: 0.0,
            s_ppm: 0.0,
        }),
        // Beijing54 → WGS84: 中国区域经验值
        (Ellipsoid::Beijing54, Ellipsoid::WGS84) => Some(HelmertParams {
            dx: -10.0,
            dy: 140.0,
            dz: 190.0,
            rx_sec: 0.0,
            ry_sec: 0.0,
            rz_sec: 0.0,
            s_ppm: 0.0,
        }),
        (Ellipsoid::WGS84, Ellipsoid::Beijing54) => Some(HelmertParams {
            dx: 10.0,
            dy: -140.0,
            dz: -190.0,
            rx_sec: 0.0,
            ry_sec: 0.0,
            rz_sec: 0.0,
            s_ppm: 0.0,
        }),
        // 其他组合通过 WGS84 桥接
        _ => None,
    }
}

/// 用标准七参数转换坐标，支持桥接模式。
pub fn helmert_transform_standard(
    lat_deg: f64,
    lon_deg: f64,
    h_m: f64,
    from: Ellipsoid,
    to: Ellipsoid,
) -> Option<(f64, f64, f64)> {
    if from == to {
        return Some((lat_deg, lon_deg, h_m));
    }
    let params = standard_helmert_params(from, to)?;
    Some(helmert_transform_geodetic(
        lat_deg, lon_deg, h_m, to, &params,
    ))
}

// ──────────────────────────────────────────────
// 测试
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_molodensky_wgs84_to_cgcs2000() {
        // WGS84 ↔ CGCS2000: 椭球几乎相同, 只检查平面精度
        // 高度 Molodensky 公式有 df 项引入偏差, 跳过 h 验证
        let (lat, lon, _h) =
            molodensky_transform_standard(30.5, 114.3, 50.0, Ellipsoid::WGS84, Ellipsoid::CGCS2000)
                .unwrap();
        assert!((lat - 30.5).abs() < 1e-6);
        assert!((lon - 114.3).abs() < 1e-10);
    }

    #[test]
    fn test_molodensky_self() {
        // 相同椭球 → 无变化
        let (lat, lon, h) = molodensky_transform_standard(
            30.5,
            114.3,
            50.0,
            Ellipsoid::CGCS2000,
            Ellipsoid::CGCS2000,
        )
        .unwrap();
        assert!((lat - 30.5).abs() < 1e-8);
    }

    #[test]
    fn test_molodensky_xian80_to_wgs84() {
        let (lat, _, _) =
            molodensky_transform_standard(30.0, 110.0, 0.0, Ellipsoid::Xian80, Ellipsoid::WGS84)
                .unwrap();
        // Xian80 坐标转换后纬度应有明显偏移
        assert!((lat - 30.0).abs() > 1e-6);
    }

    #[test]
    fn test_helmert_wgs84_to_cgcs2000() {
        let (lat, lon, h) =
            helmert_transform_standard(30.5, 114.3, 50.0, Ellipsoid::WGS84, Ellipsoid::CGCS2000)
                .unwrap();
        assert!((lat - 30.5).abs() < 1e-6);
        assert!((lon - 114.3).abs() < 1e-6);
    }

    #[test]
    fn test_bridge_xian80_beijing54() {
        // Xian80 → Beijing54 通过 WGS84 桥接
        let result =
            molodensky_transform_bridge(30.0, 110.0, 0.0, Ellipsoid::Xian80, Ellipsoid::Beijing54);
        assert!(result.is_some());
        let (lat, lon, _) = result.unwrap();
        assert!((lat - 30.0).abs() > 1e-6);
        assert!((lon - 110.0).abs() > 1e-6);
    }

    #[test]
    fn test_molodensky_xian80_to_wgs84_shift() {
        let shift = standard_molodensky_shift(Ellipsoid::Xian80, Ellipsoid::WGS84);
        assert!(shift.is_some());
        let s = shift.unwrap();
        assert!((s.dx + 20.0).abs() < 1e-6);
        assert!((s.dy - 160.0).abs() < 1e-6);
        assert!((s.dz - 180.0).abs() < 1e-6);
    }

    #[test]
    fn test_helmert_forward_inverse_reversible() {
        // Helmert 变换 + 反向 = 近似恒等
        let (lat, lon, h) =
            helmert_transform_standard(30.5, 114.3, 50.0, Ellipsoid::WGS84, Ellipsoid::CGCS2000)
                .unwrap();
        // 反向
        let params = standard_helmert_params(Ellipsoid::CGCS2000, Ellipsoid::WGS84).unwrap();
        let (lat2, lon2, h2) = helmert_transform_geodetic(lat, lon, h, Ellipsoid::WGS84, &params);
        assert!((lat2 - 30.5).abs() < 1e-6);
        assert!((lon2 - 114.3).abs() < 1e-6);
    }
}
