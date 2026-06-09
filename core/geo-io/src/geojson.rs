//! GeoJSON 解析工具。
//!
//! 解析 FeatureCollection，提取 Feature 属性和几何。

use geo_core::errors::{GeoError, GeoResult};
use geo_core::types::BBox;
use serde::{Deserialize, Serialize};

/// 一个 GeoJSON Feature 的轻量表示。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoJsonFeature {
    /// Feature ID（如果存在）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// 属性表（任意 JSON）。
    pub properties: serde_json::Value,
    /// 几何对象（GeoJSON geometry）。
    pub geometry: serde_json::Value,
}

/// 解析 GeoJSON FeatureCollection 字符串。
///
/// 返回 feature 列表和整体 bbox。
pub fn parse_feature_collection(geojson: &str) -> GeoResult<(Vec<GeoJsonFeature>, Option<BBox>)> {
    let fc: serde_json::Value = serde_json::from_str(geojson)
        .map_err(|e| GeoError::Validation(format!("Invalid GeoJSON: {e}")))?;

    let fc_type = fc["type"].as_str().unwrap_or("");
    if fc_type != "FeatureCollection" {
        return Err(GeoError::Validation(format!(
            "Expected FeatureCollection, got '{fc_type}'"
        )));
    }

    let features_arr = fc["features"].as_array()
        .ok_or_else(|| GeoError::Validation("GeoJSON has no 'features' array".into()))?;

    let mut features = Vec::with_capacity(features_arr.len());

    for feat_value in features_arr {
        let id = feat_value["id"].as_str().map(|s| s.to_string());
        let properties = feat_value["properties"].clone();
        let geometry = feat_value["geometry"].clone();

        features.push(GeoJsonFeature {
            id,
            properties: if properties.is_null() {
                serde_json::json!({})
            } else {
                properties
            },
            geometry,
        });
    }

    let bbox = extract_bbox_from_fc(&fc);

    Ok((features, bbox))
}

/// 从所有 feature 中提取整体边界框。
pub fn extract_bbox(geojson: &str) -> GeoResult<BBox> {
    let (_, bbox) = parse_feature_collection(geojson)?;
    bbox.ok_or_else(|| GeoError::Validation("Cannot compute bbox from empty/point features".into()))
}

fn extract_bbox_from_fc(fc: &serde_json::Value) -> Option<BBox> {
    let features = fc["features"].as_array()?;

    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    let mut found = false;

    for feat in features {
        if let Some(coords) = extract_all_coords(&feat["geometry"]) {
            for (x, y) in coords {
                if x < min_x { min_x = x; }
                if y < min_y { min_y = y; }
                if x > max_x { max_x = x; }
                if y > max_y { max_y = y; }
                found = true;
            }
        }
    }

    if found {
        Some(BBox::new(min_x, min_y, max_x, max_y))
    } else {
        None
    }
}

fn extract_all_coords(geom: &serde_json::Value) -> Option<Vec<(f64, f64)>> {
    let geom_type = geom["type"].as_str()?;
    let coords = &geom["coordinates"];

    match geom_type {
        "Polygon" => {
            let rings = coords.as_array()?;
            rings.first()?.as_array()?.iter()
                .filter_map(|p| {
                    let arr = p.as_array()?;
                    Some((arr.get(0)?.as_f64()?, arr.get(1)?.as_f64()?))
                })
                .collect::<Vec<_>>()
                .into()
        }
        "MultiPolygon" => {
            let polys = coords.as_array()?;
            let mut all = Vec::new();
            for poly in polys {
                let rings = poly.as_array()?;
                for ring in rings {
                    for pt in ring.as_array()? {
                        let arr = pt.as_array()?;
                        all.push((arr.get(0)?.as_f64()?, arr.get(1)?.as_f64()?));
                    }
                }
            }
            Some(all)
        }
        "Point" => {
            let arr = coords.as_array()?;
            Some(vec![(arr.get(0)?.as_f64()?, arr.get(1)?.as_f64()?)])
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_feature_collection() {
        let geojson = r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "id": "1",
                    "properties": {"landcover": "forest", "area_ha": 12.5},
                    "geometry": {
                        "type": "Polygon",
                        "coordinates": [[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]
                    }
                }
            ]
        }"#;

        let (features, bbox) = parse_feature_collection(geojson).unwrap();
        assert_eq!(features.len(), 1);
        assert_eq!(features[0].id.as_deref(), Some("1"));
        assert_eq!(features[0].properties["landcover"], "forest");
        assert!(bbox.is_some());
    }

    #[test]
    fn test_extract_bbox() {
        let geojson = r#"{
            "type": "FeatureCollection",
            "features": [
                {"type": "Feature", "properties": {}, "geometry": {"type": "Point", "coordinates": [104.0, 30.5]}},
                {"type": "Feature", "properties": {}, "geometry": {"type": "Point", "coordinates": [105.0, 31.0]}}
            ]
        }"#;

        let bbox = extract_bbox(geojson).unwrap();
        assert_eq!(bbox.min_x, 104.0);
        assert_eq!(bbox.max_x, 105.0);
        assert_eq!(bbox.min_y, 30.5);
        assert_eq!(bbox.max_y, 31.0);
    }

    #[test]
    fn test_empty_feature_collection() {
        let geojson = r#"{"type": "FeatureCollection", "features": []}"#;
        let (features, bbox) = parse_feature_collection(geojson).unwrap();
        assert_eq!(features.len(), 0);
        assert!(bbox.is_none());
    }
}
