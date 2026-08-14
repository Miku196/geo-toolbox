use serde::{Deserialize, Serialize};

/// 水土保持措施类型（用于 P 因子计算）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PracticeType {
    /// 无措施
    None,
    /// 等高耕作
    Contouring,
    /// 等高带状种植
    StripCropping,
    /// 梯田
    Terracing,
}
