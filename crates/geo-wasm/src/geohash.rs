//! Browser-side geohash encoding, decoding, and neighbor lookup.
//!
//! Pure-Rust via [`geo_index::geohash`].
//! All functions return JSON strings for reliable JS interop.

use geo_core::errors::GeoResult;
use geo_core::types::BBox;
use wasm_bindgen::prelude::*;

/// Encode a (lon, lat) pair into a geohash string.
#[wasm_bindgen(js_name = geohashEncode)]
pub fn geohash_encode(lon: f64, lat: f64, precision: usize) -> String {
    geohash_encode_inner(lon, lat, precision)
}

fn geohash_encode_inner(lon: f64, lat: f64, precision: usize) -> String {
    geo_index::geohash::encode(lon, lat, precision)
}

/// Decode a geohash into its center coordinate and bounding box.
///
/// Returns JSON: `{"lat":..., "lon":..., "bbox":{"minLon":...,"minLat":...,"maxLon":...,"maxLat":...}}`
#[wasm_bindgen(js_name = geohashDecode)]
pub fn geohash_decode(hash: &str) -> Result<String, JsValue> {
    geohash_decode_inner(hash).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn geohash_decode_inner(hash: &str) -> GeoResult<String> {
    match geo_index::geohash::decode(hash) {
        Some((lat, lon, bbox)) => {
            let json = serde_json::json!({
                "lat": lat,
                "lon": lon,
                "bbox": {
                    "minLon": bbox.min_x,
                    "minLat": bbox.min_y,
                    "maxLon": bbox.max_x,
                    "maxLat": bbox.max_y,
                }
            });
            serde_json::to_string(&json).map_err(geo_core::errors::GeoError::Serde)
        }
        None => Err(geo_core::errors::GeoError::Validation(
            "Invalid geohash".into(),
        )),
    }
}

/// Get all 8 neighbors of a geohash.
///
/// Returns JSON array of geohash strings.
#[wasm_bindgen(js_name = geohashNeighbors)]
pub fn geohash_neighbors(hash: &str) -> Result<String, JsValue> {
    geohash_neighbors_inner(hash).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn geohash_neighbors_inner(hash: &str) -> GeoResult<String> {
    let nbs = geo_index::geohash::neighbors(hash);
    serde_json::to_string(&nbs).map_err(geo_core::errors::GeoError::Serde)
}

/// Get all geohashes that intersect a bounding box.
///
/// Returns JSON array of geohash strings.
#[wasm_bindgen(js_name = bboxToGeohashes)]
pub fn bbox_to_geohashes(
    min_lon: f64,
    min_lat: f64,
    max_lon: f64,
    max_lat: f64,
    precision: usize,
) -> Result<String, JsValue> {
    bbox_to_geohashes_inner(min_lon, min_lat, max_lon, max_lat, precision)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

fn bbox_to_geohashes_inner(
    min_lon: f64,
    min_lat: f64,
    max_lon: f64,
    max_lat: f64,
    precision: usize,
) -> GeoResult<String> {
    let bbox = BBox::new(min_lon, min_lat, max_lon, max_lat);
    let hashes = geo_index::geohash::bbox_to_geohashes(&bbox, precision);
    serde_json::to_string(&hashes).map_err(geo_core::errors::GeoError::Serde)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let hash = geohash_encode_inner(104.0, 30.5, 6);
        assert_eq!(hash.len(), 6);

        let decoded = geohash_decode_inner(&hash).unwrap();
        assert!(decoded.contains("lat"));
        assert!(decoded.contains("lon"));
        assert!(decoded.contains("bbox"));
    }

    #[test]
    fn test_neighbors() {
        let hash = geohash_encode_inner(104.0, 30.5, 5);
        let nbs = geohash_neighbors_inner(&hash).unwrap();
        let arr: Vec<String> = serde_json::from_str(&nbs).unwrap();
        assert_eq!(arr.len(), 8);
    }

    #[test]
    fn test_bbox_to_geohashes() {
        let result = bbox_to_geohashes_inner(104.0, 30.5, 104.1, 30.6, 4).unwrap();
        let arr: Vec<String> = serde_json::from_str(&result).unwrap();
        assert!(!arr.is_empty());
    }

    #[test]
    fn test_invalid_decode() {
        // Characters outside base32 alphabet
        let result = geohash_decode_inner("!!!!!");
        assert!(result.is_err());
    }
}
