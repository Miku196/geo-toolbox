//! Fuzz target: NMEA 0183 GPS parser.
//!
//! Run: `cargo +nightly fuzz run nmea_parse`

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::str;

/// Minimal NMEA sentence validator — checks for legal prefixes, checksum format, etc.
fn validate_nmea(sentence: &str) -> Result<(), &'static str> {
    let sentence = sentence.trim();

    if sentence.is_empty() {
        return Ok(());
    }

    // Must start with $ or !
    if !sentence.starts_with('$') && !sentence.starts_with('!') {
        return Err("no start delimiter");
    }

    // Check for optional checksum: *XX at end
    if let Some(star_pos) = sentence.rfind('*') {
        let checksum = &sentence[star_pos + 1..];
        if checksum.len() != 2 {
            return Err("bad checksum length");
        }
        if !checksum.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("bad checksum hex");
        }
    }

    // Must contain comma-separated fields
    if !sentence.contains(',') {
        return Err("no fields");
    }

    Ok(())
}

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = str::from_utf8(data) {
        // Normalise line endings
        for line in s.lines() {
            let _ = validate_nmea(line);
        }
    }
});
