//! Message queue abstraction for GEE task dispatch.
//!
//! Supports NATS (default, lightweight), file-based queue (dev/testing),
//! and Kafka (optional feature `kafka`).
//!
//! ## Architecture
//!
//! ```text
//! geo-toolbox → GeeMq::publish(task) → gee.tasks topic
//!                                        ↓
//!                                   Python gee-worker
//!                                        ↓
//! geo-toolbox ← GeeMq::subscribe() ← gee.callbacks topic
//! ```

// GeoError 仅在 gee feature 开启时使用，关闭时有 unused 警告
#[allow(unused_imports)]
use geo_core::errors::{GeoError, GeoResult};

use crate::dispatcher::{GeeCallback, GeeTask};

/// Message queue trait for GEE task dispatch.
///
/// Only includes dyn-compatible methods. Subscription is handled
/// by concrete implementations (not part of this trait).
#[async_trait::async_trait]
pub trait GeeMq: Send + Sync {
    /// Publish a GEE task to the task queue.
    async fn publish_task(&self, task: &GeeTask) -> GeoResult<()>;

    /// Publish a callback (used by the Python gee-worker to report results).
    async fn publish_callback(&self, callback: &GeeCallback) -> GeoResult<()>;
}

// ── NATS implementation ──────────────────────────────────────

/// NATS-based message queue (requires `nats` feature).
#[cfg(feature = "nats")]
pub struct NatsMq {
    client: async_nats::Client,
    task_subject: String,
    callback_subject: String,
}

#[cfg(feature = "nats")]
impl NatsMq {
    /// Connect to a NATS server.
    ///
    /// # Arguments
    /// * `url` - NATS server URL, e.g. "nats://localhost:4222"
    /// * `task_subject` - NATS subject for publishing tasks (default: "gee.tasks")
    /// * `callback_subject` - NATS subject for receiving callbacks (default: "gee.callbacks")
    pub async fn connect(
        url: &str,
        task_subject: Option<&str>,
        callback_subject: Option<&str>,
    ) -> GeoResult<Self> {
        let client = async_nats::connect(url)
            .await
            .map_err(|e| GeoError::MessageQueue(format!("NATS connect: {e}")))?;

        tracing::info!("Connected to NATS at {url}");

        Ok(Self {
            client,
            task_subject: task_subject.unwrap_or("gee.tasks").to_string(),
            callback_subject: callback_subject.unwrap_or("gee.callbacks").to_string(),
        })
    }

    /// Get the task subject name.
    pub fn task_subject(&self) -> &str {
        &self.task_subject
    }

    /// Get the callback subject name.
    pub fn callback_subject(&self) -> &str {
        &self.callback_subject
    }

    /// Request-reply: publish a task and wait for a callback on a reply subject.
    /// Uses NATS request-reply pattern for synchronous dispatch.
    pub async fn request_task(
        &self,
        task: &GeeTask,
        timeout: std::time::Duration,
    ) -> GeoResult<GeeCallback> {
        let payload = serde_json::to_vec(task)?;

        let response = tokio::time::timeout(
            timeout,
            self.client.request(self.task_subject.clone(), payload.into()),
        )
        .await
        .map_err(|_| {
            GeoError::MessageQueue(format!(
                "NATS request timeout after {}s",
                timeout.as_secs()
            ))
        })?
        .map_err(|e| GeoError::MessageQueue(format!("NATS request: {e}")))?;

        let callback: GeeCallback = serde_json::from_slice(&response.payload)?;
        Ok(callback)
    }
}

#[cfg(feature = "nats")]
#[async_trait::async_trait]
impl GeeMq for NatsMq {
    async fn publish_task(&self, task: &GeeTask) -> GeoResult<()> {
        let payload = serde_json::to_vec(task)?;

        self.client
            .publish(self.task_subject.clone(), payload.into())
            .await
            .map_err(|e| GeoError::MessageQueue(format!("NATS publish task: {e}")))?;

        tracing::debug!(
            "NATS published task {} to {}",
            task.correlation_id,
            self.task_subject
        );
        Ok(())
    }

    async fn publish_callback(&self, callback: &GeeCallback) -> GeoResult<()> {
        let payload = serde_json::to_vec(callback)?;

        self.client
            .publish(self.callback_subject.clone(), payload.into())
            .await
            .map_err(|e| GeoError::MessageQueue(format!("NATS publish callback: {e}")))?;

        Ok(())
    }
}

#[cfg(feature = "nats")]
impl NatsMq {
    /// Subscribe to callbacks and call `handler` for each one received.
    /// Runs until the future is dropped or an error occurs.
    pub async fn subscribe_callbacks(
        &self,
        handler: impl Fn(GeeCallback) + Send + Sync + 'static,
    ) -> GeoResult<()> {
        use futures::StreamExt;

        let mut subscriber = self
            .client
            .subscribe(self.callback_subject.clone())
            .await
            .map_err(|e| GeoError::MessageQueue(format!("NATS subscribe: {e}")))?;

        tracing::info!(
            "Subscribed to NATS callbacks on {}",
            self.callback_subject
        );

        while let Some(msg) = subscriber.next().await {
            match serde_json::from_slice::<GeeCallback>(&msg.payload) {
                Ok(callback) => {
                    tracing::debug!("NATS callback: {} ({})", callback.correlation_id, callback.status);
                    handler(callback);
                }
                Err(e) => {
                    tracing::warn!("Failed to parse NATS callback: {e}");
                }
            }
        }

        Ok(())
    }
}

