//! MCP (Model Context Protocol) JSON-RPC server over stdio.
//!
//! Pure protocol layer — all tool dispatch goes through PluginRegistry.
//! Implements the mandatory MCP handshake:
//!   1. `initialize` request → `InitializeResult` response
//!   2. `initialized` notification from client
//!   3. `tools/list` and `tools/call` after handshake
//!
//! Reference: https://spec.modelcontextprotocol.io/specification/2024-11-05/

use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::{stdin, stdout, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::{timeout, Duration};

use geo_registry::PluginRegistry;

/// Server capabilities declared during initialization.
const SERVER_INFO: &str = "geo-toolbox";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Maximum duration for a single tool call before timeout.
const TOOL_TIMEOUT_SECS: u64 = 300;
/// Maximum concurrent requests (rejects excess).
const MAX_CONCURRENT: usize = 8;
/// Maximum total connection duration.
const CONNECTION_TIMEOUT_SECS: u64 = 3600;

// ── Entry point ──

/// Run the MCP server loop over stdio.

/// Run the MCP server loop over stdio.
pub async fn serve(registry: PluginRegistry) -> Result<(), Box<dyn std::error::Error>> {
    let registry = Arc::new(registry);
    let reader = BufReader::new(stdin());
    let mut writer = stdout();
    let mut lines = reader.lines();
    let mut handshake_done = false;
    let active_requests = Arc::new(AtomicUsize::new(0));

    tracing::info!("geo-toolbox MCP server v{SERVER_VERSION} starting on stdio");

    let serve_future = async {
        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }

            let request: Value = match serde_json::from_str(&line) {
                Ok(value) => value,
                Err(error) => {
                    tracing::warn!("Invalid JSON: {error}");
                    continue;
                }
            };
            let method = request["method"].as_str().unwrap_or("");
            tracing::debug!("→ {method}");

            let Some(response) =
                dispatch_request(&registry, &active_requests, &mut handshake_done, &request).await
            else {
                continue;
            };
            let mut response_line = serde_json::to_string(&response)?;
            response_line.push('\n');
            writer.write_all(response_line.as_bytes()).await?;
            writer.flush().await?;
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    };

    match timeout(Duration::from_secs(CONNECTION_TIMEOUT_SECS), serve_future).await {
        Ok(Ok(())) => {
            tracing::info!("MCP server shutting down normally");
            Ok(())
        }
        Ok(Err(error)) => {
            tracing::error!("MCP server error: {error}");
            Err(error)
        }
        Err(_) => {
            tracing::warn!("MCP server connection timeout after {CONNECTION_TIMEOUT_SECS}s");
            Ok(())
        }
    }
}

/// Dispatch one parsed JSON-RPC request and update the MCP handshake state.
///
/// A None result represents a notification, which has no response body.
async fn dispatch_request(
    registry: &Arc<PluginRegistry>,
    active_requests: &Arc<AtomicUsize>,
    handshake_done: &mut bool,
    request: &Value,
) -> Option<Value> {
    let method = request["method"].as_str().unwrap_or("");
    let id = request["id"].clone();
    let response = match method {
        "initialize" => initialize_response(&id, request),
        "notifications/initialized" => {
            *handshake_done = true;
            tracing::info!("MCP handshake complete");
            return None;
        }
        "tools/list" if *handshake_done => {
            let mut tools = registry.generate_mcp_tools();
            tools["id"] = id;
            tools
        }
        "resources/list" if *handshake_done => {
            let mut resources = registry.generate_mcp_resources();
            resources["id"] = id;
            resources
        }
        "resources/read" if *handshake_done => resource_response(&id, request),
        "prompts/list" if *handshake_done => {
            let mut prompts = registry.generate_mcp_prompts();
            prompts["id"] = id;
            prompts
        }
        "prompts/get" if *handshake_done => prompt_response(&id, request),
        "tools/call" if *handshake_done => {
            handle_tools_call(registry, active_requests, request, &id).await
        }
        _ if !*handshake_done && method != "initialize" => json!({
            "jsonrpc": "2.0", "id": id,
            "error": { "code": -32002, "message": "Not initialized. Send 'initialize' first." }
        }),
        _ => json!({
            "jsonrpc": "2.0", "id": id,
            "error": { "code": -32601, "message": format!("Method not found: {method}") }
        }),
    };
    Some(response)
}

fn initialize_response(id: &Value, request: &Value) -> Value {
    let protocol_version = request["params"]["protocolVersion"]
        .as_str()
        .unwrap_or("2024-11-05");
    json!({
        "jsonrpc": "2.0", "id": id,
        "result": {
            "protocolVersion": protocol_version,
            "capabilities": { "tools": {}, "resources": {}, "prompts": {} },
            "serverInfo": { "name": SERVER_INFO, "version": SERVER_VERSION }
        }
    })
}

