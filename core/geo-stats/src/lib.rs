//! geo-stats: 空间统计基类。
//!
//! 提供：
//! - 分区统计（zonal statistics）：在多边形区域内统计栅格值

#![warn(missing_docs)]

pub mod zonal;
pub mod tools;

pub use zonal::{ZonalStats, ZonalResult, zonal_stats};
