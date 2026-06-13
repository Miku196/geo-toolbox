//! geo-index: 空间索引基类。
//!
//! 提供：
//! - GeoHash 编解码（Base32）
//! - 邻域 GeoHash 计算
//! - BBox → GeoHash 覆盖

#![allow(missing_docs)]

pub mod geohash;
pub mod tools;

pub use geohash::{bbox_to_geohashes, decode, encode, neighbors, GeohashBounds};
