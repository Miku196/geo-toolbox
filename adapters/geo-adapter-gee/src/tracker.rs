//! GEE task tracker: monitors callback queue and tracks task lifecycle.
//!
//! ## Backends
//!
//! - **File-based**: reads `gee-callbacks.jsonl` (polling every 500ms)

use crate::dispatcher::GeeCallback;
use geo_core::errors::GeoResult;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Status of a tracked GEE task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum TaskStatus {
    /// Task dispatched, not yet picked up by worker.
    Pending,
    /// Task started by the gee-worker.
    Started,
    /// Task completed successfully.
    Completed,
    /// Task failed with an error.
    Failed(String),
    /// Task tracking timed out.
    TimedOut,
}

/// A tracked GEE task with its current status and result.
#[derive(Debug, Clone, Serialize)]
pub struct TrackedTask {
    /// Correlation ID matching the original task.
    pub correlation_id: String,
    /// Current status.
    pub status: TaskStatus,
    /// GEE internal task ID.
    pub gee_task_id: Option<String>,
    /// GCS URI of the result.
    pub output_uri: Option<String>,
    /// Asset type: raster | vector | table
    pub asset_type: Option<String>,
}

/// Summary of all tracked GEE tasks.
#[derive(Debug, Default, Serialize)]
pub struct TaskSummary {
    /// Total number of tasks.
    pub total: usize,
    /// Tasks still running.
    pub running: usize,
    /// Tasks completed successfully.
    pub completed: usize,
    /// Tasks that failed.
    pub failed: usize,
}

/// Tracks GEE task callbacks from a file or NATS subject.
pub struct GeeTracker {
    /// File-based callback path (Some for file mode).
    callback_path: Option<PathBuf>,

    /// Callback map from last read (cached).
    cache: HashMap<String, GeeCallback>,
}

impl GeeTracker {
    /// Create a file-based tracker that reads `gee-callbacks.jsonl`.
    pub fn new_file(queue_dir: impl Into<PathBuf>) -> Self {
        let path: PathBuf = queue_dir.into().join("gee-callbacks.jsonl");
        Self {
            callback_path: Some(path),
            cache: HashMap::new(),
        }
    }

    /// Read all callbacks from the file and return a map of correlation_id → callback.
    async fn read_callbacks_file(
        &self,
        path: &std::path::Path,
    ) -> GeoResult<HashMap<String, GeeCallback>> {
        let mut map = HashMap::new();

        if !path.exists() {
            return Ok(map);
        }

        let content = tokio::fs::read_to_string(path).await?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<GeeCallback>(line) {
                Ok(cb) => {
                    map.insert(cb.correlation_id.clone(), cb);
                }
                Err(e) => {
                    tracing::warn!("Failed to parse callback line: {e}");
                }
            }
        }

