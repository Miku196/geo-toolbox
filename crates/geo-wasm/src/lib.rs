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
mod ingest;
mod output;
mod spatial;
mod storage;
mod tile;
mod utils;

use wasm_bindgen::prelude::*;

pub use carbon::CarbonEngine;
pub use crs::CrsEngine;
pub use storage::GeoStore;
pub use tile::TileEngine;

/// Call this once after loading the WASM module.
/// Sets up panic hook for readable browser console errors.
#[wasm_bindgen(js_name = initPanicHook)]
pub fn init_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
