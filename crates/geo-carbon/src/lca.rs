//! LCA (Life Cycle Assessment) via brightway2 subprocess.
//!
//! Delegates to an external Python brightway2 process for full
//! life-cycle analysis. The brightway2 service runs as a microservice
//! (see Risk 6) to avoid blocking the main carbon pipeline during
//! database downloads (ecoinvent ~10GB).

use geo_core::errors::GeoResult;
use std::process::Command;

/// Submit an LCA task to the brightway2 Python microservice.
///
/// The microservice is expected to be running on localhost:8080,
/// started separately via `python lca_service.py`.
///
/// This function is a placeholder — the actual HTTP call will be
/// implemented when the brightway2 service is ready.
pub fn submit_lca(inventory_path: &str) -> GeoResult<String> {
    // Attempt subprocess call (fallback if microservice isn't running)
    let output = Command::new("python")
        .args(["-c", &format!(
            "print('LCA WIP: inventory={inventory_path}')"
        )])
        .output()
        .map_err(|e| geo_core::errors::GeoError::ExternalProcess {
            command: "lca".into(),
            message: e.to_string(),
        })?;

    let message = String::from_utf8_lossy(&output.stdout).trim().to_string();
    tracing::info!("LCA: {message}");
    Ok(message)
}
