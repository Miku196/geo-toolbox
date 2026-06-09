//! Utility functions exposed to JavaScript.

use wasm_bindgen::prelude::*;

/// Get the version of the geo-wasm library.
#[wasm_bindgen(js_name = getVersion)]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Get build info (Rust version, target, features).
#[wasm_bindgen(js_name = getBuildInfo)]
pub fn get_build_info() -> String {
    serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "rustc": env!("CARGO_PKG_RUST_VERSION"),
        "features": {
            "console_error_panic_hook": cfg!(feature = "console_error_panic_hook"),
        }
    }).to_string()
}

/// Log a message to the browser console.
/// NOTE: exported as "consoleLog" to avoid WASM name collision with
/// the math library's `log` function (used by f64::ln()).
#[wasm_bindgen(js_name = consoleLog)]
pub fn log_to_console(msg: &str) {
    web_sys::console::log_1(&JsValue::from_str(msg));
}

/// Get memory usage stats (placeholder — useful for monitoring WASM heap).
#[wasm_bindgen(js_name = getMemoryStats)]
pub fn get_memory_stats() -> String {
    serde_json::json!({
        "note": "WASM heap stats not available in this build. Use browser DevTools Memory tab."
    }).to_string()
}
