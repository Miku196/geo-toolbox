//! CamoFox web-scraping data ingestion.
//!
//! Reads JSON output from the `camoufox-browser` Pi Agent Skill,
//! validates each record, and feeds into the batch writer.

use geo_core::errors::{GeoError, GeoResult};
use geo_core::types::SpatialRow;
use serde::Deserialize;

/// A single record from a CamoFox scrape session.
///
/// Expected JSON schema:
/// ```json
/// {
///   "name": "Site Name",
///   "lat": 22.54,
///   "lng": 113.93,
///   "type": "forest",
///   "area_ha": 3170.0
/// }
/// ```
#[derive(Debug, Deserialize)]
pub struct CamofoxRecord {
    /// Site name.
    pub name: String,
    /// Latitude.
    pub lat: f64,
    /// Longitude.
    pub lng: f64,
    /// Landcover type or category.
    #[serde(rename = "type", default)]
    pub category: String,
    /// Area in hectares (optional).
    #[serde(default)]
    pub area_ha: Option<f64>,
    /// Catch-all for extra fields.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Result of processing one or more CamoFox records.
#[derive(Debug)]
pub struct IngestResult {
    /// Records that passed validation.
    pub accepted: usize,
    /// Records that failed validation.
    pub rejected: usize,
    /// Rejection reasons (one per failed record).
    pub errors: Vec<String>,
}

/// Parse and validate a CamoFox JSON file.
///
/// Returns a vec of validated [`SpatialRow`]s ready for batch insert,
/// plus an [`IngestResult`] summary.
pub fn parse_camofox_file(
    json_content: &str,
    source_name: &str,
) -> GeoResult<(Vec<SpatialRow>, IngestResult)> {
    // Try array first, then single object
    let records: Vec<CamofoxRecord> = if json_content.trim().starts_with('[') {
        serde_json::from_str(json_content)?
    } else if let Ok(rec) = serde_json::from_str::<CamofoxRecord>(json_content) {
        vec![rec]
    } else {
        // Try FeatureCollection
        let fc: serde_json::Value = serde_json::from_str(json_content)?;
        if let Some(features) = fc["features"].as_array() {
            features
                .iter()
                .map(|f| {
                    let props = &f["properties"];
                    let coords = &f["geometry"]["coordinates"];
                    Ok(CamofoxRecord {
                        name: props["name"].as_str().unwrap_or("unknown").to_string(),
                        lng: coords[0].as_f64().unwrap_or(0.0),
                        lat: coords[1].as_f64().unwrap_or(0.0),
                        category: props["type"].as_str().unwrap_or("").to_string(),
                        area_ha: props["area_ha"].as_f64(),
                        extra: serde_json::Map::new(),
                    })
                })
                .collect::<Result<Vec<_>, GeoError>>()?
        } else {
            return Err(GeoError::Validation(
                "expected JSON array, object, or GeoJSON FeatureCollection".into(),
            ));
        }
    };

    let mut rows = Vec::with_capacity(records.len());
    let mut result = IngestResult {
        accepted: 0,
        rejected: 0,
        errors: Vec::new(),
    };

    for rec in &records {
        // Validate coordinates
        if let Err(e) = geo_core::types::validate_coord(rec.lng, rec.lat) {
            result.rejected += 1;
            result.errors.push(format!("{}: {e}", rec.name));
            tracing::warn!("Rejected {}: {e}", rec.name);
            continue;
        }

        // Build standard 2D Point WKB (little-endian)。
        // 数据源仅有 (lng, lat) 两维，故写标准 2D：byte_order + type=1(Point) + 2 个 double。
        // （旧实现置 0x20000000 EWKB Z/SRID 标志但 type=0 且只写两维，产生 PostGIS 拒读的畸形 WKB。）
        let mut wkb = Vec::with_capacity(21);
        wkb.push(0x01); // byte order: LE
        wkb.extend_from_slice(&1u32.to_le_bytes()); // type: 1 = Point (2D)
        wkb.extend_from_slice(&rec.lng.to_le_bytes());
        wkb.extend_from_slice(&rec.lat.to_le_bytes());

        // Properties as JSON
        let mut props = serde_json::json!({
            "name": rec.name,
            "type": rec.category,
            "source": source_name,
        });
        if let Some(area) = rec.area_ha {
            props["area_ha"] = serde_json::json!(area);
        }
        // Merge extra fields
        for (k, v) in &rec.extra {
            props[k] = v.clone();
        }
        // Embed coordinates for easy querying
        props["lon"] = serde_json::json!(rec.lng);
        props["lat"] = serde_json::json!(rec.lat);

        rows.push(SpatialRow::new(wkb, props, source_name));
        result.accepted += 1;
    }

    tracing::info!(
        "CamoFox ingest: {} accepted, {} rejected ({} records)",
        result.accepted,
        result.rejected,
        records.len()
    );

    Ok((rows, result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_record() {
        let json = r#"{"name": "Wutong Mountain", "lat": 22.55, "lng": 114.06, "type": "forest", "area_ha": 3170}"#;
        let (rows, result) = parse_camofox_file(json, "test").unwrap();
        assert_eq!(result.accepted, 1);
        assert_eq!(result.rejected, 0);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].properties.contains("Wutong Mountain"));
    }

    #[test]
    // 回归：WKB 头曾写 0x20000000（EWKB Z/SRID 标志）但 type=0 且只写 2 维，
    // 生成畸形 WKB（PostGIS 拒读）。标准 2D Point 应为 byte_order=1, type=1, 2 个坐标。
    fn test_wkb_is_valid_2d_point() {
        let json = r#"{"name": "Wutong", "lat": 22.55, "lng": 114.06, "type": "forest"}"#;
        let (rows, _) = parse_camofox_file(json, "test").unwrap();
        let wkb = &rows[0].wkb;

        // WKB Point(2D): byte_order(1) + type(4) + 2 * double(8)
        assert_eq!(wkb.len(), 1 + 4 + 2 * 8, "WKB length for 2D point");
        assert_eq!(wkb[0], 0x01, "byte order must be little-endian");

        let type_raw = u32::from_le_bytes([wkb[1], wkb[2], wkb[3], wkb[4]]);
        assert_eq!(type_raw & 0xFF, 1, "geometry type must be Point");
        assert_eq!(type_raw & 0xFF000000, 0, "no EWKB Z/SRID flag for 2D data");
        assert_eq!(type_raw, 1, "type must be exactly 1 (2D Point)");
        assert_eq!((wkb.len() - 5) / 8, 2, "2D point has exactly 2 coords");

        // 坐标应为 lng/lat 原生双精度
        let lng = f64::from_le_bytes([
            wkb[5], wkb[6], wkb[7], wkb[8], wkb[9], wkb[10], wkb[11], wkb[12],
        ]);
        let lat = f64::from_le_bytes([
            wkb[13], wkb[14], wkb[15], wkb[16], wkb[17], wkb[18], wkb[19], wkb[20],
        ]);
        assert!((lng - 114.06).abs() < 1e-9);
        assert!((lat - 22.55).abs() < 1e-9);
    }

    #[test]
    fn test_parse_array() {
        let json = r#"[
            {"name": "A", "lat": 22.5, "lng": 113.9, "type": "park"},
            {"name": "B", "lat": 22.6, "lng": 114.0, "type": "water"}
        ]"#;
        let (rows, result) = parse_camofox_file(json, "test").unwrap();
        assert_eq!(result.accepted, 2);
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_reject_invalid_coord() {
        let json = r#"{"name": "Bad", "lat": 200.0, "lng": 0.0, "type": "park"}"#;
        let (_, result) = parse_camofox_file(json, "test").unwrap();
        assert_eq!(result.accepted, 0);
        assert_eq!(result.rejected, 1);
    }

    #[test]
    fn test_parse_feature_collection() {
        let json = r#"{
            "type": "FeatureCollection",
            "features": [
                {"type": "Feature", "geometry": {"type": "Point", "coordinates": [113.93, 22.54]}, "properties": {"name": "Shenzhen Bay", "type": "park", "area_ha": 128.5}},
                {"type": "Feature", "geometry": {"type": "Point", "coordinates": [114.06, 22.55]}, "properties": {"name": "Wutong", "type": "forest", "area_ha": 3170}}
            ]
        }"#;
        let (rows, result) = parse_camofox_file(json, "test").unwrap();
        assert_eq!(result.accepted, 2);
        assert_eq!(rows.len(), 2);
    }
}
