//! GEE task dispatcher: high-level API on top of the message queue abstraction.
//!
//! Does NOT call GEE API directly. Instead:
//! 1. Serializes tasks as JSON
//! 2. Publishes to message queue (NATS or file)
//! 3. Python gee-worker picks up and executes
//! 4. Worker publishes callbacks that the tracker reads

use crate::mq::GeeMq;
use geo_core::errors::GeoResult;
use serde::{Deserialize, Serialize};

/// A GEE task ready for dispatch to the Python gee-worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeeTask {
    /// Unique correlation ID for tracking.
    pub correlation_id: String,

    /// Task type: landcover_classification | ndvi_timeseries | change_detection | custom
    pub task_type: String,

    /// AOI path on S3 / MinIO (e.g., `s3://geo-data/vector/sites.gpkg`)
    pub aoi_path: String,

    /// Target year for classification / analysis.
    pub year: u16,

    /// Output GCS URI (e.g., `gs://gee-exports/lc_2025.tif`).
    pub output_gcs: String,

    /// Optional algorithm parameters (JSON).
    #[serde(default)]
    pub params: serde_json::Value,

    /// ISO-8601 timestamp when the task was dispatched.
    #[serde(default = "default_timestamp")]
    pub dispatched_at: String,
}

fn default_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Result written by the Python gee-worker after task completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeeCallback {
    /// Matches the original task's correlation_id.
    pub correlation_id: String,

    /// Task status: started | completed | failed
    pub status: String,

    /// GEE internal task ID (for reference).
    #[serde(default)]
    pub gee_task_id: Option<String>,

    /// GCS URI of the exported result (if completed).
    #[serde(default)]
    pub output_uri: Option<String>,

    /// Error message (if failed).
    #[serde(default)]
    pub error: Option<String>,

    /// ISO-8601 timestamp.
    #[serde(default = "default_timestamp")]
    pub timestamp: String,

    /// Asset type: raster | vector | table
    #[serde(default)]
    pub asset_type: Option<String>,
}

/// High-level GEE task dispatcher.
///
/// Wraps a [`GeeMq`] implementation and provides typed dispatch methods
/// for each supported task type.
pub struct GeeDispatcher {
    mq: Box<dyn GeeMq>,
}

impl GeeDispatcher {
    /// Create a dispatcher backed by the given message queue.
    pub fn new(mq: Box<dyn GeeMq>) -> Self {
        Self { mq }
    }

    /// Dispatch a landcover classification task.
    ///
    /// Defaults to Random Forest with 50 trees at 10m resolution.
    pub async fn dispatch_classification(
        &self,
        aoi_path: &str,
        year: u16,
        output_gcs: &str,
        algorithm_params: Option<serde_json::Value>,
    ) -> GeoResult<String> {
        let task = GeeTask {
            correlation_id: uuid::Uuid::new_v4().to_string(),
            task_type: "landcover_classification".into(),
            aoi_path: aoi_path.into(),
            year,
            output_gcs: output_gcs.into(),
            params: algorithm_params.unwrap_or_else(|| serde_json::json!({
                "algorithm": "random_forest",
                "n_trees": 50,
                "scale": 10,
                "max_pixels": 1e13
            })),
            dispatched_at: default_timestamp(),
        };

        let cid = task.correlation_id.clone();
        self.mq.publish_task(&task).await?;
        tracing::info!(
            "GEE task dispatched: {cid} (landcover_classification, {year})"
        );

        Ok(cid)
    }

    /// Dispatch an NDVI time-series task.
    pub async fn dispatch_ndvi_timeseries(
        &self,
        aoi_path: &str,
        year: u16,
        output_gcs: &str,
    ) -> GeoResult<String> {
        let task = GeeTask {
            correlation_id: uuid::Uuid::new_v4().to_string(),
            task_type: "ndvi_timeseries".into(),
            aoi_path: aoi_path.into(),
            year,
            output_gcs: output_gcs.into(),
            params: serde_json::json!({
                "collection": "COPERNICUS/S2_SR_HARMONIZED",
                "band": "NDVI",
                "temporal_reducer": "median"
            }),
            dispatched_at: default_timestamp(),
        };

        let cid = task.correlation_id.clone();
        self.mq.publish_task(&task).await?;
        tracing::info!("GEE NDVI task dispatched: {cid} ({year})");
        Ok(cid)
    }

