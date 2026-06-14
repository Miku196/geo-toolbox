//! geo-index: 空间索引基类。
//!
//! 提供：
//! - GeoHash 编解码（Base32）
//! - 邻域 GeoHash 计算
//! - BBox → GeoHash 覆盖
//! - R-tree 索引（STR 批量构建）
//! - Quadtree 自适应四叉树

pub mod geohash;
pub mod quadtree;
pub mod rtree;
pub mod tools;

pub use geohash::{bbox_to_geohashes, decode, encode, neighbors, GeohashBounds};
pub use quadtree::Quadtree;
pub use rtree::RTree;
