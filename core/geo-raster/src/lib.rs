//! geo-raster: 纯 Rust 栅格运算基类。
//!
//! 提供：
//! - 栅格数据结构（内存中的二维数组）
//! - 波段运算（add/sub/mul/div/ndvi/ndwi）
//! - NDVI 差值计算
//! - 基本统计（min/max/mean/std）
//!
//! 不依赖 GDAL、不依赖外部 C 库、不依赖数据库。

#![warn(missing_docs)]

pub mod band;
pub mod grid;
/// 镶嵌算法：合并和羽化融合。
pub mod mosaic;
pub mod ndvi;
/// 重采样算法：最邻近和双三次。
pub mod resample;
/// 地形分析算子：坡度（度/百分比）、坡向（Horn 1981 算法）。
pub mod terrain;
/// GeoTIFF 写入器：将 `RasterBand` 写出为 Float32 TIFF + .tfw 世界文件。
pub mod tiff_writer;

pub use band::{band_add, band_div, band_mul, band_sub, band_threshold, compute_ndwi};
pub use grid::RasterBand;
pub use mosaic::{mosaic_feather, mosaic_merge};
pub use ndvi::{compute_ndvi, ndvi_difference, NdviResult};
pub use resample::{resample_cubic, resample_nearest};
pub use terrain::{
    compute_aspect, compute_hillshade, compute_slope_degrees, compute_slope_percent, compute_tpi,
    compute_tri, resample_bilinear, zonal_stats, AspectResult, SlopeResult, ZonalStats, ZoneStats,
};
