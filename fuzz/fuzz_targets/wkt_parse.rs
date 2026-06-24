/// Fuzz target: WKT (Well-Known Text) parsing.
///
/// Exercise geo_core::types::from_wkt with arbitrary input.
/// The goal is to ensure no panic on malformed or truncated WKT strings.
#![no_main]

use libfuzzer_sys::fuzz_target;
use geo_fuzz::geo_core;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else { return; };
    let _ = geo_core::types::from_wkt(s);
});
