#![warn(missing_docs)]

//! geo-gdal: GDAL raster and vector operations.
//!
//! Wraps GDAL operations via **subprocess calls** to standard GDAL CLI tools
//! and optionally direct Rust bindings when the `gdal-bindings` feature is enabled.
//!
//! ## Modules
//!
//! - `raster` — COG conversion, raster algebra, band extraction, reprojection
//! - `vector` — ogr2ogr-equivalent format conversion (GeoJSON↔GPKG↔CSV)
//! - `gcs_bridge` — GCS → MinIO bridge (downloads from GCS, optionally converts to COG, uploads to MinIO)
//!
//! ## Feature flags
//!
//! - `gdal-bindings`: use Rust `gdal` crate for direct raster operations (faster, but requires libgdal)
//! - Without this feature, all operations go through `gdal_translate` / `ogr2ogr` subprocess

#[cfg(feature = "gdal-bindings")]
extern crate gdal as gdal_sys;

pub mod gcs_bridge;
pub mod raster;
pub mod vector;

pub use gcs_bridge::GcsBridge;
pub use raster::RasterOps;
pub use vector::VectorOps;