fn resource_response(id: &Value, request: &Value) -> Value {
    let uri = request["params"]["uri"].as_str().unwrap_or("");
    let content = match uri {
        "geo://datasets/emission-factors" => "IPCC 2019 emission factors with Chinese provincial defaults. Categories: forest, grassland, wetland, cropland, built_up, bare. tCO2e/ha/yr.",
        "geo://datasets/carbon-pools" => "Default carbon pool values (AGB, BGB, Deadwood, Litter, SOC) for 6 eco-zones: Tropical Moist/Dry, Temperate Coniferous/Broadleaf, Boreal, Subtropical Humid.",
        "geo://datasets/soil-groups" => "NRCS hydrologic soil groups: A (high infiltration, sand/gravel), B (moderate, silt loam), C (slow, clay loam), D (low, clay).",
        "geo://datasets/landcover-cn" => "SCS curve numbers for 26 land use types. CN values (AMC II): Forest-Good A:30 B:55 C:70 D:77, Grassland A:39 B:61 C:74 D:80, Urban A:89 B:92 C:94 D:95.",
        "geo://datasets/id-thresholds" => "Global rainfall I-D thresholds: Caine 1980 I=14.82*D^-0.39, Guzzetti 2008 I=2.20*D^-0.44, Hong 2016 I=12.5*D^-0.5, Ma 2015 I=52.0*D^-0.42.",
        "geo://datasets/coastal-carbon" => "Blue carbon IPCC Tier 1: Mangrove (AGB 8.5, BGB 4.3, Soil 49.0 Mg C/ha), Saltmarsh (AGB 2.5, BGB 3.2, Soil 41.8), Seagrass (AGB 0.5, BGB 2.2, Soil 32.0).",
        _ => "Resource not found. Available: geo://datasets/emission-factors, carbon-pools, soil-groups, landcover-cn, id-thresholds, coastal-carbon",
    };
    json!({"jsonrpc":"2.0","id":id,"result":{"contents":[{"uri":uri,"mimeType":"text/plain","text":content}]}})
}

fn prompt_response(id: &Value, request: &Value) -> Value {
    let name = request["params"]["name"].as_str().unwrap_or("");
    let (description, messages) = match name {
        "carbon-assessment" => ("Carbon emission/sink assessment for an area of interest", json!([{"role":"user","content":{"type":"text","text":"Assess carbon emissions/sinks for {{aoi_name}} in year {{year}} using {{source}} methodology. Provide breakdown by land cover type and total net emissions."}}])),
        "ecological-restoration" => ("Ecological restoration assessment with NDVI change + carbon sink", json!([{"role":"user","content":{"type":"text","text":"Assess ecological restoration status of {{aoi_name}} by comparing NDVI between {{baseline_year}} and {{assessment_year}}. Calculate carbon sink potential and provide recommendations."}}])),
        "flood-risk" => ("Flood risk assessment with SCS-CN runoff + watershed analysis", json!([{"role":"user","content":{"type":"text","text":"Perform flood risk assessment for {{aoi_name}} with {{rainfall_mm}}mm rainfall. Use SCS-CN method for runoff and evaluate inundation risk."}}])),
        "geohazard-assessment" => ("Geohazard assessment: landslide susceptibility + FS + Newmark displacement", json!([{"role":"user","content":{"type":"text","text":"Perform geohazard assessment for slope of {{slope_deg}}° with cohesion {{cohesion_kpa}}kPa, friction {{friction_deg}}°. Calculate FS and Newmark displacement for PGA {{pga_g}}g."}}])),
        "solar-suitability" => ("Solar energy site suitability assessment", json!([{"role":"user","content":{"type":"text","text":"Assess solar suitability for {{site_name}} with {{annual_radiation_kwh_m2}} kWh/m² annual radiation. Provide rating and recommendations."}}])),
        "forest-carbon-stock" => ("Forest carbon stock change assessment from NDVI time series", json!([{"role":"user","content":{"type":"text","text":"Assess forest carbon stock change for {{forest_name}} from {{baseline_year}} to {{assessment_year}}. Use IPCC BEF method and evaluate CCER applicability."}}])),
        _ => ("Prompt not found. Available: carbon-assessment, ecological-restoration, flood-risk, geohazard-assessment, solar-suitability, forest-carbon-stock", json!([])),
    };
    json!({"jsonrpc":"2.0","id":id,"result":{"description":description,"messages":messages}})
}

// ── Handler: tools/call ──

