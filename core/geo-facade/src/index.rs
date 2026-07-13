//! Index Facade — GeoHash 编码、解码、BBox 查询。

pub use geo_index::geohash::{bbox_to_geohashes, decode, encode};
pub use geo_core::types::BBox;
