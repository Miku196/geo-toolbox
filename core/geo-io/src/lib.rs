//! geo-io: Data ingestion parsers.
#![allow(missing_docs)]
pub mod camofox;
pub mod geojson;
pub mod nmea;
pub mod validator;
pub use geojson::{GeoJsonFeature, parse_feature_collection, extract_bbox};
