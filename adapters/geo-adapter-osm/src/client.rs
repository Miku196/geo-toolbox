//! OpenStreetMap Overpass API 客户端。
//!
//! 按 AOI bbox 拉取 OSM 要素（路网、建筑、POI、土地利用），
//! 返回 GeoJSON FeatureCollection。

use geo_core::errors::{GeoError, GeoResult};
use serde::Deserialize;

/// Overpass API 响应。
#[derive(Debug, Deserialize)]
struct OverpassResponse {
    elements: Vec<serde_json::Value>,
}

/// OSM 要素类型。
#[derive(Debug, Clone)]
pub enum OsmFeature {
    Building,
    Highway,
    Waterway,
    Landuse,
    Poi,
}

/// OSM 客户端。
pub struct OsmClient {
    base_url: String,
    client: reqwest::Client,
}

impl Default for OsmClient {
    fn default() -> Self {
        Self::new()
    }
}

impl OsmClient {
    pub fn new() -> Self {
        Self {
            base_url: "https://overpass-api.de/api/interpreter".into(),
            client: reqwest::Client::new(),
        }
    }

    /// 按 bbox + 要素类型查询，返回 GeoJSON。
    pub async fn query_bbox(
        &self,
        min_lon: f64,
        min_lat: f64,
        max_lon: f64,
        max_lat: f64,
        feature: OsmFeature,
    ) -> GeoResult<Vec<serde_json::Value>> {
        let osm_tag = match feature {
            OsmFeature::Building => "building",
            OsmFeature::Highway => "highway",
            OsmFeature::Waterway => "waterway",
            OsmFeature::Landuse => "landuse",
            OsmFeature::Poi => "amenity",
        };

        let query = format!(
            "[out:json];(node[\"{osm_tag}\"]({min_lat},{min_lon},{max_lat},{max_lon});way[\"{osm_tag}\"]({min_lat},{min_lon},{max_lat},{max_lon});relation[\"{osm_tag}\"]({min_lat},{min_lon},{max_lat},{max_lon}););out geom;"
        );

        let resp = self
            .client
            .post(&self.base_url)
            .body(query)
            .send()
            .await
            .map_err(|e| GeoError::ExternalProcess {
                command: "OSM Overpass query".into(),
                message: e.to_string(),
            })?;

        let body = resp
            .text()
            .await
            .map_err(|e| GeoError::Other(e.to_string()))?;

        let result: OverpassResponse =
            serde_json::from_str(&body).map_err(|e| GeoError::Other(e.to_string()))?;

        tracing::info!("OSM: {} elements", result.elements.len());
        Ok(result.elements)
    }

    /// 转换为 GeoJSON FeatureCollection。
    pub fn to_geojson(elements: &[serde_json::Value]) -> serde_json::Value {
        let features: Vec<serde_json::Value> = elements
            .iter()
            .map(|e| {
                let tags = e.get("tags").cloned().unwrap_or(serde_json::json!({}));
                let geom = if let Some(geo) = e.get("geometry") {
                    geo.clone()
                } else if let (Some(lat), Some(lon)) = (e["lat"].as_f64(), e["lon"].as_f64()) {
                    serde_json::json!({"type":"Point","coordinates":[lon,lat]})
                } else {
                    serde_json::json!(null)
                };
                serde_json::json!({
                    "type": "Feature",
                    "properties": tags,
                    "geometry": geom,
                })
            })
            .collect();

        serde_json::json!({
            "type": "FeatureCollection",
            "features": features,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_new() {
        let c = OsmClient::new();
        assert!(c.base_url.contains("overpass"));
    }

    #[test]
    fn test_to_geojson() {
        let el = serde_json::json!({"type":"node","id":1,"lat":30.57,"lon":104.06,"tags":{"name":"test"}});
        let fc = OsmClient::to_geojson(&[el]);
        assert_eq!(fc["type"], "FeatureCollection");
        assert_eq!(fc["features"].as_array().unwrap().len(), 1);
    }
}
