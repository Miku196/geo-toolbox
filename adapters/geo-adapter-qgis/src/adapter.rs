//! Unified QGIS adapter — config-driven backend selection.
//!
//! Supports two backends:
//! - **Subprocess** (`qgis_process` CLI): batch processing, no extra daemon needed
//! - **REST** (PyQGIS service): interactive use, long-running QGIS instance
//!
//! Selection: set `QGIS_BACKEND=rest` for REST mode, or defaults to subprocess.
//! Set `QGIS_REST_URL=http://host:port` to override REST endpoint (default: http://localhost:9100).
//! Set `QGIS_PROCESS_PATH=/path/to/qgis_process` to override subprocess binary.

use geo_core::errors::{GeoError, GeoResult};
use geo_core::plugin::{ExternalAdapter, Plugin, PluginCategory, GeoFeature};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::grpc_client::QgisClient;
use crate::process_runner::{BatchQgisRunner, QgisProcessConfig};

/// Backend selection for QGIS processing.
pub enum QgisBackend {
    /// Calls `qgis_process` CLI for each operation.
    Subprocess { runner: BatchQgisRunner },
    /// Talks to a PyQGIS REST service (daemon).
    Rest { client: QgisClient },
}

/// Unified QGIS adapter — same API regardless of backend.
pub struct QgisAdapter {
    backend: QgisBackend,
}

impl QgisAdapter {
    // ── Constructors ──

    /// Create adapter backed by `qgis_process` subprocess.
    pub fn new_subprocess(config: QgisProcessConfig) -> Self {
        Self {
            backend: QgisBackend::Subprocess {
                runner: BatchQgisRunner::new(config),
            },
        }
    }

    /// Create adapter backed by a PyQGIS REST service.
    pub fn new_rest(base_url: &str) -> Self {
        Self {
            backend: QgisBackend::Rest {
                client: QgisClient::new(base_url),
            },
        }
    }

