//! STAC API 客户端。

use geo_core::errors::{GeoError, GeoResult};
use serde::{Deserialize, Serialize};

/// STAC Item（单景影像条目）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StacItem {
    /// STAC Item ID。
    pub id: String,
    /// 采集时间。
    pub datetime: Option<String>,
    /// 云覆盖百分比。
    #[serde(rename = "eo:cloud_cover")]
    pub cloud_cover: Option<f64>,
    /// 几何 (GeoJSON)。
    pub geometry: Option<serde_json::Value>,
    /// 边界框 [min_lon, min_lat, max_lon, max_lat]。
    pub bbox: Option<Vec<f64>>,
    /// 资源链接（COG 地址等）。
    pub assets: Option<serde_json::Value>,
    /// 所属集合。
    pub collection: Option<String>,
}

/// STAC Collection（数据集）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StacCollection {
    pub id: String,
    pub title: Option<String>,
    pub description: String,
}

/// STAC API 搜索响应。
#[derive(Debug, Deserialize)]
struct StacSearchResponse {
    features: Vec<serde_json::Value>,
}

/// STAC API 客户端。
pub struct StacClient {
    pub base_url: String,
    client: reqwest::Client,
}

impl StacClient {
    /// 创建客户端，指向 STAC API 根地址。
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// 按 AOI + 时间 + 云量搜索影像。
    /// STAC 空间-时间搜索。参数对应 OGC STAC API 规范。
    #[allow(clippy::too_many_arguments)]
    pub async fn search(
        &self,
        collection: &str,
        min_lon: f64, min_lat: f64,
        max_lon: f64, max_lat: f64,
        date_from: &str, date_to: &str,
        limit: u32,
    ) -> GeoResult<Vec<StacItem>> {
        let url = format!("{}/search", self.base_url);
        let body = serde_json::json!({
            "collections": [collection],
            "bbox": [min_lon, min_lat, max_lon, max_lat],
            "datetime": format!("{date_from}/{date_to}"),
            "limit": limit,
            "query": {
                "eo:cloud_cover": { "lte": 20 }
            }
        });

        let resp = self.client.post(&url)
            .json(&body)
            .send().await
            .map_err(|e| GeoError::ExternalProcess {
                command: format!("STAC search {url}"),
                message: e.to_string(),
            })?;

        let search_resp: StacSearchResponse = resp.json().await
            .map_err(|e| GeoError::Other(e.to_string()))?;

        let items: Vec<StacItem> = search_resp.features.iter()
            .filter_map(|f| serde_json::from_value(f.clone()).ok())
            .collect();

        tracing::info!("STAC search: {} items for {collection}", items.len());
        Ok(items)
    }

    /// 列出可用集合。
    pub async fn list_collections(&self) -> GeoResult<Vec<StacCollection>> {
        let url = format!("{}/collections", self.base_url);
        let resp = self.client.get(&url).send().await
            .map_err(|e| GeoError::ExternalProcess {
                command: format!("STAC collections {url}"),
                message: e.to_string(),
            })?;

        let json: serde_json::Value = resp.json().await
            .map_err(|e| GeoError::Other(e.to_string()))?;

        let collections: Vec<StacCollection> = json["collections"].as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|c| serde_json::from_value(c.clone()).ok())
            .collect();

        tracing::info!("STAC: {} collections", collections.len());
        Ok(collections)
    }

    /// 获取单个 Item 的详情。
    pub async fn get_item(&self, collection: &str, item_id: &str) -> GeoResult<StacItem> {
        let url = format!("{}/collections/{collection}/items/{item_id}", self.base_url);
        let resp = self.client.get(&url).send().await
            .map_err(|e| GeoError::ExternalProcess {
                command: format!("STAC item {url}"),
                message: e.to_string(),
            })?;

        let item: StacItem = resp.json().await
            .map_err(|e| GeoError::Other(e.to_string()))?;

        Ok(item)
    }

    /// 健康检查。
    pub async fn health(&self) -> GeoResult<bool> {
        match self.client.get(&self.base_url).send().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stac_item_deserialize() {
        let json = r#"{
            "id": "S2A_MSIL2A_20250601",
            "datetime": "2025-06-01T00:00:00Z",
            "eo:cloud_cover": 5.2,
            "bbox": [104.0, 30.0, 105.0, 31.0],
            "collection": "sentinel-2-l2a"
        }"#;
        let item: StacItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.id, "S2A_MSIL2A_20250601");
        assert_eq!(item.cloud_cover, Some(5.2));
    }

    #[test]
    fn test_client_creation() {
        let client = StacClient::new("https://planetarycomputer.microsoft.com/api/stac/v1");
        assert!(client.base_url.contains("planetarycomputer"));
    }
}
