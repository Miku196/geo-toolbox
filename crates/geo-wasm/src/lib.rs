//! geo-wasm: Browser-side geospatial processing via WebAssembly.
//!
//! Brings geo-toolbox capabilities directly into the browser:
//! - CRS coordinate transforms (WGS84 ↔ Web Mercator, Equal Area)
//! - NMEA GPS parsing & validation
//! - Carbon emission calculation (IPCC Tier 1, pure Rust)
//! - GeoJSON / XLSX / Markdown export
//! - IndexedDB-backed spatial storage
//!
//! ## Design: Zero-panic, always return Result
//!
//! All public functions return `Result<T, JsValue>` — no `unwrap()` in
//! code paths reachable from JS. Panics = WASM traps = bad UX.

mod carbon;
mod crs;
mod debug;
mod geohash;
mod ingest;
mod output;
mod raster;
mod spatial;
mod storage;
mod tile;
mod utils;
mod vector;

use wasm_bindgen::prelude::*;

pub use carbon::CarbonEngine;
pub use crs::{CrsEngine, EPSG_BD09, EPSG_GCJ02};
pub use debug::*;
pub use geohash::*;
pub use ingest::*;
pub use output::*;
pub use raster::*;
pub use spatial::*;
pub use storage::GeoStore;
pub use tile::{latlon_to_tile, tile_url_gaode, tile_url_osm, TileCoord, TileEngine};
pub use utils::*;
pub use vector::*;

/// Call this once after loading the WASM module.
/// Sets up panic hook for readable browser console errors.
#[wasm_bindgen(js_name = initPanicHook)]
pub fn init_panic_hook() {
    debug::internal_log("info", "init", "Setting panic hook...");
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
    debug::internal_log("info", "init", "Panic hook set");
}

/// Auto-run when WASM module is loaded (wasm-bindgen start).
/// Logs version, features, and debug status.
#[wasm_bindgen(start)]
pub fn auto_init() {
    debug::internal_log(
        "info",
        "init",
        &format!(
            "geo-wasm v{} loaded. Debug: {}. Features: console_error_panic_hook={}",
            env!("CARGO_PKG_VERSION"),
            debug::is_debug_enabled(),
            cfg!(feature = "console_error_panic_hook"),
        ),
    );
}
