//! RUSLE — 修正通用土壤流失方程。
//!
//! `A = R × K × LS × C × P`
//!
//! | 因子 | 含义 | 单位 |
//! |------|------|------|
//! | R    | 降雨侵蚀力 | MJ·mm/ha·h·yr |
//! | K    | 土壤可蚀性 | t·ha·h/ha·MJ·mm |
//! | LS   | 坡长-坡度 | 无量纲 |
//! | C    | 覆盖管理 | 无量纲 (0-1) |
//! | P    | 水土保持措施 | 无量纲 (0-1) |
//! | A    | 年均土壤流失量 | t/ha/yr |

mod assess;
mod erosion_class;
mod factors;
mod musle;
mod practice;

#[cfg(test)]
mod tests;

pub use assess::{assess_soil_loss, RusleAssessment};
pub use erosion_class::ErosionClass;
pub use factors::{
    c_factor_for_landuse, compute_c_factor_from_ndvi, compute_k_factor, compute_k_factor_simple,
    compute_ls_factor, compute_ls_from_dem, compute_p_factor, compute_r_factor,
    compute_r_factor_simple, compute_slope_from_dem, compute_soil_loss,
};
pub use musle::compute_musle_sediment;
pub use practice::PracticeType;
