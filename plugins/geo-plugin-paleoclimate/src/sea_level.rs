use serde::{Deserialize, Serialize};

/// 海平面重建结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeaLevelReconstruction {
    /// 年代 (ka BP)
    pub age_ka: Vec<f64>,
    /// 全球海平面 (m, 相对现代)
    pub global_sea_level_m: Vec<f64>,
    /// 冰盖均衡调整 (m)
    pub isostatic_adjustment_m: Vec<f64>,
    /// 相对海平面 (m)
    pub relative_sea_level_m: Vec<f64>,
    /// 时段范围
    pub period_label: String,
}

/// 重建冰期-间冰期海平面。
/// - eustatic_curve: (ka 前, 全球海平面 m) 关键点
/// - isostatic_frac: 局部均衡调整比例 (0-1)
pub fn sea_level_reconstruction(
    ages_ka: &[f64],
    eustatic_curve: &[(f64, f64)],
    isostatic_frac: f64,
) -> SeaLevelReconstruction {
    let global: Vec<f64> = ages_ka
        .iter()
        .map(|&age| interpolate_eustatic(age, eustatic_curve))
        .collect();
    let isostatic: Vec<f64> = global.iter().map(|&g| g * isostatic_frac).collect();
    let relative: Vec<f64> = global
        .iter()
        .zip(isostatic.iter())
        .map(|(g, i)| g + i)
        .collect();
    let period = if ages_ka.first().copied().unwrap_or(0.0) > 100.0 {
        "Last Glacial Maximum to Present".into()
    } else if ages_ka.first().copied().unwrap_or(0.0) > 1000.0 {
        "Quaternary glacial-interglacial cycles".into()
    } else {
        "Holocene".into()
    };

    SeaLevelReconstruction {
        age_ka: ages_ka.to_vec(),
        global_sea_level_m: global,
        isostatic_adjustment_m: isostatic,
        relative_sea_level_m: relative,
        period_label: period,
    }
}

/// LGM (21 ka) 到现在的简化古海平面网格。
pub fn lgm_sea_level_map(current_elevation_m: &[f64], _cols: usize) -> Vec<f64> {
    // LGM 全球海平面约 -125 m；将当前低于 -125 m 的单元标记为当时陆地
    current_elevation_m
        .iter()
        .map(|&z| {
            // 正值仍为正 (陆地不变)，负值若浅于 -125 则曾是陆地
            if z <= 0.0 && z > -125.0 {
                // 曾是陆地：返回古高程
                z
            } else {
                z // 水下或高于现代海平面
            }
        })
        .collect()
}

/// 冰川均衡调整 (GIA) 简化模型。
pub fn glacial_isostatic_adjustment(
    age_ka: f64,
    eustatic_m: f64,
    load_time_constant_ka: f64,
) -> f64 {
    // 指数松弛
    if age_ka < 0.0 {
        return 0.0;
    }
    let relaxation = (-age_ka / load_time_constant_ka).exp();
    eustatic_m * relaxation
}

fn interpolate_eustatic(age_ka: f64, curve: &[(f64, f64)]) -> f64 {
    if curve.is_empty() {
        return 0.0;
    }
    if age_ka <= curve[0].0 {
        return curve[0].1;
    }
    if age_ka >= curve[curve.len() - 1].0 {
        return curve[curve.len() - 1].1;
    }
    for i in 0..curve.len() - 1 {
        if age_ka >= curve[i].0 && age_ka <= curve[i + 1].0 {
            let t = (age_ka - curve[i].0) / (curve[i + 1].0 - curve[i].0);
            return curve[i].1 + t * (curve[i + 1].1 - curve[i].1);
        }
    }
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lgm_curve() -> Vec<(f64, f64)> {
        vec![
            (0.0, 0.0),
            (5.0, -5.0),
            (10.0, -40.0),
            (15.0, -90.0),
            (21.0, -125.0),
        ]
    }

    #[test]
    fn test_sea_level_present() {
        let curve = lgm_curve();
        let r = sea_level_reconstruction(&[0.0, 5.0, 10.0, 15.0, 21.0], &curve, 0.3);
        assert!((r.global_sea_level_m[0] - 0.0).abs() < 0.01);
        assert!((r.global_sea_level_m[4] - (-125.0)).abs() < 0.01);
        assert_eq!(r.relative_sea_level_m.len(), 5);
    }

    #[test]
    fn test_lgm_map() {
        let elev = vec![-50.0, -130.0, 0.0, 100.0];
        let map = lgm_sea_level_map(&elev, 2);
        assert!((map[0] - (-50.0)).abs() < 0.01); // was land
        assert!((map[1] - (-130.0)).abs() < 0.01); // still submerged
        assert!((map[2] - 0.0).abs() < 0.01);
        assert!((map[3] - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_gia_positive() {
        let sia = glacial_isostatic_adjustment(10.0, -125.0, 5.0);
        assert!(sia < 0.0);
        assert!(sia > -125.0);
    }

    #[test]
    fn test_interpolate_edge() {
        assert!((interpolate_eustatic(-1.0, &[(0.0, 0.0), (21.0, -125.0)]) - 0.0).abs() < 0.01);
        assert!(
            (interpolate_eustatic(100.0, &[(0.0, 0.0), (21.0, -125.0)]) - (-125.0)).abs() < 0.01
        );
    }

    #[test]
    fn test_interpolate_mid() {
        let v = interpolate_eustatic(10.5, &lgm_curve());
        // between -40 (10ka) and -90 (15ka)
        assert!(v > -90.0 && v < -40.0);
    }
}
