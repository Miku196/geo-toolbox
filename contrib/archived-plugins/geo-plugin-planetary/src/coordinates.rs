use serde::{Deserialize, Serialize};

/// 行星参考框架。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PlanetaryFrame {
    /// 地球 (ITRF)
    Earth,
    /// 月球 — 平均地球极 (Mean Earth / Polar Axis)
    LunarMeanEarthPole,
    /// 火星 — 火星 2000 参考系
    Mars2000,
    /// 通用 — J2000 历元地心赤道
    J2000,
}

impl PlanetaryFrame {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Earth => "Earth_ITRF",
            Self::LunarMeanEarthPole => "Lunar_MEP",
            Self::Mars2000 => "Mars2000",
            Self::J2000 => "J2000",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Earth_ITRF" | "earth" => Some(Self::Earth),
            "Lunar_MEP" | "lunar" => Some(Self::LunarMeanEarthPole),
            "Mars2000" | "mars" => Some(Self::Mars2000),
            "J2000" => Some(Self::J2000),
            _ => None,
        }
    }
}

/// 行星坐标 (经度/纬度/高度)。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PlanetaryCoordinate {
    /// 经度 (度)
    pub lon_deg: f64,
    /// 纬度 (度)
    pub lat_deg: f64,
    /// 高度 (km)
    pub altitude_km: f64,
    /// 参考框架
    pub frame: PlanetaryFrame,
}

/// 简单天体坐标系变换 (旋转近似)。
///  Lunar → Earth: 月面经/纬度 → 对应月球轨道附近的经/纬度
///  实际需要 JPL DE 星历；此处返回近似平移 + 旋转。
pub fn lunar_coordinate_transform(
    lon_deg: f64,
    lat_deg: f64,
    _altitude_km: f64,
    from: PlanetaryFrame,
    to: PlanetaryFrame,
) -> Option<PlanetaryCoordinate> {
    if from == to {
        return Some(PlanetaryCoordinate {
            lon_deg,
            lat_deg,
            altitude_km: _altitude_km,
            frame: to,
        });
    }
    // 简化的框架间平移
    let (dlon, dlat) = match (from, to) {
        (PlanetaryFrame::LunarMeanEarthPole, PlanetaryFrame::Earth) => (1.5, -0.6),
        (PlanetaryFrame::Earth, PlanetaryFrame::LunarMeanEarthPole) => (-1.5, 0.6),
        (PlanetaryFrame::Mars2000, PlanetaryFrame::Earth) => (0.8, 1.2),
        (PlanetaryFrame::Earth, PlanetaryFrame::Mars2000) => (-0.8, -1.2),
        (PlanetaryFrame::J2000, PlanetaryFrame::Earth) => (0.0, 23.44), // 黄赤交角
        _ => return None,                                               // 不支持的直接转换
    };
    Some(PlanetaryCoordinate {
        lon_deg: (lon_deg + dlon).rem_euclid(360.0),
        lat_deg: (lat_deg + dlat).clamp(-90.0, 90.0),
        altitude_km: _altitude_km,
        frame: to,
    })
}

/// 火星坐标变换 (Mars2000 ↔ 地球 ITRF 近似)。
pub fn mars_coordinate_transform(
    lon_deg: f64,
    lat_deg: f64,
    altitude_km: f64,
    to_earth: bool,
) -> PlanetaryCoordinate {
    let (from, to) = if to_earth {
        (PlanetaryFrame::Mars2000, PlanetaryFrame::Earth)
    } else {
        (PlanetaryFrame::Earth, PlanetaryFrame::Mars2000)
    };
    lunar_coordinate_transform(lon_deg, lat_deg, altitude_km, from, to).unwrap_or(
        PlanetaryCoordinate {
            lon_deg,
            lat_deg,
            altitude_km,
            frame: to,
        },
    )
}

/// Celestial → geographic (将 J2000 赤道坐标转至地球 ITRF 近似)。
pub fn celestial_to_geographic(
    ra_deg: f64,
    dec_deg: f64,
    _distance_au: f64,
) -> PlanetaryCoordinate {
    // 极简：赤纬→纬度，赤经→经度 (忽略岁差/章动)
    PlanetaryCoordinate {
        lon_deg: ra_deg.rem_euclid(360.0),
        lat_deg: dec_deg.clamp(-90.0, 90.0),
        altitude_km: _distance_au * 1.496e8, // AU→km
        frame: PlanetaryFrame::Earth,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lunar_to_earth() {
        let r = lunar_coordinate_transform(
            45.0,
            0.0,
            0.0,
            PlanetaryFrame::LunarMeanEarthPole,
            PlanetaryFrame::Earth,
        )
        .unwrap();
        assert!((r.lon_deg - 46.5).abs() < 0.01);
        assert!((r.lat_deg - (-0.6)).abs() < 0.01);
        assert_eq!(r.frame, PlanetaryFrame::Earth);
    }

    #[test]
    fn test_mars_to_earth() {
        let r = mars_coordinate_transform(30.0, 10.0, 0.0, true);
        assert!((r.lon_deg - 30.8).abs() < 0.01);
        assert!((r.lat_deg - 11.2).abs() < 0.01);
    }

    #[test]
    fn test_celestial_to_geographic() {
        // 北极星近似
        let r = celestial_to_geographic(0.0, 89.26, 0.0043);
        assert!((r.lat_deg - 89.26).abs() < 0.01);
        assert!(r.altitude_km > 0.0);
    }

    #[test]
    fn test_identity_transform() {
        let r = lunar_coordinate_transform(
            10.0,
            20.0,
            0.0,
            PlanetaryFrame::Earth,
            PlanetaryFrame::Earth,
        )
        .unwrap();
        assert!((r.lon_deg - 10.0).abs() < 1e-6);
        assert!((r.lat_deg - 20.0).abs() < 1e-6);
    }

    #[test]
    fn test_frame_str_roundtrip() {
        for f in &[
            PlanetaryFrame::Earth,
            PlanetaryFrame::LunarMeanEarthPole,
            PlanetaryFrame::Mars2000,
            PlanetaryFrame::J2000,
        ] {
            assert_eq!(PlanetaryFrame::from_str(f.as_str()), Some(*f));
        }
    }

    #[test]
    fn test_unsupported_transform() {
        let r = lunar_coordinate_transform(
            0.0,
            0.0,
            0.0,
            PlanetaryFrame::J2000,
            PlanetaryFrame::Mars2000,
        );
        assert!(r.is_none());
    }
}
