//! CamoFox web-scraping data ingestion.
//!
//! Reads JSON output from the `camoufox-browser` Pi Agent Skill,
//! validates each record, and feeds into the batch writer.

use geo_core::errors::{GeoError, GeoResult};
use geo_store::batch_writer::SpatialRow;
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
pub fn parse_camofox_file(json_content: &str, source_name: &str) -> GeoResult<(Vec<SpatialRow>, IngestResult)> {
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

        // Build geometry WKB (Point, little-endian)
        let mut wkb = Vec::with_capacity(21);
        wkb.push(0x01); // byte order: LE
        wkb.extend_from_slice(&0x20000000u32.to_le_bytes()); // Point 2D
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
        result.accepted, result.rejected, records.len()
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
