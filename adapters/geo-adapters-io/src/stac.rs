//! STAC (SpatioTemporal Asset Catalog) API client.

use geo_core::errors::{GeoError, GeoResult};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// STAC Item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StacItem {
    pub id: String,
    pub datetime: Option<String>,
    #[serde(rename = "eo:cloud_cover")]
    pub cloud_cover: Option<f64>,
    pub geometry: Option<serde_json::Value>,
    pub bbox: Option<Vec<f64>>,
    pub assets: Option<serde_json::Value>,
    pub collection: Option<String>,
}

/// STAC Collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StacCollection {
    pub id: String,
    pub title: Option<String>,
    pub description: String,
}

#[derive(Debug, Deserialize)]
struct StacSearchResponse {
    features: Vec<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// StacClient
// ---------------------------------------------------------------------------

pub struct StacClient {
    pub base_url: String,
    client: reqwest::Client,
}

impl StacClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn search(
        &self,
        collection: &str,
        min_lon: f64,
        min_lat: f64,
        max_lon: f64,
        max_lat: f64,
        date_from: &str,
        date_to: &str,
        limit: u32,
    ) -> GeoResult<Vec<StacItem>> {
        let url = format!("{}/search", self.base_url);
        let body = serde_json::json!({
            "collections": [collection],
            "bbox": [min_lon, min_lat, max_lon, max_lat],
            "datetime": format!("{date_from}/{date_to}"),
            "limit": limit,
            "query": { "eo:cloud_cover": { "lte": 20 } }
        });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| GeoError::ExternalProcess {
                command: format!("STAC search {url}"),
                message: e.to_string(),
            })?;

        let search_resp: StacSearchResponse = resp
            .json()
            .await
            .map_err(|e| GeoError::Other(e.to_string()))?;

        let items: Vec<StacItem> = search_resp
            .features
            .iter()
            .filter_map(|f| serde_json::from_value(f.clone()).ok())
            .collect();

        tracing::info!("STAC search: {} items for {collection}", items.len());
        Ok(items)
    }

    pub async fn list_collections(&self) -> GeoResult<Vec<StacCollection>> {
        let url = format!("{}/collections", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| GeoError::ExternalProcess {
                command: format!("STAC collections {url}"),
                message: e.to_string(),
            })?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| GeoError::Other(e.to_string()))?;

        let collections: Vec<StacCollection> = json["collections"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|c| serde_json::from_value(c.clone()).ok())
            .collect();

        tracing::info!("STAC: {} collections", collections.len());
        Ok(collections)
    }

    pub async fn get_item(&self, collection: &str, item_id: &str) -> GeoResult<StacItem> {
        let url = format!("{}/collections/{collection}/items/{item_id}", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| GeoError::ExternalProcess {
                command: format!("STAC item {url}"),
                message: e.to_string(),
            })?;

        let item: StacItem = resp
            .json()
            .await
            .map_err(|e| GeoError::Other(e.to_string()))?;

        Ok(item)
    }

    pub async fn health(&self) -> GeoResult<bool> {
        match self.client.get(&self.base_url).send().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

// ---------------------------------------------------------------------------
// StacAdapter
// ---------------------------------------------------------------------------

use geo_core::plugin::{ExternalAdapter, GeoFeature, Plugin, PluginCategory};

pub struct StacAdapter {
    client: StacClient,
}

impl StacAdapter {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: StacClient::new(base_url),
        }
    }

    pub fn client(&self) -> &StacClient {
        &self.client
    }
}

impl Plugin for StacAdapter {
    type Config = geo_core::plugin::EmptyConfig;
    fn new(_: Self::Config) -> Self {
        StacAdapter::new("http://localhost:9090")
    }
    fn name(&self) -> &str {
        "stac"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "STAC API adapter for cloud-native geospatial data discovery"
    }
    fn category(&self) -> PluginCategory {
        PluginCategory::Adapter
    }
}

impl ExternalAdapter for StacAdapter {
    fn external_endpoint(&self) -> &str {
        &self.client.base_url
    }
    async fn health_check(&self) -> GeoResult<bool> {
        self.client.health().await
    }
    async fn external_version(&self) -> GeoResult<String> {
        Ok("STAC 1.0".into())
    }
    fn requires_network(&self) -> bool {
        true
    }
    async fn push(&self, _t: &str, _d: &[GeoFeature]) -> GeoResult<u64> {
        Ok(0)
    }
    async fn pull(&self, _q: &str) -> GeoResult<Vec<GeoFeature>> {
        Ok(vec![])
    }
    async fn execute(&self, cmd: &str, params: serde_json::Value) -> GeoResult<serde_json::Value> {
        match cmd {
            "search" => {
                let items = self
                    .client
                    .search(
                        params["collection"].as_str().unwrap_or("sentinel-2-l2a"),
                        params["min_lon"].as_f64().unwrap_or(0.0),
                        params["min_lat"].as_f64().unwrap_or(0.0),
                        params["max_lon"].as_f64().unwrap_or(0.0),
                        params["max_lat"].as_f64().unwrap_or(0.0),
                        params["date_from"].as_str().unwrap_or("2025-01-01"),
                        params["date_to"].as_str().unwrap_or("2025-12-31"),
                        params["limit"].as_u64().unwrap_or(10) as u32,
                    )
                    .await?;
                Ok(serde_json::to_value(items).unwrap_or_default())
            }
            _ => Err(geo_core::GeoError::Unimplemented(format!(
                "unknown cmd: {cmd}"
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// Tool registration — STAC
// ---------------------------------------------------------------------------

pub fn register_tools(registry: &mut geo_registry::PluginRegistry) {
    geo_registry::register_plugin!(registry, "stac", "STAC API client — search satellite imagery", PluginCategory::Adapter, [
        async "stac_search" => "Search STAC catalog by bbox and date range" ; serde_json::json!({"type":"object","properties":{"collection":{"type":"string","default":"sentinel-2-l2a"},"min_lon":{"type":"number"},"min_lat":{"type":"number"},"max_lon":{"type":"number"},"max_lat":{"type":"number"},"date_from":{"type":"string"},"date_to":{"type":"string"},"limit":{"type":"integer"},"endpoint":{"type":"string"}},"required":["min_lon","min_lat","max_lon","max_lat","date_from","date_to"]}) => |args| Box::pin(async move {
        let ep = args["endpoint"].as_str().unwrap_or("https://planetarycomputer.microsoft.com/api/stac/v1");
        let client = StacClient::new(ep);
        let limit = (args["limit"].as_u64().unwrap_or(10) as usize).try_into().unwrap_or(10);
        let items = client.search(args["collection"].as_str().unwrap_or("sentinel-2-l2a"),args["min_lon"].as_f64().unwrap_or(0.0),args["min_lat"].as_f64().unwrap_or(0.0),args["max_lon"].as_f64().unwrap_or(0.0),args["max_lat"].as_f64().unwrap_or(0.0),args["date_from"].as_str().unwrap_or("2025-01-01"),args["date_to"].as_str().unwrap_or("2025-12-31"),limit).await.map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        serde_json::to_value(items).map_err(geo_core::GeoError::Serde)
    })]);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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

    #[test]
    fn test_adapter_create() {
        let a = StacAdapter::new("https://planetarycomputer.microsoft.com/api/stac/v1");
        assert_eq!(a.name(), "stac");
        assert!(a.requires_network());
    }
}
