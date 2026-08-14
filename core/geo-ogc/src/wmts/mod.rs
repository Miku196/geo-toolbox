//! WMTS (Web Map Tile Service) — OGC WMTS 1.0.0 implementation.
//!
//! Supports:
//! - `GetCapabilities` — service metadata + tile matrix sets + layer listing
//! - `GetTile` — return a single tile (z/x/y) as image bytes
//! - `GetFeatureInfo` — query feature attributes at a pixel within a tile

mod cache;
mod capabilities;
mod pmtiles;
mod request;
mod response;
mod service;
mod tile_matrix;

pub mod renderers;

#[cfg(test)]
mod tests;

pub use cache::TileCache;
pub use request::{WmtsGetFeatureInfoParams, WmtsGetTileParams, WmtsRequest};
pub use response::WmtsResponse;
pub use service::WmtsService;
pub use tile_matrix::{
    global_geodetic_tile_matrix_set, global_mercator_tile_matrix_set, TileMatrix, TileMatrixSet,
    TileRendererFn, WmtsLayer,
};