    /// Auto-detect backend from environment variables.
    ///
    /// - `QGIS_BACKEND=rest` → REST mode (reads `QGIS_REST_URL`)
    /// - otherwise → subprocess mode (reads `QGIS_PROCESS_PATH`)
    pub fn from_env() -> Self {
        match std::env::var("QGIS_BACKEND").as_deref() {
            Ok("rest") | Ok("REST") => {
                let url = std::env::var("QGIS_REST_URL")
                    .unwrap_or_else(|_| "http://localhost:9100".into());
                Self::new_rest(&url)
            }
            _ => {
                let exe = std::env::var("QGIS_PROCESS_PATH")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::from("qgis_process"));
                Self::new_subprocess(QgisProcessConfig {
                    executable: exe,
                    ..Default::default()
                })
            }
        }
    }

    /// Return which backend is active.
    pub fn active_backend(&self) -> &'static str {
        match &self.backend {
            QgisBackend::Subprocess { .. } => "subprocess (qgis_process)",
            QgisBackend::Rest { .. } => "REST (PyQGIS)",
        }
    }

    // ── Processing operations ──

    /// Buffer a vector layer.
    pub async fn buffer(
        &self,
        input: &Path,
        distance: f64,
        output: &Path,
    ) -> GeoResult<PathBuf> {
        match &self.backend {
            QgisBackend::Subprocess { runner } => runner.buffer(input, distance, output).await,
            QgisBackend::Rest { client } => {
                let result = client
                    .buffer(
                        input.to_str().unwrap_or("input"),
                        distance,
                        Some(output.to_str().unwrap_or("output")),
                    )
                    .await?;
                Ok(PathBuf::from(result))
            }
        }
    }

    /// Reproject a vector layer.
    pub async fn reproject(
        &self,
        input: &Path,
        target_epsg: u16,
        output: &Path,
    ) -> GeoResult<PathBuf> {
        match &self.backend {
            QgisBackend::Subprocess { runner } => runner.reproject(input, target_epsg, output).await,
            QgisBackend::Rest { client } => {
                let result = client
                    .reproject(input.to_str().unwrap_or("input"), target_epsg)
                    .await?;
                Ok(PathBuf::from(result))
            }
        }
    }

    /// Clip a vector layer by an overlay polygon.
    pub async fn clip(
        &self,
        input: &Path,
        overlay: &Path,
        output: &Path,
    ) -> GeoResult<PathBuf> {
        match &self.backend {
            QgisBackend::Subprocess { runner } => runner.clip(input, overlay, output).await,
            QgisBackend::Rest { client } => {
                let result = client
                    .run_tool(
                        "native:clip",
                        serde_json::json!({
                            "INPUT": "input_layer",
                            "OVERLAY": "overlay_layer",
                            "OUTPUT": output.to_str().unwrap_or("clipped.gpkg"),
                        }),
                        vec![
                            crate::grpc_client::QgisInput {
                                name: "input_layer".into(),
                                path: input.to_string_lossy().to_string(),
                                crs: None,
                            },
                            crate::grpc_client::QgisInput {
                                name: "overlay_layer".into(),
                                path: overlay.to_string_lossy().to_string(),
                                crs: None,
                            },
                        ],
                    )
                    .await?;
                Ok(PathBuf::from(result))
            }
        }
    }

    /// Intersect two vector layers.
    pub async fn intersect(
        &self,
        input: &Path,
        overlay: &Path,
        output: &Path,
    ) -> GeoResult<PathBuf> {
        match &self.backend {
            QgisBackend::Subprocess { runner } => runner.intersect(input, overlay, output).await,
            QgisBackend::Rest { client } => {
                let result = client
                    .run_tool(
                        "native:intersection",
                        serde_json::json!({
                            "INPUT": "input_layer",
                            "OVERLAY": "overlay_layer",
                            "OUTPUT": output.to_str().unwrap_or("intersected.gpkg"),
                        }),
                        vec![
                            crate::grpc_client::QgisInput {
                                name: "input_layer".into(),
                                path: input.to_string_lossy().to_string(),
                                crs: None,
                            },
                            crate::grpc_client::QgisInput {
                                name: "overlay_layer".into(),
                                path: overlay.to_string_lossy().to_string(),
                                crs: None,
                            },
                        ],
                    )
                    .await?;
                Ok(PathBuf::from(result))
            }
        }
    }

    /// Union two vector layers.
    pub async fn union(
        &self,
        input: &Path,
        overlay: &Path,
        output: &Path,
    ) -> GeoResult<PathBuf> {
        match &self.backend {
            QgisBackend::Subprocess { runner } => runner.union(input, overlay, output).await,
            QgisBackend::Rest { client } => {
                let result = client
                    .run_tool(
                        "native:union",
                        serde_json::json!({
                            "INPUT": "input_layer",
                            "OVERLAY": "overlay_layer",
                            "OUTPUT": output.to_str().unwrap_or("union.gpkg"),
                        }),
                        vec![
                            crate::grpc_client::QgisInput {
                                name: "input_layer".into(),
                                path: input.to_string_lossy().to_string(),
                                crs: None,
                            },
                            crate::grpc_client::QgisInput {
                                name: "overlay_layer".into(),
                                path: overlay.to_string_lossy().to_string(),
                                crs: None,
                            },
                        ],
                    )
                    .await?;
                Ok(PathBuf::from(result))
            }
        }
    }

    /// Zonal statistics: raster stats per polygon.
    pub async fn zonal_stats(
        &self,
        polygons: &Path,
        raster: &Path,
        output: &Path,
        stats: &[&str],
    ) -> GeoResult<PathBuf> {
        match &self.backend {
            QgisBackend::Subprocess { runner } => {
                runner.zonal_stats(polygons, raster, output, stats).await
            }
            QgisBackend::Rest { client } => {
                let result = client
                    .run_tool(
                        "native:zonalstatisticsfb",
                        serde_json::json!({
                            "INPUT": "polygons",
                            "INPUT_RASTER": raster.to_str().unwrap_or("raster"),
                            "RASTER_BAND": 1,
                            "COLUMN_PREFIX": "zs_",
                            "STATISTICS": stats.join(","),
                            "OUTPUT": output.to_str().unwrap_or("zonal.gpkg"),
                        }),
                        vec![
                            crate::grpc_client::QgisInput {
                                name: "polygons".into(),
                                path: polygons.to_string_lossy().to_string(),
                                crs: None,
                            },
                            crate::grpc_client::QgisInput {
                                name: "raster".into(),
                                path: raster.to_string_lossy().to_string(),
                                crs: None,
                            },
                        ],
                    )
                    .await?;
                Ok(PathBuf::from(result))
            }
        }
    }

    /// Run a pipeline of QGIS tools sequentially.
    pub async fn run_pipeline(
        &self,
        tools: &[crate::process_runner::QgisTool],
        initial_input: &Path,
    ) -> GeoResult<PathBuf> {
        match &self.backend {
            QgisBackend::Subprocess { runner } => {
                runner.run_pipeline(tools, initial_input).await
            }
            QgisBackend::Rest { client } => {
                // REST: submit the whole pipeline as a batch job
                let steps: Vec<crate::grpc_client::QgisToolStep> = tools
                    .iter()
                    .map(|t| {
                        let params: serde_json::Value = t
                            .params
                            .iter()
                            .fold(serde_json::Map::new(), |mut m, (k, v)| {
                                m.insert(k.clone(), serde_json::Value::String(v.clone()));
                                m
                            })
                            .into();
                        crate::grpc_client::QgisToolStep {
                            algorithm: t.algorithm.clone(),
                            params,
                            label: None,
                        }
                    })
                    .collect();
                let job = crate::grpc_client::QgisJob {
                    model: None,
                    tools: steps,
                    inputs: vec![crate::grpc_client::QgisInput {
                        name: "initial_input".into(),
                        path: initial_input.to_string_lossy().to_string(),
                        crs: None,
                    }],
                    output_format: "gpkg".into(),
                };
                let job_id = client.submit(&job).await?;
                let result = client
                    .wait_for_job(&job_id, Duration::from_secs(2))
                    .await?;
                let path = result
                    .output_path
                    .ok_or_else(|| GeoError::Other("Pipeline completed but no output path".into()))?;
                Ok(PathBuf::from(path))
            }
        }
    }
}

