//! geo-io: Data ingestion parsers.
#![allow(missing_docs)]
pub mod camofox;
pub mod geojson;
pub mod nmea;
pub mod tools;
pub mod validator;
// extract_bbox 已 deprecated（迁移至 geo_facade::io::extract_bbox）；
// 根路径 re-export 保留以兼容旧调用方，故 allow(deprecated)。
#[allow(deprecated)]
pub use geojson::{extract_bbox, parse_feature_collection, GeoJsonFeature};
