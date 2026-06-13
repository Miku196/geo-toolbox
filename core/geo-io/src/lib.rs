//! geo-io: Data ingestion parsers.
#![allow(missing_docs)]
pub mod camofox;
pub mod geojson;
pub mod nmea;
pub mod tools;
pub mod validator;
pub use geojson::{extract_bbox, parse_feature_collection, GeoJsonFeature};
