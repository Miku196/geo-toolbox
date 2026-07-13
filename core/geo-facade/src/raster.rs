//! Raster Facade — 栅格运算（NDVI, NDWI, 波段运算）。

pub use geo_raster::band::{band_add, band_div, band_mul, band_sub, compute_ndwi};
pub use geo_raster::ndvi::compute_ndvi;
pub use geo_core::errors::GeoResult;
