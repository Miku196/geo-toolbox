//! QGIS subprocess runner — calls `qgis_process` CLI for batch processing.
//!
//! This is the **batch processing fallback** (Plan B). For interactive use,
//! prefer the gRPC/REST client (`QgisClient`).
//!
//! ```bash
//! # Example CLI equivalent:
//! qgis_process run native:buffer \
//!   --INPUT=/data/sites.gpkg \
//!   --DISTANCE=100 \
//!   --OUTPUT=/data/sites_buffered.gpkg
//! ```
//!
//! Criteria for choosing subprocess vs REST:
//!
//! | Factor            | Subprocess (this)     | REST (QgisClient)        |
//! |-------------------|-----------------------|--------------------------|
//! | Cold start        | 3-5s per call         | 0 (service stays alive)  |
//! | Throughput        | Low (1 call at a time)| High (concurrent jobs)   |
//! | Setup complexity   | None (just QGIS CLIs) | Need PyQGIS service daemon|
//! | Best for          | Infrequent batch jobs | Frequent interactive use  |

use geo_core::errors::{GeoError, GeoResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// A single QGIS processing tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QgisTool {
    /// QGIS algorithm ID (e.g., "native:buffer", "qgis:reprojectlayer").
    pub algorithm: String,
    /// Key-value parameters for the algorithm.
    pub params: Vec<(String, String)>,
}

/// Configuration for qgis_process subprocess.
#[derive(Debug, Clone)]
pub struct QgisProcessConfig {
    /// Path to the `qgis_process` executable (auto-detected on PATH).
    pub executable: PathBuf,
    /// Timeout per tool invocation.
    pub timeout: Duration,
    /// QGIS profile name (optional).
    pub profile: Option<String>,
}

impl Default for QgisProcessConfig {
    fn default() -> Self {
        Self {
            executable: PathBuf::from("qgis_process"),
            timeout: Duration::from_secs(300),
            profile: None,
        }
    }
}

/// Runner for batch QGIS processing via `qgis_process`.
pub struct BatchQgisRunner {
    config: QgisProcessConfig,
}

impl BatchQgisRunner {
    /// Create a new runner.
    pub fn new(config: QgisProcessConfig) -> Self {
        Self { config }
    }

    /// Run a single QGIS tool and return the output path.
    pub async fn run_tool(&self, tool: &QgisTool) -> GeoResult<PathBuf> {
        let mut args: Vec<String> = vec!["run".to_string()];

        if let Some(profile) = &self.config.profile {
            args.push("--profile".to_string());
            args.push(profile.clone());
        }

        args.push(tool.algorithm.clone());

        for (key, value) in &tool.params {
            args.push(format!("--{key}={value}"));
        }

        let output = tokio::time::timeout(
            self.config.timeout,
            tokio::process::Command::new(&self.config.executable)
                .args(&args)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output(),
        )
        .await
        .map_err(|_| GeoError::ExternalProcess {
            command: format!("{} {}", self.config.executable.display(), args.join(" ")),
            message: format!("Timed out after {} seconds", self.config.timeout.as_secs()),
        })??;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(GeoError::ExternalProcess {
                command: format!("{} {}", self.config.executable.display(), args.join(" ")),
                message: format!("{stdout}\n{stderr}").trim().to_string(),
            });
        }

        // Parse output to find the result path.
        // qgis_process outputs the result path in stdout as the last line, usually.
        let stdout = String::from_utf8_lossy(&output.stdout);
        let result_path = self.parse_output_path(&stdout, &tool.params);

        tracing::info!(
            "qgis_process completed: {} → {}",
            tool.algorithm,
            result_path.display()
        );

