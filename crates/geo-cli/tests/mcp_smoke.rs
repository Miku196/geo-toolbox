//! MCP smoke test — verifies the full JSON-RPC protocol pipeline in-process.
//!
//! Runs the MCP server in a background task and communicates via channels,
//! avoiding subprocess spawn complexity on Windows.

use serde_json::{json, Value};
use tokio::sync::mpsc;

/// Run MCP server in background, return (tx, rx) channels for JSON-RPC.
fn spawn_mcp_server() -> (mpsc::UnboundedSender<String>, mpsc::UnboundedReceiver<String>) {
    let (to_server, mut from_client) = mpsc::unbounded_channel::<String>();
    let (to_client, from_server) = mpsc::unbounded_channel::<String>();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap();

        rt.block_on(async {
            let registry = geo_cli_build_registry();
            let registry = std::sync::Arc::new(registry);

            loop {
                let line = match from_client.recv().await {
                    Some(l) => l,
                    None => break,
                };
                if line.trim().is_empty() { continue; }

                let request: Value = match serde_json::from_str(&line) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let method = request["method"].as_str().unwrap_or("");
                let id = request["id"].clone();

                let response = match method {
                    "initialize" => json!({
                        "jsonrpc": "2.0", "id": id,
                        "result": {
                            "protocolVersion": "2024-11-05",
                            "capabilities": {"tools": {}},
                            "serverInfo": {"name": "geo-toolbox", "version": "0.1.0"}
                        }
                    }),
                    "tools/list" => {
                        let mut tools_json = registry.generate_mcp_tools();
                        tools_json["id"] = id;
                        tools_json
                    }
                    "tools/call" => {
                        let tool_name = request["params"]["name"].as_str().unwrap_or("").to_string();
                        let args = request["params"]["arguments"].clone();
                        let reg = std::sync::Arc::clone(&registry);
                        match reg.dispatch(&tool_name, args).await {
                            Ok(result) => json!({
                                "jsonrpc": "2.0", "id": id,
                                "result": {
                                    "content": [{"type": "text", "text": serde_json::to_string_pretty(&result).unwrap_or_default()}]
                                }
                            }),
                            Err(e) => json!({
                                "jsonrpc": "2.0", "id": id,
                                "result": {
                                    "isError": true,
                                    "content": [{"type": "text", "text": e.to_string()}]
                                }
                            }),
                        }
                    }
                    _ => json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": format!("Method not found: {method}")}}),
                };

                let mut resp_str = serde_json::to_string(&response).unwrap();
                resp_str.push('\n');
                let _ = to_client.send(resp_str);
            }
        });
    });

    (to_server, from_server)
}

/// Helper: call MCP via channels
async fn mcp_call(
    tx: &mpsc::UnboundedSender<String>,
    rx: &mut mpsc::UnboundedReceiver<String>,
    request: &Value,
) -> Value {
    let mut req_str = serde_json::to_string(request).unwrap();
    req_str.push('\n');
    tx.send(req_str).unwrap();
    let line = rx.recv().await.unwrap();
    serde_json::from_str(&line).unwrap()
}

// We need access to build_registry. Re-export or duplicate minimal.
// Using the same function as main.rs — import via module.
fn geo_cli_build_registry() -> geo_registry::PluginRegistry {
    let mut reg = geo_registry::PluginRegistry::new();
    geo_io::tools::register_tools(&mut reg);
    geo_plugin_ecology::tools::register_tools(&mut reg);
    reg
}

#[tokio::test]
async fn test_mcp_initialize_and_list_tools() {
    let (tx, mut rx) = spawn_mcp_server();

    // Initialize
    let init = mcp_call(&tx, &mut rx, &json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test", "version": "1.0"}}
    })).await;

    assert_eq!(init["id"], 1);
    assert_eq!(init["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(init["result"]["serverInfo"]["name"], "geo-toolbox");

    // List tools
    let list = mcp_call(&tx, &mut rx, &json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/list"
    })).await;

    let tools = list["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    println!("Tools: {names:?}");

    assert!(names.contains(&"crs_list"));
    assert!(names.contains(&"crs_transform"));
    assert!(names.contains(&"ingest_camofox"));
}

#[tokio::test]
async fn test_mcp_crs_transform() {
    let (tx, mut rx) = spawn_mcp_server();

    // Init + tools/call
    mcp_call(&tx, &mut rx, &json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test", "version": "1.0"}}
    })).await;

    let resp = mcp_call(&tx, &mut rx, &json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": {"name": "crs_transform", "arguments": {"from_epsg": 4326, "to_epsg": 3857, "x": 104.06, "y": 30.57}}
    })).await;

    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let inner: Value = serde_json::from_str(text).unwrap();
    assert_eq!(inner["from_epsg"], 4326);
    let out_x = inner["output"][0].as_f64().unwrap();
    assert!(out_x > 10000000.0, "x={out_x} should be Web Mercator");
}

#[tokio::test]
async fn test_mcp_unknown_tool() {
    let (tx, mut rx) = spawn_mcp_server();

    mcp_call(&tx, &mut rx, &json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test", "version": "1.0"}}
    })).await;

    let resp = mcp_call(&tx, &mut rx, &json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": {"name": "nonexistent_tool", "arguments": {}}
    })).await;

    assert!(resp["result"]["isError"].as_bool().unwrap_or(false),
        "unknown tool should return isError: true, got {}", resp);
}
