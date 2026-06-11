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
            message: format!(
                "Timed out after {} seconds",
                self.config.timeout.as_secs()
            ),
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
            return Err(GeoError::Validation("Pipeline requires at least one tool".into()));
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
                ("OUTPUT".into(), output.as_ref().to_string_lossy().to_string()),
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
                ("OUTPUT".into(), output.as_ref().to_string_lossy().to_string()),
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
                ("OVERLAY".into(), overlay.as_ref().to_string_lossy().to_string()),
                ("OUTPUT".into(), output.as_ref().to_string_lossy().to_string()),
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
                ("OVERLAY".into(), overlay.as_ref().to_string_lossy().to_string()),
                ("OUTPUT".into(), output.as_ref().to_string_lossy().to_string()),
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
                ("OVERLAY".into(), overlay.as_ref().to_string_lossy().to_string()),
                ("OUTPUT".into(), output.as_ref().to_string_lossy().to_string()),
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
                ("INPUT".into(), polygons.as_ref().to_string_lossy().to_string()),
                ("INPUT_RASTER".into(), raster.as_ref().to_string_lossy().to_string()),
                ("RASTER_BAND".into(), "1".into()),
                ("COLUMN_PREFIX".into(), "zs_".into()),
                ("STATISTICS".into(), stats_str),
                ("OUTPUT".into(), output.as_ref().to_string_lossy().to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: skip if qgis_process is not installed.
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
}
