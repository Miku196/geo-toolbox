//! NMEA 0183 sentence parser for GPS data streams.
//!
//! Parses `$GPGGA` (fix data) and `$GPRMC` (recommended minimum) sentences.
//! Used by `gps-ingestor` to read from serial port or file.

use geo_core::errors::{GeoError, GeoResult};

/// A parsed GPS fix from a `$GPGGA` sentence.
#[derive(Debug, Clone, PartialEq)]
pub struct GgaFix {
    /// UTC time: HHMMSS.SS
    pub time: String,
    /// Latitude in decimal degrees.
    pub lat: f64,
    /// Longitude in decimal degrees.
    pub lng: f64,
    /// Fix quality: 0=invalid, 1=GPS, 2=DGPS, ...
    pub quality: u8,
    /// Number of satellites in use.
    pub satellites: u8,
    /// Horizontal dilution of precision.
    pub hdop: f64,
    /// Altitude above mean sea level (meters).
    pub altitude: f64,
}

/// A parsed `$GPRMC` sentence.
#[derive(Debug, Clone, PartialEq)]
pub struct RmcSentence {
    /// UTC time.
    pub time: String,
    /// Status: 'A' = valid, 'V' = warning.
    pub status: char,
    /// Latitude.
    pub lat: f64,
    /// Longitude.
    pub lng: f64,
    /// Speed over ground (knots).
    pub speed_knots: f64,
    /// Track angle (degrees true).
    pub track: f64,
    /// Date: DDMMYY.
    pub date: String,
}

/// A combined GPS reading from matching GGA + RMC sentences.
#[derive(Debug, Clone)]
pub struct GpsReading {
    /// UTC timestamp.
    pub timestamp: String,
    /// Latitude.
    pub lat: f64,
    /// Longitude.
    pub lng: f64,
    /// Fix quality.
    pub quality: u8,
    /// Satellite count.
    pub satellites: u8,
    /// HDOP.
    pub hdop: f64,
    /// Altitude (meters).
    pub altitude: f64,
    /// Speed (knots).
    pub speed_knots: f64,
    /// Track (degrees).
    pub track: f64,
}

/// Parse a $GPGGA sentence.
///
/// Format: `$GPGGA,HHMMSS.SS,lat,N,lng,E,quality,sats,hdop,alt,M,...`
pub fn parse_gga(sentence: &str) -> GeoResult<GgaFix> {
    let fields: Vec<&str> = sentence.split(',').collect();

    if fields.len() < 14 {
        return Err(GeoError::Validation(format!(
            "GGA sentence too short: {} fields",
            fields.len()
        )));
    }

    let talker = fields.first().unwrap_or(&"");
    if !talker.ends_with("GGA") {
        return Err(GeoError::Validation(format!(
            "not a GGA sentence: {talker}"
        )));
    }

    let time = fields[1].to_string();

    let lat_raw: f64 = fields[2]
        .parse()
        .map_err(|_| GeoError::Validation("bad latitude".into()))?;
    let lat = nmea_to_decimal(lat_raw, fields.get(3).unwrap_or(&"N"));

    let lng_raw: f64 = fields[4]
        .parse()
        .map_err(|_| GeoError::Validation("bad longitude".into()))?;
    let lng = nmea_to_decimal(lng_raw, fields.get(5).unwrap_or(&"E"));

    let quality: u8 = fields[6].parse().unwrap_or(0);
    let satellites: u8 = fields[7].parse().unwrap_or(0);
    let hdop: f64 = fields[8].parse().unwrap_or(99.9);
    let altitude: f64 = fields[9].parse().unwrap_or(0.0);

    Ok(GgaFix {
        time,
        lat,
        lng,
        quality,
        satellites,
        hdop,
        altitude,
    })
}

