//! geo-stats: 空间统计基类。
//!
//! 提供：
//! - 分区统计（zonal statistics）：在多边形区域内统计栅格值

#![warn(missing_docs)]

pub mod classify;
pub mod hotspot;
pub mod idw;
pub mod moran;
pub mod tools;
pub mod zonal;

pub use classify::{equal_interval_breaks, jenks, quantile_breaks, JenksResult};
pub use hotspot::{gistar, queen_weights_self, GiStar};
pub use idw::{idw_grid, idw_point, IdwResult};
pub use moran::{morans_i, queen_weights, rook_weights, MoranI};
pub use zonal::{zonal_stats, ZonalResult, ZonalStats};
