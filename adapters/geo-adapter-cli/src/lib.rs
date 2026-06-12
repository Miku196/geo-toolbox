//! geo-adapter-cli: GDAL raster/vector operations.
#![allow(missing_docs)]
#[cfg(feature = "gdal-bindings")]
extern crate gdal as gdal_sys;
pub mod gcs_bridge;
pub mod raster;
pub mod vector;
pub mod tools;
pub use gcs_bridge::GcsBridge;
pub use raster::RasterOps;
pub use vector::VectorOps;
