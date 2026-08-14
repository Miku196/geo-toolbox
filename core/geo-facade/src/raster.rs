//! Raster Facade — 栅格运算（NDVI, NDWI, 波段运算）。

use geo_raster::ndvi::NdviResult;
use geo_raster::RasterBand;

pub use geo_core::errors::GeoResult;
pub use geo_raster::band::{band_add, band_div, band_mul, band_sub, compute_ndwi};

/// 从红波段（RED）和近红外波段（NIR）计算 NDVI。
///
/// facade 推荐入口：内部转发到 geo_raster::ndvi::compute_ndvi（旧路径已 deprecated）。
#[allow(deprecated)]
pub fn compute_ndvi(red: &RasterBand, nir: &RasterBand) -> GeoResult<NdviResult> {
    geo_raster::ndvi::compute_ndvi(red, nir)
}
