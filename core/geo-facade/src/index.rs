//! Index Facade — GeoHash 编码、解码、BBox 查询。

pub use geo_core::types::BBox;
pub use geo_index::geohash::{bbox_to_geohashes, decode, neighbors};

/// 编码经纬度为 GeoHash 字符串。
///
/// facade 推荐入口：内部转发到 geo_index::geohash::encode（旧路径已 deprecated）。
#[allow(deprecated)]
pub fn encode(lon: f64, lat: f64, precision: usize) -> String {
    geo_index::geohash::encode(lon, lat, precision)
}
