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
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("Invalid JSON: {e}");
                    continue;
                }
            };

            let method = request["method"].as_str().unwrap_or("");
            let id = request["id"].clone();

            tracing::debug!("→ {method}");

            let response = match method {
                // ── Mandatory handshake ──
                "initialize" => {
                    let protocol_version = request["params"]["protocolVersion"]
                        .as_str()
                        .unwrap_or("2024-11-05");
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "protocolVersion": protocol_version,
                            "capabilities": { "tools": {} },
                            "serverInfo": {
                                "name": SERVER_INFO,
                                "version": SERVER_VERSION
                            }
                        }
                    })
                }

                "notifications/initialized" => {
                    handshake_done = true;
                    tracing::info!("MCP handshake complete");
                    continue; // No response for notifications
                }

                // ── Tools: list ──
                "tools/list" if handshake_done => {
                    let mut tools_json = registry.generate_mcp_tools();
                    tools_json["id"] = id;
                    tools_json
                }

                // ── Tools: call ──
                "tools/call" if handshake_done => {
                    let current = active_requests.fetch_add(1, Ordering::Relaxed);
                    if current >= MAX_CONCURRENT {
                        active_requests.fetch_sub(1, Ordering::Relaxed);
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": {
                                "code": -32001,
                                "message": format!(
                                    "Too many concurrent requests ({current}/{MAX_CONCURRENT})"
                                )
                            }
                        })
                    } else {
                        let tool_name =
                            request["params"]["name"].as_str().unwrap_or("").to_string();
                        let args = request["params"]["arguments"].clone();
                        let reg = Arc::clone(&registry);

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
                                    "message": format!(
                                        "Tool call timed out after {TOOL_TIMEOUT_SECS}s"
                                    )
                                }
                            }),
                        }
                    }
                }

                // ── Reject calls before handshake ──
                _ if !handshake_done && method != "initialize" => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32002,
                        "message": "Not initialized. Send 'initialize' first."
                    }
                }),

                // ── Unknown method ──
                _ => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("Method not found: {method}")
                    }
                }),
            };

            let mut resp_str = serde_json::to_string(&response)?;
            resp_str.push('\n');
            writer.write_all(resp_str.as_bytes()).await?;
            writer.flush().await?;
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    };

    match timeout(Duration::from_secs(CONNECTION_TIMEOUT_SECS), serve_future).await {
        Ok(Ok(())) => {
            tracing::info!("MCP server shutting down normally");
            Ok(())
        }
        Ok(Err(e)) => {
            tracing::error!("MCP server error: {e}");
            Err(e)
        }
        Err(_) => {
            tracing::warn!("MCP server connection timeout after {CONNECTION_TIMEOUT_SECS}s");
            Ok(())
        }
    }
}

/// Dispatch a single tool call through the registry, wrapping result in MCP JSON-RPC format.
async fn dispatch_tool(registry: Arc<PluginRegistry>, tool_name: &str, args: Value) -> Value {
    match registry.dispatch(tool_name, args).await {
        Ok(result) => {
            // result 是 handler 返回的 Value，直接作为 content text
            json!({
                "jsonrpc": "2.0",
                "result": {
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string_pretty(&result)
                            .unwrap_or_else(|e| e.to_string())
                    }]
                }
            })
        }
        Err(e) => json!({
            "jsonrpc": "2.0",
            "result": {
                "isError": true,
                "content": [{"type": "text", "text": e.to_string()}]
            }
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_response() {
        let resp = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "geo-toolbox", "version": SERVER_VERSION}
            }
        });

        assert_eq!(resp["result"]["protocolVersion"], "2024-11-05");
        assert_eq!(resp["result"]["serverInfo"]["name"], "geo-toolbox");
    }

    #[test]
    fn test_pre_handshake_rejection() {
        let error = json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32002,
                "message": "Not initialized. Send 'initialize' first."
            }
        });
        assert_eq!(error["error"]["code"], -32002);
    }
}
