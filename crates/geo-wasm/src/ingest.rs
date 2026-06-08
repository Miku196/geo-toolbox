//! WASM-facing data parsing & validation.
//!
//! Pure-Rust NMEA 0183 parser and coordinate validators.
//! Zero unwrap — all errors returned as JsValue.

use wasm_bindgen::prelude::*;
use serde::Serialize;

use geo_core::errors::GeoError;
use geo_core::types::validate_coord as geo_validate_coord;

// ── NMEA parsing ────────────────────────────────────────────────

#[wasm_bindgen(js_name = parseNmea)]
pub fn parse_nmea(sentence: &str) -> Result<JsValue, JsValue> {
    let msg = parse_nmea_line(sentence)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let js_msg = match msg {
        NmeaMessage::Gga(fix) => to_js(&GgaFixJs {
            msg_type: "GGA".into(), time: fix.time, lat: fix.lat, lng: fix.lng,
            quality: fix.quality, satellites: fix.satellites, hdop: fix.hdop, altitude: fix.altitude,
        })?,
        NmeaMessage::Rmc(fix) => to_js(&RmcFixJs {
            msg_type: "RMC".into(), time: fix.time, status: fix.status.to_string(),
            lat: fix.lat, lng: fix.lng, speed_knots: fix.speed_knots, track: fix.track, date: fix.date,
        })?,
        NmeaMessage::Unknown(raw) => to_js(&serde_json::json!({"type":"Unknown","raw":raw}))?,
    };

    Ok(js_msg)
}

#[wasm_bindgen(js_name = parseNmeaBatch)]
pub fn parse_nmea_batch(lines: &str) -> Result<JsValue, JsValue> {
    let results: Vec<serde_json::Value> = lines.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| match parse_nmea_line(line) {
            Ok(msg) => match msg {
                NmeaMessage::Gga(f) => serde_json::json!({
                    "type":"GGA","time":f.time,"lat":f.lat,"lng":f.lng,
                    "quality":f.quality,"satellites":f.satellites,"hdop":f.hdop,"altitude":f.altitude
                }),
                NmeaMessage::Rmc(f) => serde_json::json!({
                    "type":"RMC","time":f.time,"status":f.status.to_string(),
                    "lat":f.lat,"lng":f.lng,"speed_knots":f.speed_knots,"track":f.track,"date":f.date
                }),
                NmeaMessage::Unknown(raw) => serde_json::json!({"type":"Unknown","raw":raw}),
            },
            Err(e) => serde_json::json!({"error": e.to_string()}),
        })
        .collect();

    to_js(&results)
}

// ── Validation ───────────────────────────────────────────────────

#[wasm_bindgen(js_name = validateGpsFix)]
pub fn validate_gps_fix(hdop: f64, satellites: u8) -> Result<JsValue, JsValue> {
    let r = if hdop > 5.0 {
        json_valid(false, &format!("HDOP too high: {hdop:.1}"))
    } else if satellites < 4 {
        json_valid(false, &format!("too few satellites: {satellites}"))
    } else {
        json_valid(true, "")
    };
    to_js(&r)
}

#[wasm_bindgen(js_name = validateSensorReading)]
pub fn validate_sensor_reading(sensor_type: &str, value: f64) -> Result<JsValue, JsValue> {
    let valid = match sensor_type {
        "temperature" => (-50.0..=60.0).contains(&value),
        "humidity" => (0.0..=100.0).contains(&value),
        "pm25" => (0.0..1000.0).contains(&value),
        _ => false,
    };
    to_js(&serde_json::json!({
        "valid": valid,
        "reason": if valid { serde_json::Value::Null } else {
            serde_json::json!(format!("{sensor_type} {value} out of range"))
        }
    }))
}

#[wasm_bindgen(js_name = validateCoord)]
pub fn validate_coord(lon: f64, lat: f64) -> Result<JsValue, JsValue> {
    match geo_validate_coord(lon, lat) {
        Ok(()) => to_js(&serde_json::json!({"valid":true})),
        Err(e) => to_js(&serde_json::json!({"valid":false,"reason":e.to_string()})),
    }
}

// ── Core NMEA parser (pure Rust, no wasm-bindgen) ───────────────

