//! geo-adapter-cli: 外部 CLI 工具适配器（GDAL, DVC, shell 子进程）。
#![allow(missing_docs)]
use geo_core::errors::{GeoError, GeoResult};
use geo_core::plugin::{ExternalAdapter, GeoFeature, Plugin, PluginCategory};
pub struct CliAdapter;
impl CliAdapter {
    pub fn new() -> Self {
        Self
    }
}
impl Plugin for CliAdapter {
    type Config = geo_core::plugin::EmptyConfig;
    fn new(_config: Self::Config) -> Self {
        Self
    }
    fn name(&self) -> &str {
        "cli"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "External CLI adapter (GDAL/DVC/shell)"
    }
    fn category(&self) -> PluginCategory {
        PluginCategory::Adapter
    }
}
impl ExternalAdapter for CliAdapter {
    fn external_endpoint(&self) -> &str {
        "gdal_translate"
    }
    async fn health_check(&self) -> GeoResult<bool> {
        // No CLI backend (GDAL/DVC/shell) is wired on this adapter yet, so it is
        // genuinely unavailable. Report Ok(false) instead of a fake Ok(true).
        Ok(false)
    }
    async fn external_version(&self) -> GeoResult<String> {
        // No real external CLI can be queried for a version until the subprocess
        // wiring is implemented.
        Err(GeoError::Unimplemented(
            "gdal CliAdapter: external_version not queried — no CLI subprocess wired yet".into(),
        ))
    }
    fn requires_network(&self) -> bool {
        false
    }
    async fn push(&self, _t: &str, _d: &[GeoFeature]) -> GeoResult<u64> {
        Err(GeoError::Unimplemented(
            "gdal CliAdapter: push not implemented — no CLI subprocess wiring (use geo-adapter-postgis for DB writes)"
                .into(),
        ))
    }
    async fn pull(&self, _q: &str) -> GeoResult<Vec<GeoFeature>> {
        Err(GeoError::Unimplemented(
            "gdal CliAdapter: pull not implemented — no CLI subprocess wiring (use geo-adapter-postgis for DB reads)"
                .into(),
        ))
    }
    async fn execute(&self, _c: &str, _p: serde_json::Value) -> GeoResult<serde_json::Value> {
        // Explicit failure instead of a fake {"status":"ok"}: the GDAL/DVC/shell
        // subprocess backend is not implemented on this adapter.
        Err(GeoError::Unimplemented(
            "gdal CliAdapter: execute not implemented yet (call via CLI wiring)".into(),
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use geo_core::errors::GeoError;

    #[test]
    fn test_cli() {
        let a = CliAdapter::new();
        assert!(!a.requires_network());
    }

    #[tokio::test]
    async fn test_cli_execute_does_not_fake_success() {
        // The CLI adapter has no real backend wired: execute must fail explicitly
        // instead of returning a fake {"status":"ok"}.
        let a = CliAdapter::new();
        let err = a
            .execute("gdalinfo", serde_json::json!({"input": "x.tif"}))
            .await
            .expect_err("execute must not fake success");
        assert!(matches!(err, GeoError::Unimplemented(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn test_cli_push_pull_not_implemented() {
        let a = CliAdapter::new();
        assert!(matches!(a.push("t", &[]).await, Err(GeoError::Unimplemented(_))));
        assert!(matches!(a.pull("q").await, Err(GeoError::Unimplemented(_))));
    }

    #[tokio::test]
    async fn test_cli_health_check_reports_unavailable() {
        let a = CliAdapter::new();
        // With no CLI backend wired, health must not blindly report Ok(true).
        assert_eq!(a.health_check().await.unwrap(), false);
    }

    #[tokio::test]
    async fn test_cli_external_version_not_faked() {
        let a = CliAdapter::new();
        assert!(matches!(
            a.external_version().await,
            Err(GeoError::Unimplemented(_))
        ));
    }
}
