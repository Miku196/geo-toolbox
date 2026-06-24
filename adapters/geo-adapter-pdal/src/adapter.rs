use geo_core::errors::GeoResult;
use geo_core::plugin::{Plugin, PluginCategory};
use std::io;
use std::process::Command;
use tracing::debug;

use crate::pipeline::{LasHeader, LasStats};

/// PdalAdapter provides LiDAR point cloud processing via PDAL CLI.
pub struct PdalAdapter {
    name: String,
    version: String,
    description: String,
    pdal_bin: String,
}

impl PdalAdapter {
    pub fn new() -> Self {
        Self {
            name: "pdal".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            description: "PDAL point cloud adapter — LiDAR LAS/LAZ via subprocess".into(),
            pdal_bin: std::env::var("PDAL_BIN").unwrap_or_else(|_| "pdal".into()),
        }
    }

    pub fn with_bin(pdal_bin: impl Into<String>) -> Self {
        Self {
            pdal_bin: pdal_bin.into(),
            ..Self::new()
        }
    }

    pub fn is_available(&self) -> bool {
        Command::new(&self.pdal_bin)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn las_info(&self, input_path: &str) -> GeoResult<LasHeader> {
        debug!("pdal info {}", input_path);
        let output = self.exec(&["info", input_path])?;
        let info: LasHeader = serde_json::from_str(&output)
            .map_err(|e| geo_core::GeoError::Validation(format!("pdal info parse: {e}")))?;
        Ok(info)
    }

    pub fn las_stats(&self, input_path: &str) -> GeoResult<LasStats> {
        debug!("pdal stats {}", input_path);
        let pipeline = serde_json::json!({
            "pipeline": [
                { "type": "readers.las", "filename": input_path },
                { "type": "filters.stats" }
            ]
        });
        let output = self.exec_pipeline(&pipeline)?;
        let stats: LasStats = serde_json::from_str(&output)
            .map_err(|e| geo_core::GeoError::Validation(format!("pdal stats parse: {e}")))?;
        Ok(stats)
    }

    pub fn las_to_geojson(&self, input_path: &str) -> GeoResult<String> {
        debug!("pdal translate las→geojson {}", input_path);
        let temp_file = tempfile::NamedTempFile::new().map_err(geo_core::GeoError::Io)?;
        let pipeline = serde_json::json!({
            "pipeline": [
                { "type": "readers.las", "filename": input_path },
                { "type": "writers.geojson",
                  "filename": temp_file.path().to_str().unwrap() }
            ]
        });
        self.exec_pipeline(&pipeline)?;
        std::fs::read_to_string(temp_file.path()).map_err(geo_core::GeoError::Io)
    }

    pub fn exec_pipeline(&self, pipeline_json: &serde_json::Value) -> GeoResult<String> {
        let pipeline_str = serde_json::to_string(pipeline_json)
            .map_err(|e| geo_core::GeoError::Validation(format!("pipeline serialize: {e}")))?;
        let temp_file = tempfile::NamedTempFile::new().map_err(geo_core::GeoError::Io)?;
        std::fs::write(temp_file.path(), &pipeline_str).map_err(geo_core::GeoError::Io)?;
        self.exec(&["pipeline", temp_file.path().to_str().unwrap()])
    }

    pub fn merge(&self, inputs: &[&str], output: &str) -> GeoResult<String> {
        let pipeline = serde_json::json!({
            "pipeline": inputs.iter().map(|f| {
                serde_json::json!({ "type": "readers.las", "filename": f })
            }).chain(std::iter::once(serde_json::json!({
                "type": "writers.las", "filename": output
            }))).collect::<Vec<_>>()
        });
        let out = self.exec_pipeline(&pipeline)?;
        Ok(format!("Merged {} files → {output}: {out}", inputs.len()))
    }

    pub fn translate(&self, input: &str, output: &str) -> GeoResult<String> {
        let pipeline = serde_json::json!({
            "pipeline": [
                { "type": "readers.las", "filename": input },
                { "type": "writers.las", "filename": output }
            ]
        });
        self.exec_pipeline(&pipeline)?;
        Ok(format!("Translated {input} → {output}"))
    }

    fn exec(&self, args: &[&str]) -> GeoResult<String> {
        let output = Command::new(&self.pdal_bin)
            .args(args)
            .output()
            .map_err(|e| {
                geo_core::GeoError::Io(io::Error::new(
                    e.kind(),
                    format!("pdal command failed (is PDAL installed?): {e}"),
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(geo_core::GeoError::ExternalProcess {
                command: format!("pdal {}", args.join(" ")),
                message: stderr.into_owned(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

impl Default for PdalAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for PdalAdapter {
    type Config = geo_core::plugin::EmptyConfig;

    fn new(_: Self::Config) -> Self {
        Self::new()
    }

    fn name(&self) -> &str {
        &self.name
    }
    fn version(&self) -> &str {
        &self.version
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn category(&self) -> PluginCategory {
        PluginCategory::Adapter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_construction() {
        let adapter = PdalAdapter::new();
        assert_eq!(adapter.name(), "pdal");
        assert!(adapter.description().contains("PDAL"));
    }

    #[test]
    fn test_adapter_default() {
        let adapter = PdalAdapter::default();
        assert_eq!(adapter.version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_custom_bin() {
        let adapter = PdalAdapter::with_bin("/usr/local/bin/pdal");
        assert_eq!(adapter.name(), "pdal");
    }

    #[test]
    fn test_is_available_graceful() {
        let adapter = PdalAdapter::new();
        let _ = adapter.is_available();
    }
}
