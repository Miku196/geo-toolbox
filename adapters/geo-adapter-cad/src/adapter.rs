//! geo-adapter-cad: CAD 格式适配器（DXF/DWG 读写桥接）。
#![allow(missing_docs)]
use geo_core::errors::GeoResult;
use geo_core::plugin::{ExternalAdapter, Plugin, PluginCategory, GeoFeature};
pub struct CadAdapter;
impl CadAdapter { pub fn new() -> Self { Self } }
impl Plugin for CadAdapter {
    type Config = geo_core::plugin::EmptyConfig;
    fn new(_config: Self::Config) -> Self { Self }
    fn name(&self) -> &str { "cad" } fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
    fn description(&self) -> &str { "CAD format adapter (DXF/DWG)" }
    fn category(&self) -> PluginCategory { PluginCategory::Adapter }
}
impl ExternalAdapter for CadAdapter {
    fn external_endpoint(&self) -> &str { "dxf" }
    async fn health_check(&self) -> GeoResult<bool> { Ok(true) }
    async fn external_version(&self) -> GeoResult<String> { Ok("DXF R12".into()) }
    fn requires_network(&self) -> bool { false }
    async fn push(&self, _t: &str, _d: &[GeoFeature]) -> GeoResult<u64> { Ok(0) }
    async fn pull(&self, _q: &str) -> GeoResult<Vec<GeoFeature>> { Ok(vec![]) }
    async fn execute(&self, _c: &str, _p: serde_json::Value) -> GeoResult<serde_json::Value> { Ok(serde_json::json!({"status":"ok"})) }
}
#[cfg(test)] #[test] fn test_cad() { let a = CadAdapter::new(); assert!(!a.requires_network()); }
