//! REST client for PyQGIS long-running service.
//!
//! Communicates with a Python-based QGIS server that keeps a QGIS instance alive.
//! Uses simple JSON-over-HTTP since gRPC (tonic) is optional-heavy.
//!
//! ## PyQGIS Service API
//!
//! The Python service exposes these endpoints:
//!
//! - `POST /process` — Submit a processing job.
//!   Body: `{ "inputs": [...], "tools": [...], "output_format": "gpkg" }`
//!   Returns: `{ "job_id": "uuid", "status": "accepted" }`
//!
//! - `GET /status/{job_id}` — Query job status.
//!   Returns: `{ "job_id": "...", "status": "running|completed|failed", "progress": 0-100 }`
//!
//! - `GET /result/{job_id}` — Download result.
//!   Returns: file stream or `{ "output_path": "/path/to/result.gpkg" }`
//!
//! - `GET /health` — Health check.

use geo_core::errors::{GeoError, GeoResult};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// A QGIS processing job submitted to the PyQGIS service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QgisJob {
    /// QGIS model file path (on the server).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// List of QGIS processing tools to run in sequence.
    pub tools: Vec<QgisToolStep>,

    /// Input files referenced by the tools.
    pub inputs: Vec<QgisInput>,

    /// Output format (default: gpkg).
    #[serde(default = "default_output_format")]
    pub output_format: String,
}

fn default_output_format() -> String {
    "gpkg".to_string()
}

/// A single processing step (maps to a QGIS Processing algorithm).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QgisToolStep {
    /// Algorithm ID (e.g., "native:buffer", "qgis:reprojectlayer").
    pub algorithm: String,

    /// Algorithm parameters.
    pub params: serde_json::Value,

    /// Label for this step (for logging).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// An input file referenced by tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QgisInput {
    /// Name used to reference this input in tool params.
    pub name: String,

    /// Path to the input file (server-accessible).
    pub path: String,

    /// Optional CRS (e.g., "EPSG:4326").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crs: Option<String>,
}

/// Response from submitting a job.
#[derive(Debug, Clone, Deserialize)]
pub struct JobSubmitResponse {
    /// Assigned job ID.
    pub job_id: String,
    /// Status: "accepted" or "rejected".
    pub status: String,
    /// Optional message (e.g., rejection reason).
    #[serde(default)]
    pub message: Option<String>,
}

/// Response from job status query.
#[derive(Debug, Clone, Deserialize)]
pub struct JobStatusResponse {
    /// Job ID.
    pub job_id: String,
    /// Status: "pending" | "running" | "completed" | "failed".
    pub status: String,
    /// Progress percentage (0-100).
    #[serde(default)]
    pub progress: f64,
    /// Output file path on the server (if completed).
    #[serde(default)]
    pub output_path: Option<String>,
    /// Error message (if failed).
    #[serde(default)]
    pub error: Option<String>,
}

/// REST client for PyQGIS service.
pub struct QgisClient {
    base_url: String,
    timeout: Duration,
}

