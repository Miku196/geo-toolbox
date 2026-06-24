use serde::{Deserialize, Serialize};

/// 灾害等级。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum HazardLevel {
    /// 低风险
    Low,
    /// 中风险
    Moderate,
    /// 高风险
    High,
    /// 极高风险
    VeryHigh,
    /// 极端危险
    Extreme,
}

impl HazardLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Moderate => "moderate",
            Self::High => "high",
            Self::VeryHigh => "very_high",
            Self::Extreme => "extreme",
        }
    }
    pub fn score(&self) -> u8 {
        match self {
            Self::Low => 1,
            Self::Moderate => 2,
            Self::High => 3,
            Self::VeryHigh => 4,
            Self::Extreme => 5,
        }
    }
}

/// 灾害区划结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HazardZoneResult {
    /// 栅格级别 (每单元)
    pub zone_grid: Vec<u8>,
    /// 级别列表
    pub levels: Vec<String>,
    /// 各等级面积 (km²)
    pub area_by_level_km2: [f64; 5],
    /// 总评估面积
    pub total_area_km2: f64,
    /// 最危险等级
    pub highest_level: String,
    /// 源位置
    pub source_row: usize,
    pub source_col: usize,
}

/// 基于多种因素的火山灾害分区。
/// - ash_thickness_mm: 火山灰厚度栅格
/// - lava_flow_path: 是否在熔岩流径 (0/1)
/// - distance_from_vent_km: 距喷发口距离栅格
/// - slope_degrees: 坡度栅格
pub fn volcanic_hazard_zoning(
    ash_thickness_mm: &[f64],
    lava_flow_path: &[u8],
    distance_from_vent_km: &[f64],
    _slope_degrees: &[f64],
    n: usize,
    source_row: usize,
    source_col: usize,
) -> HazardZoneResult {
    let mut zones = vec![0u8; n];
    let mut areas = [0.0_f64; 5];
    let cell_area_km2 = 0.0009; // ~30m cells

    for i in 0..n {
        let mut score = 0u8;

        // 火山灰贡献
        if ash_thickness_mm[i] > 100.0 {
            score += 3;
        } else if ash_thickness_mm[i] > 10.0 {
            score += 2;
        } else if ash_thickness_mm[i] > 1.0 {
            score += 1;
        }

        // 熔岩流
        if lava_flow_path[i] == 1 {
            score += 4;
        }

        // 距离
        let dist = distance_from_vent_km[i];
        if dist < 1.0 {
            score += 3;
        } else if dist < 5.0 {
            score += 2;
        } else if dist < 15.0 {
            score += 1;
        }

        // 转换为等级
        let level = match score {
            0..=1 => 1,
            2..=3 => 2,
            4..=5 => 3,
            6..=7 => 4,
            _ => 5,
        };
        zones[i] = level;
        if level >= 1 && level <= 5 {
            areas[(level - 1) as usize] += cell_area_km2;
        }
    }

    let total_area = zones.iter().map(|_| cell_area_km2).sum();
    let max_level = zones.iter().max().copied().unwrap_or(1);
    let highest = match max_level {
        1 => "low",
        2 => "moderate",
        3 => "high",
        4 => "very_high",
        _ => "extreme",
    };

    HazardZoneResult {
        zone_grid: zones,
        levels: vec![
            "low".into(),
            "moderate".into(),
            "high".into(),
            "very_high".into(),
            "extreme".into(),
        ],
        area_by_level_km2: areas,
        total_area_km2: total_area,
        highest_level: highest.into(),
        source_row,
        source_col,
    }
}

/// 单点灾害等级分类。
pub fn hazard_zone_classification(
    ash_thickness_mm: f64,
    on_lava_path: bool,
    distance_km: f64,
) -> HazardLevel {
    let mut score = 0u8;

    if ash_thickness_mm > 100.0 {
        score += 3;
    } else if ash_thickness_mm > 10.0 {
        score += 2;
    } else if ash_thickness_mm > 1.0 {
        score += 1;
    }

    if on_lava_path {
        score += 4;
    }

    if distance_km < 1.0 {
        score += 3;
    } else if distance_km < 5.0 {
        score += 2;
    } else if distance_km < 15.0 {
        score += 1;
    }

    match score {
        0..=1 => HazardLevel::Low,
        2..=3 => HazardLevel::Moderate,
        4..=5 => HazardLevel::High,
        6..=7 => HazardLevel::VeryHigh,
        _ => HazardLevel::Extreme,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_point_classification() {
        assert_eq!(
            hazard_zone_classification(200.0, false, 0.5),
            HazardLevel::VeryHigh
        ); // ash 3 + dist 3 = 6
        assert_eq!(
            hazard_zone_classification(0.0, true, 0.1),
            HazardLevel::VeryHigh
        ); // lava 4 + dist 3 = 7
        assert_eq!(
            hazard_zone_classification(0.5, false, 20.0),
            HazardLevel::Low
        );
    }

    #[test]
    fn test_grid_zoning() {
        let n = 100;
        let ash = vec![50.0; n];
        let lava = vec![0u8; n];
        let dist = vec![10.0; n];
        let slope = vec![5.0; n];
        let r = volcanic_hazard_zoning(&ash, &lava, &dist, &slope, n, 5, 5);
        assert_eq!(r.zone_grid.len(), n);
        assert!(!r.highest_level.is_empty());
    }

    #[test]
    fn test_proximal_zoning() {
        let n = 100;
        let ash = vec![200.0; n];
        let mut lava = vec![0u8; n];
        lava[50] = 1;
        let mut dist = vec![0.5; n];
        dist[50] = 0.1;
        let slope = vec![5.0; n];
        let r = volcanic_hazard_zoning(&ash, &lava, &dist, &slope, n, 5, 5);
        // 远处综合分数高
        assert!(r.zone_grid[50] >= 3);
    }

    #[test]
    fn test_hazard_level_order() {
        assert!(HazardLevel::Low.score() < HazardLevel::Extreme.score());
        assert!(HazardLevel::Moderate.score() < HazardLevel::VeryHigh.score());
    }

    #[test]
    fn test_hazard_level_str() {
        assert_eq!(HazardLevel::High.as_str(), "high");
        assert_eq!(HazardLevel::Extreme.as_str(), "extreme");
    }
}
