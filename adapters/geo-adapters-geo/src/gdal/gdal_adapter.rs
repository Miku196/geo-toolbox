//! geo-adapter-cli: 外部 CLI 工具适配器（GDAL, DVC, shell 子进程）。
#![allow(missing_docs)]
use geo_core::errors::GeoResult;
use geo_core::plugin::{ExternalAdapter, Plugin, PluginCategory, GeoFeature};
pub struct CliAdapter;
impl CliAdapter { pub fn new() -> Self { Self } }
impl Plugin for CliAdapter {
    type Config = geo_core::plugin::EmptyConfig;
    fn new(_config: Self::Config) -> Self { Self }
    fn name(&self) -> &str { "cli" } fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
    fn description(&self) -> &str { "External CLI adapter (GDAL/DVC/shell)" }
    fn category(&self) -> PluginCategory { PluginCategory::Adapter }
}
impl ExternalAdapter for CliAdapter {
    fn external_endpoint(&self) -> &str { "gdal_translate" }
    async fn health_check(&self) -> GeoResult<bool> { Ok(true) }
    async fn external_version(&self) -> GeoResult<String> { Ok("GDAL CLI".into()) }
    fn requires_network(&self) -> bool { false }
    async fn push(&self, _t: &str, _d: &[GeoFeature]) -> GeoResult<u64> { Ok(0) }
    async fn pull(&self, _q: &str) -> GeoResult<Vec<GeoFeature>> { Ok(vec![]) }
    async fn execute(&self, _c: &str, _p: serde_json::Value) -> GeoResult<serde_json::Value> { Ok(serde_json::json!({"status":"ok"})) }
}
#[cfg(test)] #[test] fn test_cli() { let a = CliAdapter::new(); assert!(!a.requires_network()); }
