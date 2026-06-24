use serde::{Deserialize, Serialize};

/// 结构产状 (倾向/倾角)。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StructureAttitude {
    /// 走向 (度，正北顺时针)
    pub strike_deg: f64,
    /// 倾角 (度)
    pub dip_deg: f64,
    /// 倾向方向 (度，正北顺时针)
    pub dip_direction_deg: f64,
}

/// 断层几何。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultGeometry {
    /// 断层名称
    pub name: String,
    /// 走向 (度)
    pub strike_deg: f64,
    /// 倾角 (度)
    pub dip_deg: f64,
    /// 倾滑量 (m)
    pub slip_m: f64,
    /// 断层长度 (km)
    pub length_km: f64,
    /// 断层类型
    pub fault_type: String,
    /// 断层迹线点 [(x, y)]
    pub trace: Vec<(f64, f64)>,
}

/// 褶皱几何。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoldGeometry {
    /// 褶皱名称
    pub name: String,
    /// 褶皱类型 (anticline/syncline)
    pub fold_type: String,
    /// 轴迹走向 (度)
    pub axial_trace_strike_deg: f64,
    /// 翼间角 (度)
    pub interlimb_angle_deg: f64,
    /// 波长 (km)
    pub wavelength_km: f64,
    /// 波幅 (m)
    pub amplitude_m: f64,
}

/// 计算倾斜岩层的真厚度。
pub fn true_thickness(apparent_thickness_m: f64, dip_deg: f64) -> f64 {
    apparent_thickness_m * dip_deg.to_radians().sin()
}

/// 由走向/倾角推算倾向。
pub fn dip_direction(strike_deg: f64, dip_deg: f64, dip_quadrant: &str) -> f64 {
    match dip_quadrant {
        "NW" => (strike_deg + 90.0).rem_euclid(360.0),
        "NE" => (strike_deg + 270.0).rem_euclid(360.0),
        "SE" => (strike_deg + 90.0).rem_euclid(360.0),
        "SW" => (strike_deg + 270.0).rem_euclid(360.0),
        _ => strike_deg,
    }
}

/// 断层平面几何参数计算 (简化)。
pub fn fault_plane_geometry(
    name: &str,
    strike_deg: f64,
    dip_deg: f64,
    slip_m: f64,
    length_km: f64,
    fault_type: &str,
) -> FaultGeometry {
    let dd = dip_direction(strike_deg, dip_deg, "SE");
    FaultGeometry {
        name: name.into(),
        strike_deg,
        dip_deg,
        slip_m,
        length_km,
        fault_type: fault_type.into(),
        trace: vec![],
    }
}

/// 褶皱几何参数。
pub fn fold_geometry(
    name: &str,
    fold_type: &str,
    axial_strike_deg: f64,
    interlimb_angle_deg: f64,
    wavelength_km: f64,
    amplitude_m: f64,
) -> FoldGeometry {
    FoldGeometry {
        name: name.into(),
        fold_type: fold_type.into(),
        axial_trace_strike_deg: axial_strike_deg,
        interlimb_angle_deg,
        wavelength_km,
        amplitude_m,
    }
}

/// 结构产状。
pub fn structure_attitude(strike_deg: f64, dip_deg: f64, dip_quadrant: &str) -> StructureAttitude {
    StructureAttitude {
        strike_deg,
        dip_deg,
        dip_direction_deg: dip_direction(strike_deg, dip_deg, dip_quadrant),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_true_thickness() {
        let t = true_thickness(100.0, 30.0);
        assert!((t - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_dip_direction() {
        let dd = dip_direction(0.0, 45.0, "SE");
        assert!((dd - 90.0).abs() < 0.01);
    }

    #[test]
    fn test_fault_geometry() {
        let f = fault_plane_geometry("San Andreas", 140.0, 75.0, 10.0, 1200.0, "strike-slip");
        assert_eq!(f.name, "San Andreas");
        assert_eq!(f.fault_type, "strike-slip");
        assert!((f.dip_deg - 75.0).abs() < 0.01);
    }

    #[test]
    fn test_fold_geometry() {
        let f = fold_geometry("Barmer", "anticline", 0.0, 120.0, 5.0, 200.0);
        assert_eq!(f.fold_type, "anticline");
        assert_eq!(f.interlimb_angle_deg, 120.0);
    }

    #[test]
    fn test_structure_attitude() {
        let sa = structure_attitude(45.0, 60.0, "SE");
        assert!((sa.strike_deg - 45.0).abs() < 0.01);
        assert!((sa.dip_deg - 60.0).abs() < 0.01);
    }
}
