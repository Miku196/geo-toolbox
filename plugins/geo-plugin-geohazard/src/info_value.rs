//! 信息量模型 (Information Value Model)
//!
//! 基于双变量统计的滑坡敏感性制图方法。
//! IV_i = ln(D_i / D_total) / ln(A_i / A_total)
//!
//! 参考: Yin & Yan (1988), Van Westen (1997)

use serde::{Deserialize, Serialize};

/// 单因子单个类别的信息量值。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorClassInfo {
    /// 类别名称（如 "0-15°", "granite"）。
    pub class_name: String,
    /// 该类别的栅格数/面积。
    pub class_area: f64,
    /// 该类别内滑坡栅格数/面积。
    pub landslide_area: f64,
    /// 信息量值 IV = ln((N_i/N)/(A_i/A))
    pub information_value: f64,
    /// IV 符号: + 有利致灾, - 不利致灾。
    pub is_favorable: bool,
}

/// 单因子信息量计算结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorInfoValue {
    /// 因子名称（如 "坡度", "岩性"）。
    pub factor_name: String,
    /// 总研究区面积/栅格数。
    pub total_area: f64,
    /// 总滑坡面积/栅格数。
    pub total_landslide: f64,
    /// 各分类的信息量值。
    pub classes: Vec<FactorClassInfo>,
    /// 该因子总体 IV 范围。
    pub iv_range: (f64, f64),
}

/// 信息量模型综合结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoValueModel {
    /// 各因子信息量。
    pub factors: Vec<FactorInfoValue>,
    /// 综合信息量（各因子 IV 叠加）。
    /// 每个 cell 的 IV_total = Σ IV_i(class)
    pub total_iv: Option<f64>,
    /// 滑坡敏感性等级。
    pub susceptibility_level: InfoValueLevel,
}

/// 信息量法敏感性等级（基于 IV_total 阈值）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InfoValueLevel {
    /// IV ≤ -0.5，低敏感。
    Low,
    /// -0.5 < IV ≤ 0，较低敏感。
    ModerateLow,
    /// 0 < IV ≤ 0.5，中敏感。
    Moderate,
    /// 0.5 < IV ≤ 1.0，较高敏感。
    ModerateHigh,
    /// IV > 1.0，高敏感。
    High,
}

impl InfoValueLevel {
    pub fn from_iv(iv: f64) -> Self {
        if iv <= -0.5 {
            InfoValueLevel::Low
        } else if iv <= 0.0 {
            InfoValueLevel::ModerateLow
        } else if iv <= 0.5 {
            InfoValueLevel::Moderate
        } else if iv <= 1.0 {
            InfoValueLevel::ModerateHigh
        } else {
            InfoValueLevel::High
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            InfoValueLevel::Low => "低敏感",
            InfoValueLevel::ModerateLow => "较低敏感",
            InfoValueLevel::Moderate => "中敏感",
            InfoValueLevel::ModerateHigh => "较高敏感",
            InfoValueLevel::High => "高敏感",
        }
    }
}

/// 单个类别的输入数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInput {
    /// 类别名称。
    pub name: String,
    /// 该类别面积（栅格数或平方米）。
    pub area: f64,
    /// 该类别内滑坡面积。
    pub landslide: f64,
}

/// 计算单因子的信息量值。
///
/// # Arguments
/// * `factor_name` - 因子名称
/// * `classes` - 各类别 (面积, 滑坡面积)
/// * `total_area` - 研究区总面积
/// * `total_landslide` - 研究区总滑坡面积
pub fn compute_factor_iv(
    factor_name: &str,
    classes: &[ClassInput],
    total_area: f64,
    total_landslide: f64,
) -> FactorInfoValue {
    if total_area <= 0.0 || total_landslide <= 0.0 {
        return FactorInfoValue {
            factor_name: factor_name.to_string(),
            total_area,
            total_landslide,
            classes: vec![],
            iv_range: (0.0, 0.0),
        };
    }

    let global_ratio = total_landslide / total_area;

    let class_infos: Vec<FactorClassInfo> = classes
        .iter()
        .map(|c| {
            let class_ratio = if c.area > 0.0 {
                c.landslide / c.area
            } else {
                0.0
            };

            // IV = ln((N_i/A_i) / (N/A)) = ln(landslide_density_class / landslide_density_global)
            let iv = if class_ratio > 0.0 && global_ratio > 0.0 {
                (class_ratio / global_ratio).ln()
            } else {
                // 若该类无滑坡，给定一个小的负值
                -5.0 // ≈ 0 probability
            };

            FactorClassInfo {
                class_name: c.name.clone(),
                class_area: c.area,
                landslide_area: c.landslide,
                information_value: iv,
                is_favorable: iv > 0.0,
            }
        })
        .collect();

    let iv_min = class_infos
        .iter()
        .map(|c| c.information_value)
        .fold(f64::INFINITY, f64::min);
    let iv_max = class_infos
        .iter()
        .map(|c| c.information_value)
        .fold(f64::NEG_INFINITY, f64::max);

    FactorInfoValue {
        factor_name: factor_name.to_string(),
        total_area,
        total_landslide,
        classes: class_infos,
        iv_range: (iv_min, iv_max),
    }
}

/// 综合多个因子的信息量，按类别 IV 叠加。
///
/// # Arguments
/// * `factors` - 各因子信息量结果
/// * `cell_factors` - 每个 cell 所属各因子类别 IV 值列表
///
/// 返回单元格的综合信息量。
pub fn compute_total_iv(factors: &[FactorInfoValue], cell_class_ivs: &[f64]) -> f64 {
    // 简单叠加各因子 IV
    cell_class_ivs.iter().sum()
}

