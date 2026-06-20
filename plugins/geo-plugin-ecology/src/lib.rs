//! geo-plugin-ecology: 生态修复评估插件。
//!
//! 核心功能：
//! - NDVI 变化检测（两期遥感影像对比）
//! - 碳汇计算（直接调用 geo-carbon-math）
//! - 植被恢复评估报告
//!
//! ## 矿山修复典型案例
//!
//! ```text
//! 输入：矿区 AOI GeoJSON + 2020 年 NDVI + 2025 年 NDVI
//!   → geo-io 读取 AOI 边界
//!   → geo-raster 计算 NDVI 差值
//!   → geo-stats 分区统计植被恢复面积
//!   → geo-carbon-math 计算碳汇
//!   → 组装生态修复评估报告
//! 输出：植被恢复面积 + 碳汇量 + 评估结论
//! ```
//!
//! ⚠ 本插件不 import geo-plugin-carbon，直接调用 geo-carbon-math。
//! ⚠ 碳密度参数自行配置（rules.toml），不依赖外部。

#![allow(missing_docs)]

pub mod config;
pub mod ecology;
pub mod lulc;
pub mod rusle;
pub mod sdr;
pub mod tools;

pub use config::EcologyConfig;
pub use ecology::{AssessmentInput, EcologyPlugin, RestorationAssessment};
pub use rusle::{
    assess_soil_loss, c_factor_for_landuse, compute_c_factor_from_ndvi, compute_k_factor,
    compute_k_factor_simple, compute_ls_factor, compute_ls_from_dem, compute_p_factor,
    compute_r_factor, compute_r_factor_simple, compute_slope_from_dem, compute_soil_loss,
    ErosionClass, PracticeType, RusleAssessment,
};
pub use sdr::{
    apply_sdr_to_rusle, compute_sdr, musle_event, musle_return_periods, SdrMethod, SdrResult,
};
