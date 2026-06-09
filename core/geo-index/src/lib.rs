//! geo-index: 空间索引基类。
//!
//! 提供：
//! - GeoHash 编解码（Base32）
//! - 邻域 GeoHash 计算
//! - BBox → GeoHash 覆盖

#![allow(missing_docs)]

pub mod geohash;

pub use geohash::{encode, decode, neighbors, bbox_to_geohashes, GeohashBounds};
