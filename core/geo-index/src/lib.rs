//! geo-index: 空间索引基类。
//!
//! 提供：
//! - GeoHash 编解码（Base32）
//! - 邻域 GeoHash 计算
//! - BBox → GeoHash 覆盖
//! - R-tree 索引（STR 批量构建）
//! - Quadtree 自适应四叉树

pub mod geohash;
pub mod h3;
pub mod quadtree;
pub mod rtree;
pub mod tools;

pub use geohash::{bbox_to_geohashes, decode, encode, neighbors, GeohashBounds};
pub use h3::{
    h3_cover_bbox, h3_edge_length_km, h3_from_string, h3_grid_disk, h3_hex_area_km2,
    h3_num_hexagons, h3_to_geojson, h3_to_string, latlon_to_h3, H3Index,
};
pub use quadtree::Quadtree;
pub use rtree::RTree;
