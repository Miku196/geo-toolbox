//! Checkpoint/resume support for large raster processing.
//!
//! Writes a `.progress` JSON file after each chunk; on restart, the file
//! is read and already-completed chunks are skipped.
//!
//! Progress format: `{ "chunk_size": 256, "total_chunks": 100, "completed": [0,1,2,...] }`

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// WIP：batch 下载/分块处理的断点续传进度跟踪（尚未接入 CLI 命令，保留以备后续 batch 流程使用）。
// cargo check（非 test 目标）下无构造方，故 allow(dead_code)。
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    pub chunk_size: usize,
    pub total_chunks: usize,
    /// Indices of completed chunks
    pub completed: Vec<usize>,
    /// Input file path (for resume verification)
    pub input_file: String,
    /// Output file path
    pub output_file: String,
}

#[allow(dead_code)]
impl Progress {
    /// Create a new progress tracker for a task.
    pub fn new(
        chunk_size: usize,
        total_chunks: usize,
        input_file: &str,
        output_file: &str,
    ) -> Self {
        Self {
            chunk_size,
            total_chunks,
            completed: Vec::new(),
            input_file: input_file.to_string(),
            output_file: output_file.to_string(),
        }
    }

    /// Progress file path: `<output>.progress`
    pub fn progress_path(output: &Path) -> PathBuf {
        output.with_extension("progress")
    }

    /// Try to load an existing progress file. Returns `None` if not found.
    pub fn load(output: &Path) -> Option<Self> {
        let path = Self::progress_path(output);
        if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
        } else {
            None
        }
    }

    /// Save current progress to disk.
    pub fn save(&self, output: &Path) -> std::io::Result<()> {
        let path = Self::progress_path(output);
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)
    }

    /// Mark a chunk as completed and persist.
    pub fn complete_chunk(&mut self, chunk_idx: usize, output: &Path) -> std::io::Result<()> {
        if !self.completed.contains(&chunk_idx) {
            self.completed.push(chunk_idx);
            self.completed.sort_unstable();
            self.save(output)?;
        }
        Ok(())
    }

    /// Check if a chunk is already completed (for resume).
    pub fn is_done(&self, chunk_idx: usize) -> bool {
        self.completed.contains(&chunk_idx)
    }

    /// Remove the progress file (call on successful completion).
    pub fn cleanup(output: &Path) -> std::io::Result<()> {
        let path = Self::progress_path(output);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Print a progress summary string.
    pub fn summary(&self) -> String {
        let done = self.completed.len();
        let total = self.total_chunks;
        let pct = if total > 0 {
            (done as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        format!("Progress: {done}/{total} chunks ({pct:.1}%)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_create_save_load() {
        let tmp = std::env::temp_dir().join("test_progress.tif");
        let mut p = Progress::new(256, 10, "input.tif", "output.tif");
        assert_eq!(p.total_chunks, 10);

        p.complete_chunk(0, &tmp).unwrap();
        p.complete_chunk(5, &tmp).unwrap();

        // Reload
        let p2 = Progress::load(&tmp).unwrap();
        assert_eq!(p2.completed, vec![0, 5]);
        assert!(p2.is_done(0));
        assert!(!p2.is_done(1));

        Progress::cleanup(&tmp).unwrap();
        assert!(Progress::load(&tmp).is_none());
    }
}
