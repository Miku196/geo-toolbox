//! geo-stats: 空间统计基类。
//!
//! 提供：
//! - 分区统计（zonal statistics）：在多边形区域内统计栅格值

#![warn(missing_docs)]

pub mod tools;
pub mod zonal;

pub use zonal::{zonal_stats, ZonalResult, ZonalStats};
