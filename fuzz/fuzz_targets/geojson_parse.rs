//! Fuzz target: GeoJSON parser.
//!
//! Run: `cargo +nightly fuzz run geojson_parse`

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::str;

fuzz_target!(|data: &[u8]| {
    // GeoJSON must be valid UTF-8
    if let Ok(json_str) = str::from_utf8(data) {
        // Try to parse as GeoJSON — should never panic
        let _ = json_str.parse::<geojson::GeoJson>();
    }
});