        Ok(map)
    }

    /// Read all callbacks from the configured source.
    pub async fn read_callbacks(&self) -> GeoResult<HashMap<String, GeeCallback>> {
        if let Some(path) = &self.callback_path {
            self.read_callbacks_file(path).await
        } else {
            // NATS mode — callback is event-driven, check cache.
            Ok(self.cache.clone())
        }
    }

    /// Wait for a specific task to complete, polling every `interval`.
    ///
    /// Returns `TimedOut` if the task doesn't complete within `timeout`.
    pub async fn wait_for_task(
        &self,
        correlation_id: &str,
        timeout: Duration,
        interval: Duration,
    ) -> GeoResult<TrackedTask> {
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            let callbacks = self.read_callbacks().await?;

            if let Some(cb) = callbacks.get(correlation_id) {
                let status = match cb.status.as_str() {
                    "started" => TaskStatus::Started,
                    "completed" => TaskStatus::Completed,
                    "failed" => TaskStatus::Failed(
                        cb.error.clone().unwrap_or_else(|| "Unknown error".into()),
                    ),
                    _ => TaskStatus::Started,
                };

                if matches!(status, TaskStatus::Completed | TaskStatus::Failed(_)) {
                    return Ok(TrackedTask {
                        correlation_id: cb.correlation_id.clone(),
                        status,
                        gee_task_id: cb.gee_task_id.clone(),
                        output_uri: cb.output_uri.clone(),
                        asset_type: cb.asset_type.clone(),
                    });
                }
            }

            if tokio::time::Instant::now() >= deadline {
                return Ok(TrackedTask {
                    correlation_id: correlation_id.into(),
                    status: TaskStatus::TimedOut,
                    gee_task_id: None,
                    output_uri: None,
                    asset_type: None,
                });
            }

            tokio::time::sleep(interval).await;
        }
    }

    /// Check if a task has any result without blocking.
    pub async fn check_task(&self, correlation_id: &str) -> GeoResult<Option<TrackedTask>> {
        let callbacks = self.read_callbacks().await?;
        if let Some(cb) = callbacks.get(correlation_id) {
            let status = match cb.status.as_str() {
                "started" => TaskStatus::Started,
                "completed" => TaskStatus::Completed,
                "failed" => {
                    TaskStatus::Failed(cb.error.clone().unwrap_or_else(|| "Unknown error".into()))
                }
                _ => TaskStatus::Started,
            };
            Ok(Some(TrackedTask {
                correlation_id: cb.correlation_id.clone(),
                status,
                gee_task_id: cb.gee_task_id.clone(),
                output_uri: cb.output_uri.clone(),
                asset_type: cb.asset_type.clone(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Count tasks by status.
    pub async fn summary(&self) -> GeoResult<TaskSummary> {
        let callbacks = self.read_callbacks().await?;
        let mut summary = TaskSummary::default();

        for cb in callbacks.values() {
            summary.total += 1;
            match cb.status.as_str() {
                "started" => summary.running += 1,
                "completed" => summary.completed += 1,
                "failed" => summary.failed += 1,
                _ => {}
            }
        }

        Ok(summary)
    }
}

impl Clone for GeeTracker {
    fn clone(&self) -> Self {
        Self {
            callback_path: self.callback_path.clone(),
            cache: self.cache.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mq::FileMq;
    use std::time::Duration;

    #[tokio::test]
    async fn test_track_completed_task() {
        let dir = std::env::temp_dir().join("geo-gee-tracker-test");
        let _ = std::fs::create_dir_all(&dir);

        let mq = FileMq::new(&dir);
        let cid = "test-track-001";

        mq.publish_callback(&GeeCallback {
            correlation_id: cid.into(),
            status: "completed".into(),
            gee_task_id: Some("GEE_123".into()),
            output_uri: Some("gs://bucket/out.tif".into()),
            error: None,
            timestamp: "2025-06-07T00:00:00Z".into(),
            asset_type: Some("raster".into()),
        })
        .await
        .unwrap();

        let tracker = GeeTracker::new_file(&dir);
        let result = tracker
            .wait_for_task(cid, Duration::from_secs(5), Duration::from_millis(50))
            .await
            .unwrap();

        assert_eq!(result.status, TaskStatus::Completed);
        assert_eq!(result.output_uri, Some("gs://bucket/out.tif".into()));
        assert_eq!(result.asset_type, Some("raster".into()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_track_failed_task() {
        let dir = std::env::temp_dir().join("geo-gee-tracker-fail");
        let _ = std::fs::create_dir_all(&dir);

        let mq = FileMq::new(&dir);
        let cid = "test-track-002";

        mq.publish_callback(&GeeCallback {
            correlation_id: cid.into(),
            status: "failed".into(),
            gee_task_id: None,
            output_uri: None,
            error: Some("GEE task exceeded memory limit".into()),
            timestamp: "2025-06-07T00:00:00Z".into(),
            asset_type: None,
        })
        .await
        .unwrap();

        let tracker = GeeTracker::new_file(&dir);
        let result = tracker
            .wait_for_task(cid, Duration::from_secs(5), Duration::from_millis(50))
            .await
            .unwrap();

        assert!(matches!(result.status, TaskStatus::Failed(_)));
        if let TaskStatus::Failed(msg) = &result.status {
            assert!(msg.contains("memory"));
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_summary() {
        let dir = std::env::temp_dir().join("geo-gee-tracker-summary");
        let _ = std::fs::create_dir_all(&dir);

        let mq = FileMq::new(&dir);

        mq.publish_callback(&GeeCallback {
            correlation_id: "a".into(),
            status: "completed".into(),
            gee_task_id: None,
            output_uri: None,
            error: None,
            timestamp: "2025-06-07T00:00:00Z".into(),
            asset_type: None,
        })
        .await
        .unwrap();

        mq.publish_callback(&GeeCallback {
            correlation_id: "b".into(),
            status: "failed".into(),
            gee_task_id: None,
            output_uri: None,
            error: Some("timeout".into()),
            timestamp: "2025-06-07T00:00:00Z".into(),
            asset_type: None,
        })
        .await
        .unwrap();

        mq.publish_callback(&GeeCallback {
            correlation_id: "c".into(),
            status: "started".into(),
            gee_task_id: None,
            output_uri: None,
            error: None,
            timestamp: "2025-06-07T00:00:00Z".into(),
            asset_type: None,
        })
        .await
        .unwrap();

        let tracker = GeeTracker::new_file(&dir);
        let summary = tracker.summary().await.unwrap();

        assert_eq!(summary.total, 3);
        assert_eq!(summary.completed, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.running, 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
