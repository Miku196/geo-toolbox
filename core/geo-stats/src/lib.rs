//! geo-stats: 空间统计基类。
//!
//! 提供：
//! - 分区统计（zonal statistics）：在多边形区域内统计栅格值

#![warn(missing_docs)]

pub mod classify;
pub mod hotspot;
pub mod idw;
pub mod kmeans;
pub mod moran;
/// MCP 工具注册 — 导出 5 个空间统计工具到运行时注册表。
pub mod regression;
/// MCP tool registration.
pub mod tools;
pub mod zonal;

pub use classify::{equal_interval_breaks, jenks, quantile_breaks, JenksResult};
pub use hotspot::{gistar, queen_weights_self, GiStar};
pub use idw::{idw_grid, idw_point, IdwResult};
pub use kmeans::{kmeans, kmeans_2d, KMeansResult};
pub use moran::{morans_i, queen_weights, rook_weights, MoranI};
pub use regression::{ols_regression, predict, predict_batch, residuals, OlsResult};
pub use zonal::{zonal_stats, ZonalResult, ZonalStats};

/// Standard normal CDF (Abramowitz & Stegun 7.1.26).
pub(crate) fn normal_cdf(x: f64) -> f64 {
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs() / (2.0f64).sqrt();
    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - ((((a5 * t + a4) * t) + a3) * t + a2) * t + a1 * t * (-x * x).exp();
    0.5 * (1.0 + sign * y)
}
