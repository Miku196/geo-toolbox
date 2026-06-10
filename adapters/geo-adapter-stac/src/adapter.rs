//! STAC 适配器 — ExternalAdapter trait 实现。

use geo_core::errors::GeoResult;
use geo_core::plugin::{ExternalAdapter, Plugin, PluginCategory, GeoFeature};
use crate::client::StacClient;

pub struct StacAdapter {
    client: StacClient,
}

impl StacAdapter {
    pub fn new(base_url: &str) -> Self {
        Self { client: StacClient::new(base_url) }
    }

    pub fn client(&self) -> &StacClient { &self.client }
}

impl Plugin for StacAdapter {
    fn name(&self) -> &str { "stac" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
    fn description(&self) -> &str { "STAC API adapter for cloud-native geospatial data discovery" }
    fn category(&self) -> PluginCategory { PluginCategory::Adapter }
}

impl ExternalAdapter for StacAdapter {
    fn external_endpoint(&self) -> &str { &self.client.base_url }
    async fn health_check(&self) -> GeoResult<bool> { self.client.health().await }
    async fn external_version(&self) -> GeoResult<String> { Ok("STAC 1.0".into()) }
    fn requires_network(&self) -> bool { true }
    async fn push(&self, _t: &str, _d: &[GeoFeature]) -> GeoResult<u64> { Ok(0) }
    async fn pull(&self, _q: &str) -> GeoResult<Vec<GeoFeature>> { Ok(vec![]) }
    async fn execute(&self, cmd: &str, params: serde_json::Value) -> GeoResult<serde_json::Value> {
        match cmd {
            "search" => {
                let items = self.client.search(
                    params["collection"].as_str().unwrap_or("sentinel-2-l2a"),
                    params["min_lon"].as_f64().unwrap_or(0.0),
                    params["min_lat"].as_f64().unwrap_or(0.0),
                    params["max_lon"].as_f64().unwrap_or(0.0),
                    params["max_lat"].as_f64().unwrap_or(0.0),
                    params["date_from"].as_str().unwrap_or("2025-01-01"),
                    params["date_to"].as_str().unwrap_or("2025-12-31"),
                    params["limit"].as_u64().unwrap_or(10) as u32,
                ).await?;
                Ok(serde_json::to_value(items).unwrap_or_default())
            }
            _ => Err(geo_core::GeoError::Unimplemented(format!("unknown cmd: {cmd}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_create() {
        let a = StacAdapter::new("https://planetarycomputer.microsoft.com/api/stac/v1");
        assert_eq!(a.name(), "stac");
        assert!(a.requires_network());
    }
}
