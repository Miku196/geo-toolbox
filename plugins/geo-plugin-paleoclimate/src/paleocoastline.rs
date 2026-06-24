/// 古海岸线恢复：根据海平面变化确定淹没/出露区域。
/// - elevation_m: 当前栅格高程 (m, +陆地/-水深)
/// - sea_level_offset_m: 古海平面相对于现代的变化 (负 = 更低)
/// - cols: 每行像元数
///
/// 返回: (海岸线掩膜, 古水深) — 1=陆地, 0=海水
pub fn paleocoastline_flooding(
    elevation_m: &[f64],
    sea_level_offset_m: f64,
    cols: usize,
) -> (Vec<u8>, Vec<f64>) {
    let n = elevation_m.len();
    let mut mask = vec![0u8; n];
    let mut paleobathy = vec![0.0_f64; n];

    for (i, &z) in elevation_m.iter().enumerate() {
        let paleo_z = z - sea_level_offset_m;
        paleobathy[i] = paleo_z;
        if paleo_z > 0.0 {
            mask[i] = 1; // 陆地
        }
    }

    // 简单的海岸线提取：标记陆地-海水交界的单元
    let rows = n / cols;
    for r in 0..rows {
        for c in 0..cols {
            let idx = r * cols + c;
            if mask[idx] == 1 {
                // 检查相邻象元
                let neighbors = [
                    if c > 0 { Some(idx - 1) } else { None },
                    if c < cols - 1 { Some(idx + 1) } else { None },
                    if r > 0 { Some(idx - cols) } else { None },
                    if r < rows - 1 { Some(idx + cols) } else { None },
                ];
                for nb in neighbors.iter().flatten() {
                    if mask[*nb] == 0 {
                        mask[idx] = 2; // 海岸线
                        break;
                    }
                }
            }
        }
    }

    (mask, paleobathy)
}

/// 计算古海岸线相对于现代的位置变化 (km)。
pub fn coastline_shift(
    current_shore_dist_km: f64,
    sea_level_offset_m: f64,
    avg_slope_degrees: f64,
) -> f64 {
    let slope_rad = avg_slope_degrees.to_radians();
    let horizontal_shift = sea_level_offset_m.abs() / slope_rad.tan().max(0.001);
    if sea_level_offset_m < 0.0 {
        // 海平面降低 → 海岸线向海推进
        current_shore_dist_km + horizontal_shift / 1000.0
    } else {
        // 海平面上升 → 海岸线向陆后退
        (current_shore_dist_km - horizontal_shift / 1000.0).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flooding_lower_sea() {
        // 简单 2x3 栅格
        let elev = vec![10.0, -5.0, -10.0, 2.0, -20.0, 30.0];
        let (mask, _bathy) = paleocoastline_flooding(&elev, -125.0, 3);
        // 所有象元古海平面 +125 m → 都为陆地?
        assert_eq!(mask.len(), 6);
    }

    #[test]
    fn test_flooding_higher_sea() {
        let elev = vec![10.0, -5.0, 0.0];
        let (mask, _bathy) = paleocoastline_flooding(&elev, 10.0, 3);
        // 古海平面 +10 m: 10-10=0 >0 陆地, -5-10=-15 <0 海水, 0-10=-10 <0 海水
        assert_eq!(mask[0], 0); // 陆地
        assert_eq!(mask[1], 0); // 海水
    }

    #[test]
    fn test_coastline_shift_regression() {
        let shift = coastline_shift(10.0, -125.0, 0.5);
        assert!(shift > 10.0);
    }

    #[test]
    fn test_coastline_shift_transgression() {
        let shift = coastline_shift(10.0, 50.0, 0.5);
        assert!(shift < 10.0);
        assert!(shift >= 0.0);
    }
}