        Ok(result_path)
    }

    /// Run multiple tools in sequence (output of one feeds input of the next).
    pub async fn run_pipeline(
        &self,
        tools: &[QgisTool],
        initial_input: &Path,
    ) -> GeoResult<PathBuf> {
        geo_core::errors::validate_safe_path(&initial_input.to_string_lossy())?;
        if tools.is_empty() {
            return Err(GeoError::Validation(
                "Pipeline requires at least one tool".into(),
            ));
        }

        let mut current_input = initial_input.to_path_buf();

        for (i, tool) in tools.iter().enumerate() {
            // Replace INPUT with current_input path.
            let mut adapted_tool = tool.clone();
            adapted_tool
                .params
                .iter_mut()
                .filter(|(k, _)| k == "INPUT")
                .for_each(|(_, v)| {
                    *v = current_input.to_string_lossy().to_string();
                });

            // Make OUTPUT unique per step.
            adapted_tool
                .params
                .iter_mut()
                .filter(|(k, _)| k == "OUTPUT")
                .for_each(|(_, v)| {
                    if !v.contains("step_") {
                        let ext = current_input
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("gpkg");
                        *v = format!("step_{i:02}_output.{ext}");
                    }
                });

            current_input = self.run_tool(&adapted_tool).await?;

            tracing::info!(
                "Pipeline step {i}/{} ({}) → {}",
                tools.len(),
                adapted_tool.algorithm,
                current_input.display()
            );
        }

        Ok(current_input)
    }

    /// Convenience: buffer a layer.
    pub async fn buffer(
        &self,
        input: impl AsRef<Path>,
        distance: f64,
        output: impl AsRef<Path>,
    ) -> GeoResult<PathBuf> {
        geo_core::errors::validate_safe_path(&input.as_ref().to_string_lossy())?;
        self.run_tool(&QgisTool {
            algorithm: "native:buffer".into(),
            params: vec![
                ("INPUT".into(), input.as_ref().to_string_lossy().to_string()),
                ("DISTANCE".into(), distance.to_string()),
                (
                    "OUTPUT".into(),
                    output.as_ref().to_string_lossy().to_string(),
                ),
            ],
        })
        .await
    }

    /// Convenience: reproject a layer.
    pub async fn reproject(
        &self,
        input: impl AsRef<Path>,
        target_epsg: u16,
        output: impl AsRef<Path>,
    ) -> GeoResult<PathBuf> {
        geo_core::errors::validate_safe_path(&input.as_ref().to_string_lossy())?;
        self.run_tool(&QgisTool {
            algorithm: "native:reprojectlayer".into(),
            params: vec![
                ("INPUT".into(), input.as_ref().to_string_lossy().to_string()),
                ("TARGET_CRS".into(), format!("EPSG:{target_epsg}")),
                (
                    "OUTPUT".into(),
                    output.as_ref().to_string_lossy().to_string(),
                ),
            ],
        })
        .await
    }

    /// Convenience: clip a layer by a polygon.
    pub async fn clip(
        &self,
        input: impl AsRef<Path>,
        overlay: impl AsRef<Path>,
        output: impl AsRef<Path>,
    ) -> GeoResult<PathBuf> {
        geo_core::errors::validate_safe_path(&input.as_ref().to_string_lossy())?;
        geo_core::errors::validate_safe_path(&overlay.as_ref().to_string_lossy())?;
        self.run_tool(&QgisTool {
            algorithm: "native:clip".into(),
            params: vec![
                ("INPUT".into(), input.as_ref().to_string_lossy().to_string()),
                (
                    "OVERLAY".into(),
                    overlay.as_ref().to_string_lossy().to_string(),
                ),
                (
                    "OUTPUT".into(),
                    output.as_ref().to_string_lossy().to_string(),
                ),
            ],
        })
        .await
    }

    /// Convenience: intersect two layers.
    pub async fn intersect(
        &self,
        input: impl AsRef<Path>,
        overlay: impl AsRef<Path>,
        output: impl AsRef<Path>,
    ) -> GeoResult<PathBuf> {
        geo_core::errors::validate_safe_path(&input.as_ref().to_string_lossy())?;
        geo_core::errors::validate_safe_path(&overlay.as_ref().to_string_lossy())?;
        self.run_tool(&QgisTool {
            algorithm: "native:intersection".into(),
            params: vec![
                ("INPUT".into(), input.as_ref().to_string_lossy().to_string()),
                (
                    "OVERLAY".into(),
                    overlay.as_ref().to_string_lossy().to_string(),
                ),
                (
                    "OUTPUT".into(),
                    output.as_ref().to_string_lossy().to_string(),
                ),
            ],
        })
        .await
    }

    /// Convenience: union two layers.
    pub async fn union(
        &self,
        input: impl AsRef<Path>,
        overlay: impl AsRef<Path>,
        output: impl AsRef<Path>,
    ) -> GeoResult<PathBuf> {
        geo_core::errors::validate_safe_path(&input.as_ref().to_string_lossy())?;
        geo_core::errors::validate_safe_path(&overlay.as_ref().to_string_lossy())?;
        self.run_tool(&QgisTool {
            algorithm: "native:union".into(),
            params: vec![
                ("INPUT".into(), input.as_ref().to_string_lossy().to_string()),
                (
                    "OVERLAY".into(),
                    overlay.as_ref().to_string_lossy().to_string(),
                ),
                (
                    "OUTPUT".into(),
                    output.as_ref().to_string_lossy().to_string(),
                ),
            ],
        })
        .await
    }

    /// Convenience: zonal statistics (raster stats per polygon).
    pub async fn zonal_stats(
        &self,
        polygons: impl AsRef<Path>,
        raster: impl AsRef<Path>,
        output: impl AsRef<Path>,
        stats: &[&str], // e.g., ["mean", "sum", "count"]
    ) -> GeoResult<PathBuf> {
        geo_core::errors::validate_safe_path(&polygons.as_ref().to_string_lossy())?;
        geo_core::errors::validate_safe_path(&raster.as_ref().to_string_lossy())?;
        let stats_str = stats.join(",");
        self.run_tool(&QgisTool {
            algorithm: "native:zonalstatisticsfb".into(),
            params: vec![
                (
                    "INPUT".into(),
                    polygons.as_ref().to_string_lossy().to_string(),
                ),
                (
                    "INPUT_RASTER".into(),
                    raster.as_ref().to_string_lossy().to_string(),
                ),
                ("RASTER_BAND".into(), "1".into()),
                ("COLUMN_PREFIX".into(), "zs_".into()),
                ("STATISTICS".into(), stats_str),
                (
                    "OUTPUT".into(),
                    output.as_ref().to_string_lossy().to_string(),
                ),
            ],
        })
        .await
    }

    /// List available QGIS processing algorithms.
    pub async fn list_algorithms(&self) -> GeoResult<Vec<String>> {
        let output = tokio::process::Command::new(&self.config.executable)
            .args(["list"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let algorithms: Vec<String> = stdout
            .lines()
            .filter(|l| l.starts_with("native:") || l.starts_with("qgis:"))
            .map(|l| l.trim().to_string())
            .collect();

        Ok(algorithms)
    }

    // ─── Private helpers ───

    /// Extract the output file path from qgis_process stdout or params.
    fn parse_output_path(&self, stdout: &str, params: &[(String, String)]) -> PathBuf {
        // Try to find OUTPUT=... in stdout first.
        for line in stdout.lines() {
            if line.contains("OUTPUT:") {
                let path = line.split("OUTPUT:").nth(1).unwrap_or("").trim();
                if !path.is_empty() {
                    return PathBuf::from(path);
                }
            }
        }

        // Fall back to the OUTPUT param value.
        for (key, value) in params {
            if key == "OUTPUT" {
                return PathBuf::from(value);
            }
        }

        // Last resort.
        PathBuf::from("output.gpkg")
    }
}

/// Status of a single queued job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatus {
    /// Unique job ID.
    pub id: String,
    /// Job description.
    pub description: String,
    /// Current state.
    pub state: JobState,
    /// Output file path (if completed).
    pub output_path: Option<String>,
    /// Error message (if failed).
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JobState {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Progress callback: (job_name, progress_pct, completed, total).
pub type ProgressCallback = Box<dyn Fn(String, f64, usize, usize) + Send>;

/// A batch job queue for QGIS processing tools.
/// Jobs execute sequentially; progress is tracked via `progress()`.
pub struct JobQueue {
    runner: BatchQgisRunner,
    jobs: std::collections::VecDeque<(QgisTool, String)>,
    results: Vec<JobStatus>,
    next_id: u64,
    progress_callback: Option<ProgressCallback>,
}

impl JobQueue {
    pub fn new(config: QgisProcessConfig) -> Self {
        Self {
            runner: BatchQgisRunner::new(config),
            jobs: std::collections::VecDeque::new(),
            results: Vec::new(),
            next_id: 1,
            progress_callback: None,
        }
    }

    /// Set a progress callback that fires after each job completes.
    /// Signature: (job_description, progress_pct, completed, total).
    pub fn set_progress_callback(&mut self, cb: ProgressCallback) {
        self.progress_callback = Some(cb);
    }

    pub fn submit(&mut self, tool: QgisTool, description: impl Into<String>) -> String {
        let id = format!("job_{}", self.next_id);
        self.next_id += 1;
        let desc = description.into();
        self.results.push(JobStatus {
            id: id.clone(),
            description: desc.clone(),
            state: JobState::Pending,
            output_path: None,
            error: None,
        });
        self.jobs.push_back((tool, desc));
        id
    }

    pub fn pending(&self) -> usize {
        self.jobs.len()
    }

    pub fn total(&self) -> usize {
        self.results.len()
    }

    pub fn completed(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.state == JobState::Completed)
            .count()
    }

    pub fn progress(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            return 0.0;
        }
        self.completed() as f64 / total as f64
    }

    pub fn progress_pct(&self) -> String {
        format!("{:.0}%", self.progress() * 100.0)
    }

    pub fn status_all(&self) -> &[JobStatus] {
        &self.results
    }

    pub async fn run_all(&mut self) -> Vec<JobStatus> {
        let total = self.total();
        while let Some((tool, desc)) = self.jobs.pop_front() {
            let job_desc = desc.clone();
            if let Some(i) = self
                .results
                .iter()
                .position(|r| r.state == JobState::Pending)
            {
                self.results[i].state = JobState::Running;
            }
            match self.runner.run_tool(&tool).await {
                Ok(output) => {
                    if let Some(i) = self
                        .results
                        .iter()
                        .position(|r| r.state == JobState::Running)
                    {
                        self.results[i].state = JobState::Completed;
                        self.results[i].output_path = Some(output.to_string_lossy().to_string());
                    }
                }
                Err(e) => {
                    if let Some(i) = self
                        .results
                        .iter()
                        .position(|r| r.state == JobState::Running)
                    {
                        self.results[i].state = JobState::Failed;
                        self.results[i].error = Some(e.to_string());
                    }
                }
            }
            // Fire progress callback after each job
            if let Some(cb) = &self.progress_callback {
                let completed = self.completed();
                let failed = self
                    .results
                    .iter()
                    .filter(|r| r.state == JobState::Failed)
                    .count();
                let pct = (completed + failed) as f64 / total as f64;
                cb(job_desc, pct, completed, total);
            }
        }
        self.results.clone()
    }

    pub fn clear_pending(&mut self) {
        self.jobs.clear();
        self.results.retain(|r| r.state != JobState::Pending);
    }

    pub fn reset(&mut self) {
        self.jobs.clear();
        self.results.clear();
        self.next_id = 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: skip if qgis_process is not installed.
    #[allow(dead_code)]
    fn has_qgis_process() -> bool {
        std::process::Command::new("qgis_process")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn test_qgis_tool_serialization() {
        let tool = QgisTool {
            algorithm: "native:buffer".into(),
            params: vec![
                ("INPUT".into(), "/data/sites.gpkg".into()),
                ("DISTANCE".into(), "100.0".into()),
                ("OUTPUT".into(), "/data/buffered.gpkg".into()),
            ],
        };

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("native:buffer"));
        assert!(json.contains("/data/sites.gpkg"));
    }

    #[test]
    fn test_default_config() {
        let config = QgisProcessConfig::default();
        assert_eq!(config.executable, PathBuf::from("qgis_process"));
        assert_eq!(config.timeout, Duration::from_secs(300));
        assert!(config.profile.is_none());
    }

    #[test]
    fn test_parse_output_from_params() {
        let runner = BatchQgisRunner::new(QgisProcessConfig::default());
        let params = vec![("OUTPUT".into(), "/tmp/my_result.gpkg".into())];
        let path = runner.parse_output_path("some output text", &params);
        assert_eq!(path, PathBuf::from("/tmp/my_result.gpkg"));
    }

    // ── JobQueue tests ──

    #[test]
    fn test_job_queue_submit() {
        let mut queue = JobQueue::new(QgisProcessConfig::default());
        let id = queue.submit(
            QgisTool {
                algorithm: "native:buffer".into(),
                params: vec![
                    ("INPUT".into(), "test.gpkg".into()),
                    ("DISTANCE".into(), "100".into()),
                ],
            },
            "Buffer test layer by 100m",
        );
        assert_eq!(id, "job_1");
        assert_eq!(queue.pending(), 1);
        assert_eq!(queue.total(), 1);
        assert_eq!(queue.completed(), 0);
        assert!((queue.progress() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_job_queue_status() {
        let mut queue = JobQueue::new(QgisProcessConfig::default());
        queue.submit(
            QgisTool {
                algorithm: "native:buffer".into(),
                params: vec![("INPUT".into(), "a.gpkg".into())],
            },
            "Job A",
        );
        queue.submit(
            QgisTool {
                algorithm: "native:buffer".into(),
                params: vec![("INPUT".into(), "b.gpkg".into())],
            },
            "Job B",
        );
        assert_eq!(queue.total(), 2);
        assert_eq!(queue.pending(), 2);
        let status = queue.status_all();
        assert_eq!(status.len(), 2);
        assert!(status.iter().all(|s| s.state == JobState::Pending));
    }

    #[test]
    fn test_job_queue_progress() {
        let mut queue = JobQueue::new(QgisProcessConfig::default());
        for i in 0..5 {
            queue.submit(
                QgisTool {
                    algorithm: format!("tool_{}", i),
                    params: vec![],
                },
                format!("Job {}", i),
            );
        }
        assert_eq!(queue.pending(), 5);
        assert_eq!(queue.progress_pct(), "0%");
    }

    #[test]
    fn test_job_queue_clear() {
        let mut queue = JobQueue::new(QgisProcessConfig::default());
        queue.submit(
            QgisTool {
                algorithm: "native:buffer".into(),
                params: vec![],
            },
            "Test",
        );
        assert_eq!(queue.pending(), 1);
        queue.clear_pending();
        assert_eq!(queue.pending(), 0);
        assert_eq!(queue.total(), 0);
    }

    #[test]
    fn test_job_queue_reset() {
        let mut queue = JobQueue::new(QgisProcessConfig::default());
        queue.submit(
            QgisTool {
                algorithm: "native:buffer".into(),
                params: vec![],
            },
            "Test",
        );
        queue.reset();
        assert_eq!(queue.total(), 0);
        let id = queue.submit(
            QgisTool {
                algorithm: "native:buffer".into(),
                params: vec![],
            },
            "Re-test",
        );
        assert_eq!(id, "job_1");
    }

    #[test]
    fn test_progress_callback_type() {
        let mut queue = JobQueue::new(QgisProcessConfig::default());
        queue.submit(
            QgisTool {
                algorithm: "native:buffer".into(),
                params: vec![("INPUT".into(), "a.gpkg".into())],
            },
            "Job A",
        );
        queue.submit(
            QgisTool {
                algorithm: "native:buffer".into(),
                params: vec![("INPUT".into(), "b.gpkg".into())],
            },
            "Job B",
        );
        queue.submit(
            QgisTool {
                algorithm: "native:buffer".into(),
                params: vec![("INPUT".into(), "c.gpkg".into())],
            },
            "Job C",
        );

        let _callback: ProgressCallback = Box::new(|_desc, _pct, completed, total| {
            assert!(completed <= total);
            assert!(!_desc.is_empty());
        });
        assert_eq!(queue.total(), 3);
        assert_eq!(queue.pending(), 3);
    }

    #[test]
    fn test_job_queue_set_callback() {
        let mut queue = JobQueue::new(QgisProcessConfig::default());
        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();
        queue.set_progress_callback(Box::new(move |_desc, _pct, _completed, _total| {
            called_clone.store(true, std::sync::atomic::Ordering::Relaxed);
        }));
        assert!(queue.progress_callback.is_some());
    }

    #[test]
    fn test_callback_type_is_callable() {
        // Verify ProgressCallback type alias compiles correctly
        let _cb: ProgressCallback =
            Box::new(|_desc: String, _pct: f64, _completed: usize, _total: usize| {});
    }
}
