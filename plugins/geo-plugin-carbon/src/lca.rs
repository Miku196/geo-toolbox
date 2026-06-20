/// Life Cycle Assessment (LCA) submission via brightway2 REST API.
///
/// Sends an LCA inventory to a brightway2 microservice at the configured
/// endpoint URL. Falls back to localhost:8080 if `BRIGHTWAY2_URL` env var
/// is not set.
///
/// The brightway2 service is expected to accept a POST with an inventory
/// path (or JSON body) and return an LCA result string.
use geo_core::errors::{GeoError, GeoResult};

/// Default brightway2 service URL.
const DEFAULT_SERVICE_URL: &str = "http://localhost:8080";

/// Submit an LCA task to the brightway2 Python microservice.
///
/// `inventory_path`: Path or identifier for the LCA inventory.
///
/// Sends POST to `{BRIGHTWAY2_URL}/lca` with JSON body `{"inventory": inventory_path}`.
/// Returns the text response from the service.
pub fn submit_lca(inventory_path: &str) -> GeoResult<String> {
    // Validate input
    if inventory_path.trim().is_empty() {
        return Err(GeoError::Validation(
            "Inventory path cannot be empty".into(),
        ));
    }

    let base_url =
        std::env::var("BRIGHTWAY2_URL").unwrap_or_else(|_| DEFAULT_SERVICE_URL.to_string());
    let url = format!("{base_url}/lca");

    // Build request body
    let body = serde_json::json!({ "inventory": inventory_path });

    // Use blocking reqwest for non-async context
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| GeoError::ExternalProcess {
            command: "brightway2-client".into(),
            message: format!("Failed to create HTTP client: {e}"),
        })?;

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .map_err(|e| GeoError::ExternalProcess {
            command: "brightway2".into(),
            message: format!("HTTP request to {url} failed: {e}"),
        })?;

    let status = response.status();
    if !status.is_success() {
        return Err(GeoError::ExternalProcess {
            command: "brightway2".into(),
            message: format!("Service returned {status}"),
        });
    }

    let text = response.text().map_err(|e| GeoError::ExternalProcess {
        command: "brightway2".into(),
        message: format!("Failed to read response: {e}"),
    })?;

    tracing::info!(
        url = url,
        status = status.as_u16(),
        response_len = text.len(),
        "LCA submission completed"
    );

    Ok(text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submit_lca_empty_path() {
        let result = submit_lca("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_submit_lca_whitespace_path() {
        let result = submit_lca("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_submit_lca_unreachable_service() {
        // Set a non-existent URL to verify error handling
        std::env::set_var("BRIGHTWAY2_URL", "http://127.0.0.1:1");
        let result = submit_lca("test-inventory.json");
        assert!(result.is_err());
        std::env::remove_var("BRIGHTWAY2_URL");
    }
}
