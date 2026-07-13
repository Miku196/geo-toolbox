//! geo-facade — 统一门面层
//!
//! 将 geo-io / geo-index / geo-raster 的高频函数集中重导出。
//! Plugin 和 Adapter 层只需依赖 `geo-facade` 即可访问全部 Core 工具函数。

pub mod index;
pub mod io;
pub mod raster;