/// Handle a tools/call request with concurrency limiting and timeout.
async fn handle_tools_call(
    registry: &Arc<PluginRegistry>,
    active_requests: &Arc<AtomicUsize>,
    request: &Value,
    id: &Value,
) -> Value {
    let current = active_requests.fetch_add(1, Ordering::Relaxed);
    if current >= MAX_CONCURRENT {
        active_requests.fetch_sub(1, Ordering::Relaxed);
        return json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32001,
                "message": format!("Too many concurrent requests ({current}/{MAX_CONCURRENT})")
            }
        });
    }

    let tool_name = request["params"]["name"].as_str().unwrap_or("").to_string();
    let args = request["params"]["arguments"].clone();
    let reg = Arc::clone(registry);

    let result = timeout(
        Duration::from_secs(TOOL_TIMEOUT_SECS),
        dispatch_tool(reg, &tool_name, args),
    )
    .await;

    active_requests.fetch_sub(1, Ordering::Relaxed);

    match result {
        Ok(resp) => resp,
        Err(_) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32000,
                "message": format!("Tool call timed out after {TOOL_TIMEOUT_SECS}s")
            }
        }),
    }
}

/// Dispatch a single tool call through the registry, wrapping result in MCP JSON-RPC format.
async fn dispatch_tool(registry: Arc<PluginRegistry>, tool_name: &str, args: Value) -> Value {
    match registry.dispatch(tool_name, args).await {
        Ok(result) => json!({
            "jsonrpc": "2.0",
            "result": {
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|e| e.to_string())
                }]
            }
        }),
        Err(e) => json!({
            "jsonrpc": "2.0",
            "result": {
                "isError": true,
                "content": [{"type": "text", "text": e.to_string()}]
            }
        }),
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    fn test_registry() -> Arc<PluginRegistry> {
        Arc::new(PluginRegistry::new())
    }

    #[tokio::test]
    async fn initialize_preserves_request_id_and_protocol_version() {
        let registry = test_registry();
        let active_requests = Arc::new(AtomicUsize::new(0));
        let mut handshake_done = false;
        let response = dispatch_request(
            &registry,
            &active_requests,
            &mut handshake_done,
            &json!({"id":"init-1","method":"initialize","params":{"protocolVersion":"2025-03-26"}}),
        )
        .await
        .unwrap();

        assert_eq!(response["id"], "init-1");
        assert_eq!(response["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(response["result"]["serverInfo"]["name"], SERVER_INFO);
        assert!(!handshake_done);
    }

    #[tokio::test]
    async fn initialized_notification_unlocks_registry_methods_without_response() {
        let registry = test_registry();
        let active_requests = Arc::new(AtomicUsize::new(0));
        let mut handshake_done = false;
        let notification = dispatch_request(
            &registry,
            &active_requests,
            &mut handshake_done,
            &json!({"method":"notifications/initialized"}),
        )
        .await;

        assert!(notification.is_none());
        assert!(handshake_done);

        let response = dispatch_request(
            &registry,
            &active_requests,
            &mut handshake_done,
            &json!({"id":2,"method":"resources/list"}),
        )
        .await
        .unwrap();
        assert_eq!(response["id"], 2);
        assert!(response["result"]["resources"].as_array().unwrap().len() >= 6);
    }

    #[tokio::test]
    async fn methods_are_rejected_before_handshake() {
        let registry = test_registry();
        let active_requests = Arc::new(AtomicUsize::new(0));
        let mut handshake_done = false;
        let response = dispatch_request(
            &registry,
            &active_requests,
            &mut handshake_done,
            &json!({"id":3,"method":"tools/list"}),
        )
        .await
        .unwrap();

        assert_eq!(response["error"]["code"], -32002);
    }

    #[tokio::test]
    async fn resource_prompt_and_unknown_method_responses_preserve_protocol_contract() {
        let registry = test_registry();
        let active_requests = Arc::new(AtomicUsize::new(0));
        let mut handshake_done = true;

        let resource = dispatch_request(
            &registry,
            &active_requests,
            &mut handshake_done,
            &json!({"id":4,"method":"resources/read","params":{"uri":"geo://datasets/carbon-pools"}}),
        )
        .await
        .unwrap();
        assert_eq!(
            resource["result"]["contents"][0]["uri"],
            "geo://datasets/carbon-pools"
        );
        assert!(resource["result"]["contents"][0]["text"]
            .as_str()
            .unwrap()
            .contains("carbon pool"));

        let prompt = dispatch_request(
            &registry,
            &active_requests,
            &mut handshake_done,
            &json!({"id":5,"method":"prompts/get","params":{"name":"flood-risk"}}),
        )
        .await
        .unwrap();
        assert!(prompt["result"]["messages"].as_array().unwrap().len() == 1);

        let unknown = dispatch_request(
            &registry,
            &active_requests,
            &mut handshake_done,
            &json!({"id":6,"method":"unknown/method"}),
        )
        .await
        .unwrap();
        assert_eq!(unknown["error"]["code"], -32601);
    }
}
