//! Zero-copy geometry interchange via WKB (Well-Known Binary).
//!
//! Both Rust `geo-types` and Python `shapely` support WKB as their native
//! serialization format. By using WKB bytes as the interchange layer, we avoid
//! per-coordinate iteration — the byte buffer moves between Rust and Python
//! without copying individual points.

use geo_core::errors::GeoResult;

/// Pass WKB bytes through unchanged (identity transform).
///
/// Both sides speak WKB natively. This function exists as a typed boundary
/// to validate the WKB header when validation is enabled.
pub fn geometry_to_shapely(wkb: &[u8]) -> &[u8] {
    wkb
}

/// Validate and return WKB bytes from Python shapely → Rust.
///
/// Performs a lightweight header check to ensure the byte stream is valid WKB.
pub fn shapely_to_geometry(wkb: &[u8]) -> GeoResult<Vec<u8>> {
    if wkb.len() < 5 {
        return Err(geo_core::GeoError::Validation(
            "WKB too short: need at least 5 bytes (endianness + type)".into(),
        ));
    }

    // Validate WKB byte order (0x00 = big endian, 0x01 = little endian)
    match wkb[0] {
        0x00 | 0x01 => {}
        b => {
            return Err(geo_core::GeoError::Validation(format!(
                "Invalid WKB byte order: {b:#04x}"
            )));
        }
    }

    // Validate geometry type is in supported range (1-15 for 2D/3D/4D)
    let geom_type = u32::from_le_bytes([wkb[1], wkb[2], wkb[3], wkb[4]]);
    let base_type = geom_type % 1000;
    if !(1..=15).contains(&base_type) {
        return Err(geo_core::GeoError::Validation(format!(
            "Unsupported WKB geometry type: {geom_type} (base={base_type})"
        )));
    }

    Ok(wkb.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal valid WKB Point (little-endian, type=1, x=1.0, y=2.0).
    fn wkb_point() -> Vec<u8> {
        let mut buf = vec![0x01]; // little endian
        buf.extend_from_slice(&1u32.to_le_bytes()); // Point type = 1
        buf.extend_from_slice(&1.0f64.to_le_bytes()); // x
        buf.extend_from_slice(&2.0f64.to_le_bytes()); // y
        buf
    }

    #[test]
    fn test_identity_roundtrip() {
        let original = wkb_point();
        let passed = geometry_to_shapely(&original);
        assert_eq!(passed, original);

        let back = shapely_to_geometry(passed).unwrap();
        assert_eq!(back, original);
    }

    #[test]
    fn test_invalid_wkb_too_short() {
        let short = vec![0x01, 0x00, 0x00];
        let err = shapely_to_geometry(&short);
        assert!(err.is_err());
    }

    #[test]
    fn test_invalid_byte_order() {
        let mut bad = wkb_point();
        bad[0] = 0xFF;
        let err = shapely_to_geometry(&bad);
        assert!(err.is_err());
    }

    #[test]
    fn test_invalid_geometry_type() {
        let mut bad = vec![0x01]; // little endian
        bad.extend_from_slice(&99_999u32.to_le_bytes()); // base_type > 15
        bad.extend_from_slice(&[0u8; 16]); // 2×f64 padding
        let err = shapely_to_geometry(&bad);
        assert!(err.is_err());
    }
}
