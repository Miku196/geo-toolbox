//! DVC (Data Version Control) integration.
//!
//! Calls the `dvc` CLI as a subprocess for versioned data management.
//! Design decision: subprocess rather than Rust DVC bindings because
//! the DVC Python ecosystem evolves quickly and a CLI wrapper is simpler
//! to maintain.
//!
//! ## Operations
//!
//! - `dvc_snapshot` — runs `dvc add` + `dvc push` to version-track a file
//! - `dvc_pull` — pulls tracked data from the remote
//! - `dvc_hash` — returns the md5 hash of a tracked file (for audit trails)

use geo_core::errors::{GeoError, GeoResult};
use std::path::Path;
use std::process::Command;

/// Result of a DVC snapshot operation.
#[derive(Debug, Clone)]
pub struct DvcSnapshot {
    /// The DVC-tracked file path.
    pub file: String,
    /// MD5 hash computed by DVC (stored in .dvc file).
    pub dvc_hash: String,
}

/// Check if `dvc` is available on the system PATH.
pub fn dvc_available() -> bool {
    Command::new("dvc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run `dvc add` to start tracking a file, then `dvc push` to upload to the remote.
///
/// Returns the DVC hash for audit trail linking.
///
/// ## Example
/// ```ignore
/// let snapshot = dvc_snapshot("data/carbon/emission_factors.csv")?;
/// println!("DVC hash: {}", snapshot.dvc_hash);
/// ```
pub fn dvc_snapshot(file_path: &str) -> GeoResult<DvcSnapshot> {
    // 安全：校验路径不含遍历/注入字符
    geo_core::errors::validate_safe_path(file_path)?;

    let path = Path::new(file_path);

    if !path.exists() {
        return Err(GeoError::ExternalProcess {
            command: "dvc add".into(),
            message: format!("File not found: {file_path}"),
        });
    }

    // ── dvc add <file> ──
    let add_status = Command::new("dvc")
        .arg("add")
        .arg(file_path)
        .status()
        .map_err(|e| GeoError::ExternalProcess {
            command: format!("dvc add {file_path}"),
            message: e.to_string(),
        })?;

    if !add_status.success() {
        return Err(GeoError::ExternalProcess {
            command: format!("dvc add {file_path}"),
            message: format!("exit code: {}", add_status.code().unwrap_or(-1)),
        });
    }

    // ── dvc push <file>.dvc ──
    let dvc_file = format!("{file_path}.dvc");
    let push_status = Command::new("dvc")
        .arg("push")
        .arg(&dvc_file)
        .status()
        .map_err(|e| GeoError::ExternalProcess {
            command: format!("dvc push {dvc_file}"),
            message: e.to_string(),
        });

    match push_status {
        Ok(s) if s.success() => {
            tracing::info!("DVC push succeeded for {file_path}");
        }
        Ok(s) => {
            tracing::warn!(
                "DVC push exited with {} (remote may be unavailable)",
                s.code().unwrap_or(-1)
            );
        }
        Err(e) => {
            tracing::warn!("DVC push failed (remote may be unavailable): {e}");
        }
    }

    // ── Read .dvc file for the hash ──
    let dvc_content = std::fs::read_to_string(&dvc_file).map_err(|e| GeoError::Io(e))?;

    let dvc_hash = extract_dvc_hash(&dvc_content).unwrap_or_else(|| {
        tracing::warn!("Could not extract DVC hash from {dvc_file}");
        "unknown".to_string()
    });

    tracing::info!("DVC snapshot: {file_path} → {dvc_hash}");

    Ok(DvcSnapshot {
        file: file_path.to_string(),
        dvc_hash,
    })
}

/// Pull tracked data from the DVC remote.
///
/// If `target` is `None`, pulls all tracked files.
pub fn dvc_pull(target: Option<&str>) -> GeoResult<()> {
    let mut cmd = Command::new("dvc");
    cmd.arg("pull");

    if let Some(t) = target {
        cmd.arg(t);
    }

    let output = cmd.output().map_err(|e| GeoError::ExternalProcess {
        command: "dvc pull".into(),
        message: e.to_string(),
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GeoError::ExternalProcess {
            command: "dvc pull".into(),
            message: stderr.trim().to_string(),
        });
    }

    tracing::info!("DVC pull complete");
    Ok(())
}

/// Get the MD5 hash of a DVC-tracked file from its `.dvc` metadata.
pub fn dvc_hash(file_path: &str) -> GeoResult<String> {
    let dvc_file = format!("{file_path}.dvc");

    if !Path::new(&dvc_file).exists() {
        return Err(GeoError::ExternalProcess {
            command: "dvc hash".into(),
            message: format!("DVC file not found: {dvc_file}. Run `dvc add` first."),
        });
    }

    let content = std::fs::read_to_string(&dvc_file)?;
    extract_dvc_hash(&content).ok_or_else(|| GeoError::ExternalProcess {
        command: "dvc hash".into(),
        message: format!("No hash found in {dvc_file}"),
    })
}

/// Run `dvc repro` to reproduce a DVC pipeline.
///
/// Executes the pipeline defined in `dvc.yaml` and returns the list
/// of stages that were run.
pub fn dvc_repro() -> GeoResult<Vec<String>> {
    let output =
        Command::new("dvc")
            .arg("repro")
            .output()
            .map_err(|e| GeoError::ExternalProcess {
                command: "dvc repro".into(),
                message: e.to_string(),
            })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GeoError::ExternalProcess {
            command: "dvc repro".into(),
            message: stderr.trim().to_string(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stages: Vec<String> = stdout
        .lines()
        .filter(|l| l.contains("Stage") || l.contains("Running"))
        .map(|l| l.trim().to_string())
        .collect();

    tracing::info!("DVC repro complete: {} stages", stages.len());
    Ok(stages)
}

// ── Helpers ───────────────────────────────────────────────────────

/// Extract the `md5` field from a `.dvc` YAML file.
///
/// The `.dvc` file format:
/// ```yaml
/// outs:
/// - md5: abc123def456...
///   size: 12345
///   path: data.csv
/// ```
fn extract_dvc_hash(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(hash) = trimmed.strip_prefix("md5: ") {
            return Some(hash.trim().to_string());
        }
        // Handle YAML with indentation: "  - md5: abc123"
        if trimmed.contains("md5:") {
            if let Some(hash) = trimmed.split("md5:").nth(1) {
                return Some(hash.trim().to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_dvc_hash() {
        let yaml = r#"outs:
- md5: abc123def456789
  size: 12345
  hash: md5
  path: data.csv"#;

        let hash = extract_dvc_hash(yaml);
        assert_eq!(hash, Some("abc123def456789".to_string()));
    }

    #[test]
    fn test_extract_dvc_hash_no_match() {
        let yaml = "outs:\n- path: data.csv\n  size: 12345";
        let hash = extract_dvc_hash(yaml);
        assert_eq!(hash, None);
    }

    #[test]
    fn test_dvc_available() {
        // Just check it doesn't panic — DVC may or may not be installed
        let _ = dvc_available();
    }
}