/// 计算综合信息量模型（便捷函数）。
///
/// # Arguments
/// * `factor_data` - 各因子数据: (因子名, [类别数据], 总面积, 滑坡总面积)
pub fn compute_iv_model(
    factor_data: &[(&str, &[ClassInput], f64, f64)],
    total_area: f64,
    total_landslide: f64,
) -> InfoValueModel {
    let factors: Vec<FactorInfoValue> = factor_data
        .iter()
        .map(|(name, classes, _, _)| {
            compute_factor_iv(name, classes, total_area, total_landslide)
        })
        .collect();

    // 采样 cell 综合 IV（用中间类别的 IV 算术平均近似）
    let sample_ivs: Vec<f64> = factors
        .iter()
        .map(|f| {
            f.classes
                .iter()
                .filter(|c| c.information_value > -5.0)
                .map(|c| c.information_value)
                .sum::<f64>()
                / f.classes.len().max(1) as f64
        })
        .collect();

    let total_iv = if sample_ivs.is_empty() {
        0.0
    } else {
        sample_ivs.iter().sum::<f64>() / sample_ivs.len() as f64
    };

    InfoValueModel {
        factors,
        total_iv: Some(total_iv),
        susceptibility_level: InfoValueLevel::from_iv(total_iv),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_factor_iv_slope() {
        let total_area = 100_000.0;
        let total_landslide = 1_000.0;

        let classes = vec![
            ClassInput {
                name: "0-15°".into(),
                area: 30_000.0,
                landslide: 100.0,
            },
            ClassInput {
                name: "15-25°".into(),
                area: 40_000.0,
                landslide: 400.0,
            },
            ClassInput {
                name: "25-35°".into(),
                area: 20_000.0,
                landslide: 300.0,
            },
            ClassInput {
                name: ">35°".into(),
                area: 10_000.0,
                landslide: 200.0,
            },
        ];

        let result = compute_factor_iv("坡度", &classes, total_area, total_landslide);

        assert_eq!(result.classes.len(), 4);

        // 0-15°: IV = ln((100/30000) / (1000/100000)) = ln(0.003333 / 0.01) = ln(0.3333) ≈ -1.099
        assert!((result.classes[0].information_value + 1.099).abs() < 0.01);
        assert!(!result.classes[0].is_favorable);

        // 15-25°: IV = ln((400/40000) / 0.01) = ln(0.01 / 0.01) = 0
        assert!((result.classes[1].information_value - 0.0).abs() < 0.01);
        assert!(!result.classes[1].is_favorable); // exactly 0: not favorable

        // >35°: IV = ln((200/10000) / 0.01) = ln(0.02 / 0.01) = ln(2) ≈ 0.693
        assert!((result.classes[3].information_value - 0.693).abs() < 0.01);
        assert!(result.classes[3].is_favorable);

        assert!(result.iv_range.0 < 0.0);
        assert!(result.iv_range.1 > 0.0);
    }

    #[test]
    fn test_compute_factor_iv_no_landslide() {
        let total_area = 100_000.0;
        let total_landslide = 0.0;

        let classes = vec![ClassInput {
            name: "flat".into(),
            area: 50_000.0,
            landslide: 0.0,
        }];

        let result = compute_factor_iv("empty", &classes, total_area, total_landslide);
        assert_eq!(result.classes.len(), 0);
    }

    #[test]
    fn test_iv_level_from_iv() {
        assert_eq!(InfoValueLevel::from_iv(-2.0), InfoValueLevel::Low);
        assert_eq!(InfoValueLevel::from_iv(-0.3), InfoValueLevel::ModerateLow);
        assert_eq!(InfoValueLevel::from_iv(0.2), InfoValueLevel::Moderate);
        assert_eq!(InfoValueLevel::from_iv(0.8), InfoValueLevel::ModerateHigh);
        assert_eq!(InfoValueLevel::from_iv(2.0), InfoValueLevel::High);
    }

    #[test]
    fn test_compute_iv_model_integration() {
        let total_area = 100_000.0;
        let total_landslide = 1_000.0;

        let slope_classes = vec![
            ClassInput {
                name: "0-15°".into(),
                area: 30_000.0,
                landslide: 100.0,
            },
            ClassInput {
                name: "15-35°".into(),
                area: 50_000.0,
                landslide: 600.0,
            },
            ClassInput {
                name: ">35°".into(),
                area: 20_000.0,
                landslide: 300.0,
            },
        ];

        let lithology_classes = vec![
            ClassInput {
                name: "硬岩".into(),
                area: 60_000.0,
                landslide: 300.0,
            },
            ClassInput {
                name: "软岩".into(),
                area: 40_000.0,
                landslide: 700.0,
            },
        ];

        let factor_data: Vec<(&str, &[ClassInput], f64, f64)> = vec![
            ("坡度", &slope_classes, total_area, total_landslide),
            ("岩性", &lithology_classes, total_area, total_landslide),
        ];

        let model = compute_iv_model(&factor_data, total_area, total_landslide);

        assert_eq!(model.factors.len(), 2);
        assert!(model.total_iv.is_some());
        assert_eq!(model.factors[0].factor_name, "坡度");
        assert_eq!(model.factors[1].factor_name, "岩性");
    }
}