    /// Dispatch a change detection task (two-year comparison).
    pub async fn dispatch_change_detection(
        &self,
        aoi_path: &str,
        year_from: u16,
        year_to: u16,
        output_gcs: &str,
    ) -> GeoResult<String> {
        let task = GeeTask {
            correlation_id: uuid::Uuid::new_v4().to_string(),
            task_type: "change_detection".into(),
            aoi_path: aoi_path.into(),
            year: year_to,
            output_gcs: output_gcs.into(),
            params: serde_json::json!({
                "year_from": year_from,
                "year_to": year_to,
                "collection": "COPERNICUS/S2_SR_HARMONIZED",
                "algorithm": "random_forest",
                "n_trees": 50
            }),
            dispatched_at: default_timestamp(),
        };

        let cid = task.correlation_id.clone();
        self.mq.publish_task(&task).await?;
        tracing::info!(
            "GEE change detection dispatched: {cid} ({year_from}→{year_to})"
        );
        Ok(cid)
    }

    /// Dispatch a custom task with arbitrary type and params.
    pub async fn dispatch_custom(
        &self,
        task_type: &str,
        aoi_path: &str,
        year: u16,
        output_gcs: &str,
        params: serde_json::Value,
    ) -> GeoResult<String> {
        let task = GeeTask {
            correlation_id: uuid::Uuid::new_v4().to_string(),
            task_type: task_type.into(),
            aoi_path: aoi_path.into(),
            year,
            output_gcs: output_gcs.into(),
            params,
            dispatched_at: default_timestamp(),
        };

        let cid = task.correlation_id.clone();
        self.mq.publish_task(&task).await?;
        tracing::info!("GEE custom task dispatched: {cid} ({task_type}, {year})");
        Ok(cid)
    }

    /// Publish a callback (for testing or manual status updates).
    pub async fn publish_callback(&self, callback: &GeeCallback) -> GeoResult<()> {
        self.mq.publish_callback(callback).await
    }

    /// Get a reference to the underlying message queue.
    pub fn mq(&self) -> &dyn GeeMq {
        self.mq.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mq::FileMq;

    #[tokio::test]
    async fn test_dispatch_classification_via_mq() {
        let dir = std::env::temp_dir().join("geo-gee-dispatcher-mq-test");
        let _ = std::fs::create_dir_all(&dir);

        let mq = FileMq::new(&dir);
        let dispatcher = GeeDispatcher::new(Box::new(mq));

        let cid = dispatcher
            .dispatch_classification(
                "s3://geo-data/vector/sites.gpkg",
                2025,
                "gs://gee-exports/lc_2025.tif",
                None,
            )
            .await
            .unwrap();

        assert!(!cid.is_empty());

        // Verify task file was written.
        let content = tokio::fs::read_to_string(dir.join("gee-tasks.jsonl"))
            .await
            .unwrap();
        assert!(content.contains(&cid));
        assert!(content.contains("landcover_classification"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_dispatch_change_detection() {
        let dir = std::env::temp_dir().join("geo-gee-dispatch-change");
        let _ = std::fs::create_dir_all(&dir);

        let mq = FileMq::new(&dir);
        let dispatcher = GeeDispatcher::new(Box::new(mq));

        let cid = dispatcher
            .dispatch_change_detection(
                "s3://geo-data/vector/aoi.gpkg",
                2020,
                2025,
                "gs://gee-exports/change.tif",
            )
            .await
            .unwrap();

        let content = tokio::fs::read_to_string(dir.join("gee-tasks.jsonl"))
            .await
            .unwrap();
        assert!(content.contains("change_detection"));
        assert!(content.contains("2020"));
        assert!(content.contains("2025"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_gee_task_serde() {
        let task = GeeTask {
            correlation_id: "test-001".into(),
            task_type: "landcover_classification".into(),
            aoi_path: "s3://test/aoi.gpkg".into(),
            year: 2025,
            output_gcs: "gs://test/out.tif".into(),
            params: serde_json::json!({"trees": 50}),
            dispatched_at: "2025-06-07T00:00:00Z".into(),
        };

        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("test-001"));
        let back: GeeTask = serde_json::from_str(&json).unwrap();
        assert_eq!(back.correlation_id, "test-001");
        assert_eq!(back.year, 2025);
    }
}
