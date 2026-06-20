//! geo-tile: 矢量瓦片 (MVT) 编码 + 栅格瓦片 (PMTiles) 读写。
//!
//! 纯 Rust 实现，零 C 依赖。为浏览器端 MapLibre 渲染提供数据管道。
//!
//! ## 功能
//!
//! - **MVT 编码** — GeoJSON FeatureCollection → Mapbox Vector Tile (protobuf)
//! - **PMTiles 读写** — 单一文件栅格瓦片归档格式 v3
//! - **瓦片索引** — 经纬度 ↔ z/x/y 互转，Geohash 辅助索引
//!
//! ## 示例
//!
//! ```rust,ignore
//! use geo_tile::{latlon_to_tile, tile_to_latlon, MvtEncoder};
//!
//! let tile = latlon_to_tile(104.06, 30.57, 12);
//! assert_eq!(tile, (3270, 1671, 12));
//!
//! let mut encoder = MvtEncoder::new(4096);
//! encoder.add_layer("sites", &features)?;
//! let mvt_bytes = encoder.encode()?;  // 可直接喂给 MapLibre
//! ```

#![warn(missing_docs)]

mod mvt;
mod pmtiles;
mod tile_index;
pub mod tools;

pub use mvt::{GeomType, MvtEncoder, MvtFeature, MvtLayer, MvtValue};
pub use pmtiles::{Compression, PmtilesReader, PmtilesWriter, TileType};
pub use tile_index::{latlon_to_tile, tile_bounds, tile_to_latlon, tile_url, TileSource};