// ── File-based queue (dev/testing fallback) ──────────────────

/// File-based message queue for development and testing.
///
/// Writes tasks/callbacks as JSON-lines to local files.
/// No external dependencies required. Always available.
pub struct FileMq {
    task_path: std::path::PathBuf,
    callback_path: std::path::PathBuf,
}

impl FileMq {
    /// Create a file-based queue in the given directory.
    pub fn new(queue_dir: impl Into<std::path::PathBuf>) -> Self {
        let dir = queue_dir.into();
        Self {
            task_path: dir.join("gee-tasks.jsonl"),
            callback_path: dir.join("gee-callbacks.jsonl"),
        }
    }

    /// Path to the task queue file (for Python gee-worker to tail).
    pub fn task_path(&self) -> &std::path::Path {
        &self.task_path
    }

    /// Path to the callback file (for tracker to read).
    pub fn callback_path(&self) -> &std::path::Path {
        &self.callback_path
    }

    async fn ensure_dir(&self, path: &std::path::Path) -> GeoResult<()> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        Ok(())
    }

    async fn append_line(&self, path: &std::path::Path, line: &str) -> GeoResult<()> {
        self.ensure_dir(path).await?;

        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;

        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl GeeMq for FileMq {
    async fn publish_task(&self, task: &GeeTask) -> GeoResult<()> {
        let line = serde_json::to_string(task)?;
        self.append_line(&self.task_path, &line).await?;
        tracing::debug!("FileMq task: {}", task.correlation_id);
        Ok(())
    }

    async fn publish_callback(&self, callback: &GeeCallback) -> GeoResult<()> {
        let line = serde_json::to_string(callback)?;
        self.append_line(&self.callback_path, &line).await?;
        Ok(())
    }
}

impl FileMq {
    /// Subscribe to callbacks: poll file for new lines every 500ms.
    pub async fn subscribe_callbacks(
        &self,
        handler: impl Fn(GeeCallback) + Send + Sync + 'static,
    ) -> GeoResult<()> {
        let mut last_size = 0u64;

        loop {
            if let Ok(meta) = tokio::fs::metadata(&self.callback_path).await {
                let current_size = meta.len();
                if current_size > last_size {
                    let content = tokio::fs::read_to_string(&self.callback_path).await?;
                    let new_content = &content[last_size as usize..];

                    for line in new_content.lines() {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }
                        if let Ok(callback) = serde_json::from_str::<GeeCallback>(line) {
                            handler(callback);
                        }
                    }
                    last_size = current_size;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
}

// ── Factory ──────────────────────────────────────────────────

/// Create the appropriate MQ implementation based on environment.
///
/// Priority:
/// 1. NATS if `GEO_NATS_URL` env var is set and `nats` feature is enabled
/// 2. File-based queue as fallback
pub async fn create_mq() -> GeoResult<Box<dyn GeeMq>> {
    #[cfg(feature = "nats")]
    {
        if let Ok(nats_url) = std::env::var("GEO_NATS_URL") {
            let mq = NatsMq::connect(&nats_url, None, None).await?;
            return Ok(Box::new(mq));
        }
    }

    // Fallback: file-based queue (always available).
    let queue_dir = std::env::var("GEO_QUEUE_DIR")
        .unwrap_or_else(|_| "./queue".to_string());
    Ok(Box::new(FileMq::new(queue_dir)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_mq_publish_and_read() {
        let dir = std::env::temp_dir().join("geo-gee-mq-test");
        let _ = std::fs::create_dir_all(&dir);

        let mq = FileMq::new(&dir);

        let task = GeeTask {
            correlation_id: "mq-test-001".into(),
            task_type: "landcover_classification".into(),
            aoi_path: "s3://test/aoi.gpkg".into(),
            year: 2025,
            output_gcs: "gs://test/out.tif".into(),
            params: Default::default(),
            dispatched_at: "2025-06-07T00:00:00Z".into(),
        };

        mq.publish_task(&task).await.unwrap();

        let content = tokio::fs::read_to_string(mq.task_path()).await.unwrap();
        assert!(content.contains("mq-test-001"));
        assert!(content.contains("landcover_classification"));

        let callback = GeeCallback {
            correlation_id: "mq-test-001".into(),
            status: "completed".into(),
            gee_task_id: Some("GEE_001".into()),
            output_uri: Some("gs://test/out.tif".into()),
            error: None,
            timestamp: "2025-06-07T00:00:00Z".into(),
            asset_type: Some("raster".into()),
        };

        mq.publish_callback(&callback).await.unwrap();

        let cb_content = tokio::fs::read_to_string(mq.callback_path()).await.unwrap();
        assert!(cb_content.contains("completed"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
