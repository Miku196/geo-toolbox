//! Data quality validation rules for ingestion pipeline.
//!
//! Each validator returns `Ok(())` if the record passes,
//! or `Err(GeoError::Validation(...))` with a human-readable message.

use geo_core::errors::{GeoError, GeoResult};
use geo_core::types::validate_coord;

/// Validate a single coordinate pair that came from web scraping.
pub fn validate_web_coord(lon: f64, lat: f64) -> GeoResult<()> {
    validate_coord(lon, lat)
}

/// Validate a GPS NMEA fix.
///
/// Returns `Err` if HDOP is too high or satellite count is too low.
pub fn validate_gps_fix(hdop: f64, satellites: u8) -> GeoResult<()> {
    if hdop > 5.0 {
        return Err(GeoError::Validation(format!(
            "HDOP too high: {hdop:.1} (max 5.0)"
        )));
    }
    if satellites < 4 {
        return Err(GeoError::Validation(format!(
            "too few satellites: {satellites} (min 4)"
        )));
    }
    Ok(())
}

/// Validate an IoT sensor reading.
///
/// Checks that the value is within a reasonable range for the sensor type.
pub fn validate_iot_reading(sensor_type: &str, value: f64) -> GeoResult<()> {
    match sensor_type {
        "temperature" if (-50.0..=60.0).contains(&value) => Ok(()),
        "humidity" if (0.0..=100.0).contains(&value) => Ok(()),
        "pm25" if (0.0..1000.0).contains(&value) => Ok(()),
        "temperature" => Err(GeoError::Validation(format!(
            "temperature {value}°C out of range [-50, 60]"
        ))),
        "humidity" => Err(GeoError::Validation(format!(
            "humidity {value}% out of range [0, 100]"
        ))),
        "pm25" => Err(GeoError::Validation(format!(
            "PM2.5 {value} out of range [0, 1000)"
        ))),
        _ => Err(GeoError::Validation(format!(
            "unknown sensor type: {sensor_type}"
        ))),
    }
}

/// Validate that required JSON fields exist and are non-null.
pub fn validate_required_fields(record: &serde_json::Value, fields: &[&str]) -> GeoResult<()> {
    for field in fields {
        match record.get(field) {
            None | Some(serde_json::Value::Null) => {
                return Err(GeoError::Validation(format!(
                    "missing required field: {field}"
                )));
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gps_fix_good() {
        assert!(validate_gps_fix(1.5, 12).is_ok());
    }

    #[test]
    fn test_gps_fix_bad_hdop() {
        assert!(validate_gps_fix(6.0, 10).is_err());
    }

    #[test]
    fn test_gps_fix_few_sats() {
        assert!(validate_gps_fix(2.0, 3).is_err());
    }

    #[test]
    fn test_iot_temperature_good() {
        assert!(validate_iot_reading("temperature", 25.0).is_ok());
    }

    #[test]
    fn test_iot_temperature_bad() {
        assert!(validate_iot_reading("temperature", 100.0).is_err());
    }

    #[test]
    fn test_required_fields_missing() {
        let rec = serde_json::json!({"name": "test"});
        assert!(validate_required_fields(&rec, &["name", "lat"]).is_err());
    }

    #[test]
    fn test_required_fields_ok() {
        let rec = serde_json::json!({"name": "test", "lat": 22.5, "lng": 113.9});
        assert!(validate_required_fields(&rec, &["name", "lat", "lng"]).is_ok());
    }
}