impl QgisClient {
    /// Create a new client pointing to the PyQGIS service.
    ///
    /// # Arguments
    /// * `base_url` - e.g., "http://localhost:9100"
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            timeout: Duration::from_secs(300),
        }
    }

    /// Set the request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Check if the PyQGIS service is alive.
    pub async fn health_check(&self) -> GeoResult<bool> {
        let url = format!("{}/health", self.base_url);
        match Self::http_get(&url).await {
            Ok(body) => {
                tracing::info!("PyQGIS service healthy: {body}");
                Ok(true)
            }
            Err(e) => {
                tracing::warn!("PyQGIS service unreachable: {e}");
                Ok(false)
            }
        }
    }

    /// Submit a processing job and return the job_id.
    pub async fn submit(&self, job: &QgisJob) -> GeoResult<String> {
        let url = format!("{}/process", self.base_url);
        let body = serde_json::to_vec(job)?;

        let response = Self::http_post_json(&url, &body).await?;
        let result: JobSubmitResponse = serde_json::from_str(&response).map_err(GeoError::Serde)?;

        tracing::info!(
            "QGIS job submitted: {} (status: {})",
            result.job_id,
            result.status
        );
        Ok(result.job_id)
    }

    /// Poll job status until completion or failure.
    pub async fn wait_for_job(
        &self,
        job_id: &str,
        poll_interval: Duration,
    ) -> GeoResult<JobStatusResponse> {
        loop {
            let status = self.job_status(job_id).await?;

            match status.status.as_str() {
                "completed" => {
                    tracing::info!("QGIS job {job_id} completed");
                    return Ok(status);
                }
                "failed" => {
                    let err = status
                        .error
                        .clone()
                        .unwrap_or_else(|| "Unknown error".into());
                    tracing::error!("QGIS job {job_id} failed: {err}");
                    return Err(GeoError::ExternalProcess {
                        command: format!("qgis job {job_id}"),
                        message: err,
                    });
                }
                _ => {
                    tracing::debug!(
                        "QGIS job {job_id}: {} ({:.0}%)",
                        status.status,
                        status.progress
                    );
                    tokio::time::sleep(poll_interval).await;
                }
            }
        }
    }

    /// Query the status of a single job.
    pub async fn job_status(&self, job_id: &str) -> GeoResult<JobStatusResponse> {
        let url = format!("{}/status/{job_id}", self.base_url);
        let response = Self::http_get(&url).await?;
        let status: JobStatusResponse = serde_json::from_str(&response).map_err(GeoError::Serde)?;

        Ok(status)
    }

    /// Get the result output path for a completed job.
    pub async fn get_result(&self, job_id: &str) -> GeoResult<String> {
        let url = format!("{}/result/{job_id}", self.base_url);
        let response = Self::http_get(&url).await?;

        // Try to parse as JSON first.
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response) {
            if let Some(path) = json["output_path"].as_str() {
                return Ok(path.to_string());
            }
        }

        // If not JSON, treat the body as the path.
        Ok(response.trim().to_string())
    }

    /// Convenience: submit a single QGIS tool with inputs.
    pub async fn run_tool(
        &self,
        algorithm: &str,
        params: serde_json::Value,
        inputs: Vec<QgisInput>,
    ) -> GeoResult<String> {
        let job = QgisJob {
            model: None,
            tools: vec![QgisToolStep {
                algorithm: algorithm.into(),
                params,
                label: Some(algorithm.into()),
            }],
            inputs,
            output_format: "gpkg".into(),
        };

        let job_id = self.submit(&job).await?;
        let result = self.wait_for_job(&job_id, Duration::from_secs(2)).await?;

        result
            .output_path
            .ok_or_else(|| GeoError::Other("Job completed but no output path returned".into()))
    }

    /// Convenience: buffer a layer.
    pub async fn buffer(
        &self,
        input_path: &str,
        distance: f64,
        output_name: Option<&str>,
    ) -> GeoResult<String> {
        self.run_tool(
            "native:buffer",
            serde_json::json!({
                "INPUT": "input_layer",
                "DISTANCE": distance,
                "OUTPUT": format!("{}_buffered.gpkg", output_name.unwrap_or("output"))
            }),
            vec![QgisInput {
                name: "input_layer".into(),
                path: input_path.into(),
                crs: None,
            }],
        )
        .await
    }

    /// Convenience: reproject a layer.
    pub async fn reproject(&self, input_path: &str, target_epsg: u16) -> GeoResult<String> {
        self.run_tool(
            "native:reprojectlayer",
            serde_json::json!({
                "INPUT": "input_layer",
                "TARGET_CRS": format!("EPSG:{target_epsg}"),
                "OUTPUT": format!("reprojected_{target_epsg}.gpkg")
            }),
            vec![QgisInput {
                name: "input_layer".into(),
                path: input_path.into(),
                crs: None,
            }],
        )
        .await
    }

    // ─── Internal HTTP helpers ───

    async fn http_get(url: &str) -> GeoResult<String> {
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| GeoError::Other(format!("reqwest: {e}")))?;

        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| GeoError::ExternalProcess {
                command: format!("GET {url}"),
                message: e.to_string(),
            })?;

        let body = resp
            .text()
            .await
            .map_err(|e| GeoError::Other(format!("read response: {e}")))?;

        Ok(body)
    }

    async fn http_post_json(url: &str, body: &[u8]) -> GeoResult<String> {
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| GeoError::Other(format!("reqwest: {e}")))?;

        let resp = client
            .post(url)
            .header("Content-Type", "application/json")
            .body(body.to_vec())
            .send()
            .await
            .map_err(|e| GeoError::ExternalProcess {
                command: format!("POST {url}"),
                message: e.to_string(),
            })?;

        let resp_body = resp
            .text()
            .await
            .map_err(|e| GeoError::Other(format!("read response: {e}")))?;

        Ok(resp_body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qgis_job_serialization() {
        let job = QgisJob {
            model: None,
            tools: vec![QgisToolStep {
                algorithm: "native:buffer".into(),
                params: serde_json::json!({"INPUT": "layer1", "DISTANCE": 100.0}),
                label: Some("Buffer 100m".into()),
            }],
            inputs: vec![QgisInput {
                name: "layer1".into(),
                path: "/data/sites.gpkg".into(),
                crs: Some("EPSG:4326".into()),
            }],
            output_format: "gpkg".into(),
        };

        let json = serde_json::to_string(&job).unwrap();
        assert!(json.contains("native:buffer"));
        assert!(json.contains("Buffer 100m"));
        assert!(json.contains("/data/sites.gpkg"));
    }

    #[test]
    fn test_job_status_deserialize() {
        let json = r#"{"job_id":"abc","status":"running","progress":45.5}"#;
        let status: JobStatusResponse = serde_json::from_str(json).unwrap();
        assert_eq!(status.job_id, "abc");
        assert_eq!(status.status, "running");
        assert!((status.progress - 45.5).abs() < 0.01);
    }
}
