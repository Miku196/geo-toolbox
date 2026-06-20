//! Data ingestion: NMEA sentence parsing and coordinate validation.

use pyo3::prelude::*;

/// Parse a single NMEA 0183 sentence.
pub fn parse_nmea_impl(sentence: &str) -> PyResult<std::collections::HashMap<String, String>> {
    let mut result = std::collections::HashMap::new();
    let s = sentence.trim();
    if s.is_empty() {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Empty sentence",
        ));
    }
    if !s.starts_with('$') && !s.starts_with('!') {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Not an NMEA sentence",
        ));
    }
    let body = if let Some(star_idx) = s.rfind('*') {
        &s[1..star_idx]
    } else {
        &s[1..]
    };
    let parts: Vec<&str> = body.split(',').collect();
    if parts.is_empty() {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "No talker/sentence ID",
        ));
    }
    let talker_sentence = parts[0];
    result.insert("sentence_type".to_string(), talker_sentence.to_string());
    match talker_sentence
        .rsplitn(2, "GP")
        .next()
        .or_else(|| talker_sentence.rsplitn(2, "GL").next())
        .or_else(|| talker_sentence.rsplitn(2, "GN").next())
        .or_else(|| talker_sentence.rsplitn(2, "BD").next())
    {
        Some("GGA") => parse_gga(&parts, &mut result),
        Some("RMC") => parse_rmc(&parts, &mut result),
        Some("GLL") => parse_gll(&parts, &mut result),
        Some("VTG") => parse_vtg(&parts, &mut result),
        _ => {
            for (i, val) in parts.iter().skip(1).enumerate() {
                result.insert(format!("field_{i}"), val.to_string());
            }
        }
    }
    Ok(result)
}

/// Parse multiple NMEA sentences (one per line).
pub fn parse_nmea_batch_impl(
    sentences: Vec<String>,
) -> PyResult<Vec<std::collections::HashMap<String, String>>> {
    let mut results = Vec::with_capacity(sentences.len());
    for s in &sentences {
        results.push(parse_nmea_impl(s)?);
    }
    Ok(results)
}

/// Validate a WGS84 coordinate pair. Returns (ok, message).
pub fn validate_coord_impl(lat: f64, lon: f64) -> (bool, String) {
    if !(-90.0..=90.0).contains(&lat) {
        return (false, format!("Latitude {lat} out of range [-90, 90]"));
    }
    if !(-180.0..=180.0).contains(&lon) {
        return (false, format!("Longitude {lon} out of range [-180, 180]"));
    }
    if lat == 0.0 && lon == 0.0 {
        return (false, "Coordinate at (0, 0) — likely uninitialized".into());
    }
    (true, "OK".into())
}

/// Validate a GPS fix quality value.
pub fn validate_gps_fix_impl(quality: u8) -> (bool, String) {
    match quality {
        0 => (false, "Invalid fix".into()),
        1 => (true, "GPS fix (SPS)".into()),
        2 => (true, "DGPS fix".into()),
        4 => (true, "RTK fixed".into()),
        5 => (true, "RTK float".into()),
        6 => (true, "Estimated (dead reckoning)".into()),
        _ => (false, format!("Unknown fix quality: {quality}")),
    }
}

/// Validate a sensor reading (check for reasonable numeric range).
pub fn validate_sensor_reading_impl(
    value: f64,
    sensor_type: &str,
    min_val: f64,
    max_val: f64,
) -> (bool, String) {
    if !(min_val..=max_val).contains(&value) {
        return (
            false,
            format!("{sensor_type} value {value} out of range [{min_val}, {max_val}]"),
        );
    }
    (true, "OK".into())
}

fn parse_gga(parts: &[&str], result: &mut std::collections::HashMap<String, String>) {
    if parts.len() > 1 {
        result.insert("utc_time".into(), parts[1].to_string());
    }
    if parts.len() > 2 {
        result.insert("lat".into(), parts[2].to_string());
    }
    if parts.len() > 3 {
        result.insert("lat_dir".into(), parts[3].to_string());
    }
    if parts.len() > 4 {
        result.insert("lon".into(), parts[4].to_string());
    }
    if parts.len() > 5 {
        result.insert("lon_dir".into(), parts[5].to_string());
    }
    if parts.len() > 6 {
        result.insert("quality".into(), parts[6].to_string());
    }
    if parts.len() > 7 {
        result.insert("satellites".into(), parts[7].to_string());
    }
    if parts.len() > 8 {
        result.insert("hdop".into(), parts[8].to_string());
    }
    if parts.len() > 9 {
        result.insert("altitude".into(), parts[9].to_string());
    }
    if parts.len() > 10 {
        result.insert("alt_unit".into(), parts[10].to_string());
    }
    if parts.len() > 11 {
        result.insert("geoid_sep".into(), parts[11].to_string());
    }
}
fn parse_rmc(parts: &[&str], result: &mut std::collections::HashMap<String, String>) {
    if parts.len() > 1 {
        result.insert("utc_time".into(), parts[1].to_string());
    }
    if parts.len() > 2 {
        result.insert("status".into(), parts[2].to_string());
    }
    if parts.len() > 3 {
        result.insert("lat".into(), parts[3].to_string());
    }
    if parts.len() > 4 {
        result.insert("lat_dir".into(), parts[4].to_string());
    }
    if parts.len() > 5 {
        result.insert("lon".into(), parts[5].to_string());
    }
    if parts.len() > 6 {
        result.insert("lon_dir".into(), parts[6].to_string());
    }
    if parts.len() > 7 {
        result.insert("speed_kn".into(), parts[7].to_string());
    }
    if parts.len() > 8 {
        result.insert("course".into(), parts[8].to_string());
    }
    if parts.len() > 9 {
        result.insert("date".into(), parts[9].to_string());
    }
}
fn parse_gll(parts: &[&str], result: &mut std::collections::HashMap<String, String>) {
    if parts.len() > 1 {
        result.insert("lat".into(), parts[1].to_string());
    }
    if parts.len() > 2 {
        result.insert("lat_dir".into(), parts[2].to_string());
    }
    if parts.len() > 3 {
        result.insert("lon".into(), parts[3].to_string());
    }
    if parts.len() > 4 {
        result.insert("lon_dir".into(), parts[4].to_string());
    }
    if parts.len() > 5 {
        result.insert("utc_time".into(), parts[5].to_string());
    }
    if parts.len() > 6 {
        result.insert("status".into(), parts[6].to_string());
    }
}
fn parse_vtg(parts: &[&str], result: &mut std::collections::HashMap<String, String>) {
    if parts.len() > 1 {
        result.insert("course_true".into(), parts[1].to_string());
    }
    if parts.len() > 3 {
        result.insert("course_mag".into(), parts[3].to_string());
    }
    if parts.len() > 5 {
        result.insert("speed_kn".into(), parts[5].to_string());
    }
    if parts.len() > 7 {
        result.insert("speed_kph".into(), parts[7].to_string());
    }
}
