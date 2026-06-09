//! geo-adapter-mcp: MCP 协议适配器 — AI Agent JSON-RPC 入口。
#![allow(missing_docs)]
use geo_core::errors::GeoResult;
use geo_core::plugin::{ExternalAdapter, Plugin, PluginCategory, GeoFeature};
pub struct McpAdapter { pub port: u16 }
impl McpAdapter { pub fn new(port: u16) -> Self { Self { port } } }
impl Plugin for McpAdapter {
    fn name(&self) -> &str { "mcp" } fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
    fn description(&self) -> &str { "MCP protocol adapter for AI Agent integration" }
    fn category(&self) -> PluginCategory { PluginCategory::Adapter }
}
impl ExternalAdapter for McpAdapter {
    fn external_endpoint(&self) -> &str { "stdio/stdout" }
    async fn health_check(&self) -> GeoResult<bool> { Ok(true) }
    async fn external_version(&self) -> GeoResult<String> { Ok("MCP 2024-11-05".into()) }
    fn requires_network(&self) -> bool { false }
    async fn push(&self, _t: &str, _d: &[GeoFeature]) -> GeoResult<u64> { Ok(0) }
    async fn pull(&self, _q: &str) -> GeoResult<Vec<GeoFeature>> { Ok(vec![]) }
    async fn execute(&self, _c: &str, _p: serde_json::Value) -> GeoResult<serde_json::Value> { Ok(serde_json::json!({"port": self.port})) }
}
#[cfg(test)] #[test] fn test_mcp() { let a = McpAdapter::new(9378); assert_eq!(a.name(), "mcp"); assert!(!a.requires_network()); }
