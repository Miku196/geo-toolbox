//! IO Facade — GeoJSON 解析与边界框提取。

pub use geo_io::geojson::{extract_bbox, parse_feature_collection, GeoJsonFeature};
pub use geo_core::types::BBox;
pub use geo_core::errors::GeoResult;