/// Parse a $GPRMC sentence.
pub fn parse_rmc(sentence: &str) -> GeoResult<RmcSentence> {
    let fields: Vec<&str> = sentence.split(',').collect();

    if fields.len() < 12 {
        return Err(GeoError::Validation(format!(
            "RMC sentence too short: {} fields",
            fields.len()
        )));
    }

    let talker = fields[0];
    if !talker.ends_with("RMC") {
        return Err(GeoError::Validation(format!(
            "not an RMC sentence: {talker}"
        )));
    }

    let time = fields[1].to_string();
    let status = fields[2].chars().next().unwrap_or('V');

    let lat_raw: f64 = fields[3].parse().unwrap_or(0.0);
    let lat = nmea_to_decimal(lat_raw, fields.get(4).unwrap_or(&"N"));

    let lng_raw: f64 = fields[5].parse().unwrap_or(0.0);
    let lng = nmea_to_decimal(lng_raw, fields.get(6).unwrap_or(&"E"));

    let speed: f64 = fields[7].parse().unwrap_or(0.0);
    let track: f64 = fields[8].parse().unwrap_or(0.0);
    let date = fields[9].to_string();

    Ok(RmcSentence {
        time,
        status,
        lat,
        lng,
        speed_knots: speed,
        track,
        date,
    })
}

/// Process a raw NMEA line (possibly prefixed with checksum, e.g. `$GPGGA,...*XX`).
pub fn parse_nmea_line(line: &str) -> GeoResult<NmeaMessage> {
    let line = line.trim();
    if line.is_empty() || !line.starts_with('$') {
        return Err(GeoError::Validation("not an NMEA sentence".into()));
    }

    // Strip checksum if present
    let sentence = if let Some(idx) = line.find('*') {
        &line[..idx]
    } else {
        line
    };

    if sentence.contains("GGA") {
        Ok(NmeaMessage::Gga(parse_gga(sentence)?))
    } else if sentence.contains("RMC") {
        Ok(NmeaMessage::Rmc(parse_rmc(sentence)?))
    } else {
        Ok(NmeaMessage::Unknown(line.to_string()))
    }
}

/// Parsed NMEA message variant.
#[derive(Debug, Clone)]
pub enum NmeaMessage {
    /// GPS fix data.
    Gga(GgaFix),
    /// Recommended minimum.
    Rmc(RmcSentence),
    /// Unrecognized sentence type.
    Unknown(String),
}

/// Convert NMEA lat/lng format (DDMM.MMMM) to decimal degrees.
///
/// Example: 2232.1234,N → 22.53539
fn nmea_to_decimal(raw: f64, hemisphere: &str) -> f64 {
    let degrees = (raw / 100.0).floor();
    let minutes = raw - degrees * 100.0;
    let decimal = degrees + minutes / 60.0;

    match hemisphere {
        "S" | "W" => -decimal,
        _ => decimal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gga() {
        let gga = "$GPGGA,123519,4807.038,N,01131.000,E,1,08,1.2,545.4,M,46.9,M,,*47";
        let fix = parse_gga(gga).unwrap();
        assert_eq!(fix.time, "123519");
        assert_eq!(fix.quality, 1);
        assert_eq!(fix.satellites, 8);
        assert!((fix.hdop - 1.2).abs() < 0.01);
        assert!((fix.altitude - 545.4).abs() < 0.01);
        // 4807.038,N → 48 + 7.038/60 = 48.1173
        assert!((fix.lat - 48.1173).abs() < 0.001);
    }

    #[test]
    fn test_parse_rmc() {
        let rmc = "$GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W*6A";
        let fix = parse_rmc(rmc).unwrap();
        assert_eq!(fix.status, 'A');
        assert!((fix.speed_knots - 22.4).abs() < 0.1);
        assert_eq!(fix.date, "230394");
    }

    #[test]
    fn test_nmea_to_decimal() {
        assert!((nmea_to_decimal(4807.038, "N") - 48.1173).abs() < 0.001);
        assert!((nmea_to_decimal(1130.000, "E") - 11.5000).abs() < 0.001);
    }

    #[test]
    fn test_parse_nmea_line_with_checksum() {
        let msg = parse_nmea_line("$GPGGA,123519,2232.1234,N,11355.5678,E,1,12,0.8,100.0,M,,,*7F")
            .unwrap();
        match msg {
            NmeaMessage::Gga(fix) => {
                assert_eq!(fix.satellites, 12);
            }
            _ => panic!("expected GGA"),
        }
    }

    #[test]
    fn test_parse_unknown() {
        let msg = parse_nmea_line("$GPGSV,3,1,12,...").unwrap();
        match msg {
            NmeaMessage::Unknown(_) => {}
            _ => panic!("expected Unknown"),
        }
    }
}
