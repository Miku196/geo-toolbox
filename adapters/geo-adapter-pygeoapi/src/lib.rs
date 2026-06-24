//! geo-adapter-pygeoapi — PyO3 FFI adapter for zero-copy geometry interchange.
//!
//! Converts between Rust geo-types and Python shapely geometries, numpy arrays
//! for raster data, and xarray Dataset/DataArray for n-dimensional data.

pub mod adapter;
pub mod geometry;
pub mod raster;

pub use adapter::PyGeoAdapter;
