use serde::{Deserialize, Serialize};

/// 土壤侵蚀严重等级（t/ha/yr）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErosionClass {
    /// 微度 < 5
    Slight,
    /// 轻度 5-10
    Moderate,
    /// 中度 10-20
    High,
    /// 强烈 20-50
    Severe,
    /// 极强烈 > 50
    VerySevere,
}

impl ErosionClass {
    pub fn from_rate(t_per_ha_yr: f64) -> Self {
        if t_per_ha_yr < 5.0 {
            Self::Slight
        } else if t_per_ha_yr < 10.0 {
            Self::Moderate
        } else if t_per_ha_yr < 20.0 {
            Self::High
        } else if t_per_ha_yr < 50.0 {
            Self::Severe
        } else {
            Self::VerySevere
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Slight => "微度",
            Self::Moderate => "轻度",
            Self::High => "中度",
            Self::Severe => "强烈",
            Self::VerySevere => "极强烈",
        }
    }
}
