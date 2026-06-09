//! geo-raster: 纯 Rust 栅格运算基类。
//!
//! 提供：
//! - 栅格数据结构（内存中的二维数组）
//! - 波段运算（add/sub/mul/div/ndvi/ndwi）
//! - NDVI 差值计算
//! - 基本统计（min/max/mean/std）
//!
//! 不依赖 GDAL、不依赖外部 C 库、不依赖数据库。

#![warn(missing_docs)]

pub mod band;
pub mod grid;
pub mod ndvi;

pub use grid::RasterBand;
pub use ndvi::{compute_ndvi, ndvi_difference, NdviResult};
