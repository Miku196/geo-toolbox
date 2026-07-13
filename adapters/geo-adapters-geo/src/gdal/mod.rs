//! GDAL raster/vector operations (CLI adapter).
#![allow(missing_docs)]
#[cfg(feature = "gdal-bindings")]
extern crate gdal as gdal_sys;

pub mod gdal_adapter;
pub mod gdal_gcs_bridge;
pub mod gdal_raster;
pub mod gdal_tools;
pub mod gdal_vector;

pub use gdal_adapter::CliAdapter;
pub use gdal_gcs_bridge::{GcsBridge, GcsBridgeConfig};
pub use gdal_raster::{
    CogOptions, DataType, GdalTranslateOptions, GdalWarpOptions, OutputDriver, RasterFormat,
    RasterInfo, RasterOps, ResamplingMethod,
};
pub use gdal_vector::{Ogr2OgrOptions, VectorOps};
pub use gdal_tools::register_tools;
