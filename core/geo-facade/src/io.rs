//! IO Facade — GeoJSON 解析与边界框提取。

pub use geo_core::errors::GeoResult;
pub use geo_core::types::BBox;
pub use geo_io::geojson::{parse_feature_collection, GeoJsonFeature};

/// 从 GeoJSON FeatureCollection 提取整体边界框。
///
/// facade 推荐入口：内部转发到 geo_io::geojson::extract_bbox（旧路径已 deprecated）。
#[allow(deprecated)]
pub fn extract_bbox(geojson: &str) -> GeoResult<BBox> {
    geo_io::geojson::extract_bbox(geojson)
}