#[derive(Debug, Clone)]
struct GgaFix { time: String, lat: f64, lng: f64, quality: u8, satellites: u8, hdop: f64, altitude: f64 }

#[derive(Debug, Clone)]
struct RmcSentence { time: String, status: char, lat: f64, lng: f64, speed_knots: f64, track: f64, date: String }

#[derive(Debug, Clone)]
enum NmeaMessage { Gga(GgaFix), Rmc(RmcSentence), Unknown(String) }

fn parse_nmea_line(line: &str) -> Result<NmeaMessage, GeoError> {
    let line = line.trim();
    if line.is_empty() || !line.starts_with('$') {
        return Err(GeoError::Validation("not an NMEA sentence".into()));
    }
    let sentence = if let Some(idx) = line.find('*') { &line[..idx] } else { line };
    if sentence.contains("GGA") {
        Ok(NmeaMessage::Gga(parse_gga(sentence)?))
    } else if sentence.contains("RMC") {
        Ok(NmeaMessage::Rmc(parse_rmc(sentence)?))
    } else {
        Ok(NmeaMessage::Unknown(line.to_string()))
    }
}

fn parse_gga(sentence: &str) -> Result<GgaFix, GeoError> {
    let fields: Vec<&str> = sentence.split(',').collect();
    if fields.len() < 14 { return Err(GeoError::Validation("GGA too short".into())); }
    if !fields[0].ends_with("GGA") { return Err(GeoError::Validation("not GGA".into())); }
    let lat = nmea_to_decimal(fields[2].parse().map_err(|_| GeoError::Validation("bad lat".into()))?, fields.get(3).unwrap_or(&"N"));
    let lng = nmea_to_decimal(fields[4].parse().map_err(|_| GeoError::Validation("bad lng".into()))?, fields.get(5).unwrap_or(&"E"));
    Ok(GgaFix {
        time: fields[1].into(), lat, lng,
        quality: fields[6].parse().unwrap_or(0),
        satellites: fields[7].parse().unwrap_or(0),
        hdop: fields[8].parse().unwrap_or(99.9),
        altitude: fields[9].parse().unwrap_or(0.0),
    })
}

fn parse_rmc(sentence: &str) -> Result<RmcSentence, GeoError> {
    let fields: Vec<&str> = sentence.split(',').collect();
    if fields.len() < 12 { return Err(GeoError::Validation("RMC too short".into())); }
    if !fields[0].ends_with("RMC") { return Err(GeoError::Validation("not RMC".into())); }
    let lat = nmea_to_decimal(fields[3].parse().unwrap_or(0.0), fields.get(4).unwrap_or(&"N"));
    let lng = nmea_to_decimal(fields[5].parse().unwrap_or(0.0), fields.get(6).unwrap_or(&"E"));
    Ok(RmcSentence {
        time: fields[1].into(), status: fields[2].chars().next().unwrap_or('V'), lat, lng,
        speed_knots: fields[7].parse().unwrap_or(0.0),
        track: fields[8].parse().unwrap_or(0.0),
        date: fields[9].into(),
    })
}

fn nmea_to_decimal(raw: f64, hemisphere: &str) -> f64 {
    let degrees = (raw / 100.0).floor();
    let decimal = degrees + (raw - degrees * 100.0) / 60.0;
    match hemisphere { "S" | "W" => -decimal, _ => decimal }
}

// ── Helpers ──────────────────────────────────────────────────────

fn json_valid(valid: bool, reason: &str) -> serde_json::Value {
    if valid {
        serde_json::json!({"valid": true})
    } else {
        serde_json::json!({"valid": false, "reason": reason})
    }
}

fn to_js(v: &impl Serialize) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(v).map_err(|e| JsValue::from_str(&e.to_string()))
}

// ── JS-friendly types ────────────────────────────────────────────

#[derive(Serialize)] struct GgaFixJs { #[serde(rename="type")] msg_type: String, time: String, lat: f64, lng: f64, quality: u8, satellites: u8, hdop: f64, altitude: f64 }
#[derive(Serialize)] struct RmcFixJs { #[serde(rename="type")] msg_type: String, time: String, status: String, lat: f64, lng: f64, speed_knots: f64, track: f64, date: String }
