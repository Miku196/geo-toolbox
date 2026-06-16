//! geo-adapter-cli: GDAL raster/vector operations.
#![allow(missing_docs)]
#[cfg(feature = "gdal-bindings")]
extern crate gdal as gdal_sys;
pub mod gcs_bridge;
pub mod raster;
pub mod tools;
pub mod vector;
pub use gcs_bridge::GcsBridge;
pub use raster::{
    CogOptions, DataType, GdalTranslateOptions, GdalWarpOptions, OutputDriver, RasterFormat,
    RasterInfo, RasterOps, ResamplingMethod,
};
pub use vector::VectorOps;