// ── Plugin trait ──

impl Plugin for QgisAdapter {
    fn name(&self) -> &str {
        "qgis"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "QGIS processing bridge (subprocess or REST)"
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::Adapter
    }

    fn is_healthy(&self) -> bool {
        true
    }
}

// ── ExternalAdapter trait ──

impl ExternalAdapter for QgisAdapter {
    fn external_endpoint(&self) -> &str {
        // Static ref to avoid allocation issue
        match &self.backend {
            QgisBackend::Subprocess { .. } => "qgis_process",
            QgisBackend::Rest { client: _ } => {
                // No accessor for base_url on QgisClient, fallback
                "PyQGIS REST"
            }
        }
    }

    async fn health_check(&self) -> GeoResult<bool> {
        match &self.backend {
            QgisBackend::Subprocess { .. } => {
                // Check if qgis_process is available
                let ok = tokio::process::Command::new("qgis_process")
                    .arg("--version")
                    .output()
                    .await
                    .map(|o| o.status.success())
                    .unwrap_or(false);
                Ok(ok)
            }
            QgisBackend::Rest { client } => client.health_check().await,
        }
    }

    async fn external_version(&self) -> GeoResult<String> {
        match &self.backend {
            QgisBackend::Subprocess { .. } => {
                let output = tokio::process::Command::new("qgis_process")
                    .arg("--version")
                    .output()
                    .await
                    .map_err(|e| GeoError::ExternalProcess {
                        command: "qgis_process --version".into(),
                        message: e.to_string(),
                    })?;
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                Ok(version)
            }
            QgisBackend::Rest { client } => {
                // REST service doesn't expose version endpoint; fallback
                if client.health_check().await? {
                    Ok("PyQGIS (unknown version)".into())
                } else {
                    Err(GeoError::Other("PyQGIS service unreachable".into()))
                }
            }
        }
    }

    fn requires_network(&self) -> bool {
        matches!(&self.backend, QgisBackend::Rest { .. })
    }

    async fn push(&self, _table: &str, _data: &[GeoFeature]) -> GeoResult<u64> {
        Err(GeoError::Other(
            "QGIS adapter does not support push — use geo-adapter-postgis for database writes"
                .into(),
        ))
    }

    async fn pull(&self, _query: &str) -> GeoResult<Vec<GeoFeature>> {
        Err(GeoError::Other(
            "QGIS adapter does not support pull — use geo-adapter-postgis for database reads".into(),
        ))
    }

    async fn execute(
        &self,
        command: &str,
        params: serde_json::Value,
    ) -> GeoResult<serde_json::Value> {
        match command {
            "buffer" => {
                let input = Path::new(params["input"].as_str().unwrap_or("input"));
                let distance = params["distance"].as_f64().unwrap_or(0.0);
                let output = Path::new(params["output"].as_str().unwrap_or("output"));
                let result = self.buffer(input, distance, output).await?;
                Ok(serde_json::json!({"output": result.to_string_lossy()}))
            }
            "reproject" => {
                let input = Path::new(params["input"].as_str().unwrap_or("input"));
                let epsg = params["epsg"].as_u64().unwrap_or(4326) as u16;
                let output = Path::new(params["output"].as_str().unwrap_or("output"));
                let result = self.reproject(input, epsg, output).await?;
                Ok(serde_json::json!({"output": result.to_string_lossy()}))
            }
            "clip" => {
                let input = Path::new(params["input"].as_str().unwrap_or("input"));
                let overlay = Path::new(params["overlay"].as_str().unwrap_or("overlay"));
                let output = Path::new(params["output"].as_str().unwrap_or("output"));
                let result = self.clip(input, overlay, output).await?;
                Ok(serde_json::json!({"output": result.to_string_lossy()}))
            }
            "intersect" => {
                let input = Path::new(params["input"].as_str().unwrap_or("input"));
                let overlay = Path::new(params["overlay"].as_str().unwrap_or("overlay"));
                let output = Path::new(params["output"].as_str().unwrap_or("output"));
                let result = self.intersect(input, overlay, output).await?;
                Ok(serde_json::json!({"output": result.to_string_lossy()}))
            }
            _ => Err(GeoError::Other(format!(
                "Unknown QGIS command: {command}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let adapter = QgisAdapter::new_subprocess(QgisProcessConfig::default());
        assert_eq!(adapter.name(), "qgis");
        assert_eq!(adapter.category(), PluginCategory::Adapter);
    }

    #[test]
    fn test_rest_adapter_creation() {
        let adapter = QgisAdapter::new_rest("http://localhost:9100");
        assert_eq!(adapter.name(), "qgis");
        assert!(adapter.requires_network());
    }
}
