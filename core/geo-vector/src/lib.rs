//! geo-vector: 纯 Rust 矢量运算基类。
//!
//! 提供：
//! - 缓冲区（buffer）
//! - 相交（intersection）
//! - 合并（union）
//! - 裁剪（clip）
//! - 拓扑验证
//! - 矢量统计（长度、面积、质心）
//!
//! 不依赖 GDAL/QGIS，纯 geo crate 实现。

#![allow(missing_docs)]

pub mod ops;
pub mod stats;

pub use ops::{buffer, intersect, union_all};
pub use stats::{centroid, feature_area};
