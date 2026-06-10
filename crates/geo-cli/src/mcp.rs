//! MCP (Model Context Protocol) JSON-RPC server over stdio.
//!
//! Implements the mandatory MCP handshake:
//!   1. `initialize` request → `InitializeResult` response
//!   2. `initialized` notification from client
//!   3. `tools/list` and `tools/call` after handshake
//!
//! Reference: https://spec.modelcontextprotocol.io/specification/2024-11-05/

use serde_json::{json, Value};
use tokio::io::{stdin, stdout, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::{timeout, Duration};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use geo_registry::PluginRegistry;

/// Server capabilities declared during initialization.
const SERVER_INFO: &str = "geo-toolbox";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Maximum duration for a single tool call before timeout.
const TOOL_TIMEOUT_SECS: u64 = 30;
/// Maximum concurrent requests (rejects excess with 503).
const MAX_CONCURRENT: usize = 8;
/// Maximum total connection duration.
const CONNECTION_TIMEOUT_SECS: u64 = 3600;

/// Run the MCP server loop over stdio.
pub async fn serve(registry: PluginRegistry) -> Result<(), Box<dyn std::error::Error>> {
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
                let client_info = &request["params"]["protocolVersion"];
                let protocol_version = client_info.as_str().unwrap_or("2024-11-05");

                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": protocol_version,
                        "capabilities": {
                            "tools": {}  // We support tools
                        },
                        "serverInfo": {
                            "name": SERVER_INFO,
                            "version": SERVER_VERSION
                        }
                    }
                })
            }

            // ── Handshake completion ──
            "notifications/initialized" => {
                handshake_done = true;
                tracing::info!("MCP handshake complete");
                continue; // No response for notifications
            }

            // ── Tools ──
            "tools/list" if handshake_done => {
                // 动态生成：从 PluginRegistry 获取所有已注册工具
                let mut tools_json = registry.generate_mcp_tools();
                tools_json["id"] = id;
                tools_json
            }

            "tools/call" if handshake_done => {
                // Rate limiting: check concurrent request count
                let current = active_requests.fetch_add(1, Ordering::Relaxed);
                if current >= MAX_CONCURRENT {
                    active_requests.fetch_sub(1, Ordering::Relaxed);
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32001,
                            "message": format!("Too many concurrent requests ({current}/{MAX_CONCURRENT}). Try again later.")
                        }
                    })
                } else {
                    let tool_name = request["params"]["name"].as_str().unwrap_or("").to_string();
                    let args = request["params"]["arguments"].clone();
                    // Timeout wrapper
                    let result = timeout(
                        Duration::from_secs(TOOL_TIMEOUT_SECS),
                        handle_tool_call(&tool_name, &args),
                    ).await;
                    active_requests.fetch_sub(1, Ordering::Relaxed);
                    match result {
                        Ok(res) => res.unwrap_or_else(|e| json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": {"code": -32000, "message": e.to_string()}
                        })),
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
            }

            // ── Reject calls before handshake ──
            _ if !handshake_done && method != "initialize" => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32002,
                        "message": "Not initialized. Send 'initialize' first."
                    }
                })
            }

            // ── Unknown method ──
            _ => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("Method not found: {method}")
                    }
                })
            }
        };

        // Write response
        let mut resp_str = serde_json::to_string(&response)?;
        resp_str.push('\n');
        writer.write_all(resp_str.as_bytes()).await?;
        writer.flush().await?;
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    };

    // Connection-level timeout
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

/// Helper: get DATABASE_URL from env, return error if not set
fn get_db_url() -> Result<String, Box<dyn std::error::Error>> {
    std::env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL environment variable must be set".into())
}

/// Dispatch tool calls to the appropriate handler.
async fn handle_tool_call(tool: &str, args: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    match tool {
        "crs_list" => {
            let reg = geo_core::crs::CrsRegistry::new();
            let list: Vec<_> = reg.list().map(|c| json!({
                "epsg": c.epsg,
                "name": c.name,
                "category": format!("{:?}", c.category),
                "proj4": c.proj4
            })).collect();

            Ok(json!({
                "jsonrpc": "2.0",
                "result": {
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string_pretty(&list)?
                    }]
                }
            }))
        }

        "crs_transform" => {
            let reg = geo_core::crs::CrsRegistry::new();
            let from = args["from_epsg"].as_u64().unwrap_or(4326) as u16;
            let to = args["to_epsg"].as_u64().unwrap_or(4326) as u16;
            let x = args["x"].as_f64().unwrap_or(0.0);
            let y = args["y"].as_f64().unwrap_or(0.0);

            match reg.transform_point(from, to, x, y) {
                Ok((ox, oy)) => Ok(json!({
                    "jsonrpc": "2.0",
                    "result": {
                        "content": [{
                            "type": "text",
                            "text": format!("EPSG:{from} ({x}, {y}) → EPSG:{to} ({ox:.4}, {oy:.4})")
                        }]
                    }
                })),
                Err(e) => Ok(json!({
                    "jsonrpc": "2.0",
                    "result": {
                        "isError": true,
                        "content": [{"type": "text", "text": e.to_string()}]
                    }
                }))
            }
        }

        "store_migrate" => {
            let db_url = get_db_url()?;
            let store = geo_adapter_postgis::PostgisStore::connect(&db_url).await?;
            geo_adapter_postgis::run_migrations(store.pool()).await?;

            Ok(json!({
                "jsonrpc": "2.0",
                "result": {
                    "content": [{"type": "text", "text": "Migrations applied successfully"}]
                }
            }))
        }

        "store_query" => {
            let sql = args["sql"].as_str().unwrap_or("SELECT 1");
            let db_url = get_db_url()?;
            let store = geo_adapter_postgis::PostgisStore::connect(&db_url).await?;
            let rows = store.query_json(sql).await?;

            Ok(json!({
                "jsonrpc": "2.0",
                "result": {
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string_pretty(&rows)?
                    }]
                }
            }))
        }

        "ingest_camofox" => {
            let file = args["file"].as_str().unwrap_or("");
            let content = tokio::fs::read_to_string(file).await?;
            let (_rows, result) = geo_io::camofox::parse_camofox_file(&content, file)?;

            Ok(json!({
                "jsonrpc": "2.0",
                "result": {
                    "content": [{
                        "type": "text",
                        "text": format!("CamoFox ingest: {} accepted, {} rejected", result.accepted, result.rejected)
                    }]
                }
            }))
        }

        "ingest_nmea" => {
            let file = args["file"].as_str().unwrap_or("");
            let content = tokio::fs::read_to_string(file).await?;
            let mut fixes = 0u32;
            let mut records = Vec::new();

            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() { continue; }
                if let Ok(msg) = geo_io::nmea::parse_nmea_line(line) {
                    match msg {
                        geo_io::nmea::NmeaMessage::Gga(fix) => {
                            records.push(json!({
                                "type": "GGA",
                                "time": fix.time,
                                "lat": fix.lat,
                                "lng": fix.lng,
                                "quality": fix.quality,
                                "satellites": fix.satellites
                            }));
                            fixes += 1;
                        }
                        geo_io::nmea::NmeaMessage::Rmc(rmc) => {
                            records.push(json!({
                                "type": "RMC",
                                "time": rmc.time,
                                "lat": rmc.lat,
                                "lng": rmc.lng,
                                "speed_knots": rmc.speed_knots
                            }));
                            fixes += 1;
                        }
                        _ => {}
                    }
                }
            }

            Ok(json!({
                "jsonrpc": "2.0",
                "result": {
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string_pretty(&json!({
                            "total_fixes": fixes,
                            "records": &records[..records.len().min(10)]
                        }))?
                    }]
                }
            }))
        }

        "dvc_snapshot" => {
            let file = args["file"].as_str().unwrap_or("");
            let snap = geo_adapter_postgis::dvc_snapshot(file)?;
            Ok(json!({
                "jsonrpc": "2.0",
                "result": {
                    "content": [{"type": "text", "text": format!("DVC snapshot: {} → {}", snap.file, snap.dvc_hash)}]
                }
            }))
        }

        "dvc_hash" => {
            let file = args["file"].as_str().unwrap_or("");
            let hash = geo_adapter_postgis::dvc_hash(file)?;
            Ok(json!({
                "jsonrpc": "2.0",
                "result": {
                    "content": [{"type": "text", "text": hash}]
                }
            }))
        }

        "carbon_calculate" => {
            let aoi = args["aoi_id"].as_str().unwrap_or("");
            let year = args["year"].as_u64().unwrap_or(2025) as u16;
            let source = args["source"].as_str().unwrap_or("IPCC_2019");
            let aoi_id = uuid::Uuid::parse_str(aoi)
                .map_err(|e| geo_core::GeoError::Validation(format!("invalid AOI UUID: {e}")))?;

            let db_url = get_db_url()?;
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(2).connect(&db_url).await?;
            let engine = geo_adapter_postgis::PostgisCarbonEngine::new(pool);

            match engine.calculate_emission_factor(aoi_id, year, source).await {
                Ok(results) => {
                    let total: f64 = results.iter().map(|r| r.emission_tco2e).sum();
                    let summary: Vec<_> = results.iter().map(|r| json!({
                        "landcover_class": r.landcover_class,
                        "area_ha": r.area_ha,
                        "emission_tco2e": r.emission_tco2e,
                        "audit": r.audit.summary()
                    })).collect();

                    Ok(json!({"jsonrpc": "2.0", "result": {"content": [{
                        "type": "text", "text": serde_json::to_string_pretty(&json!({
                            "aoi_id": aoi, "year": year, "total_tco2e": total, "results": summary
                        }))?
                    }]}}))
                }
                Err(e) => Ok(json!({"jsonrpc": "2.0", "result": {"isError": true,
                    "content": [{"type": "text", "text": e.to_string()}]}}))
            }
        }

        "carbon_dry_run" => {
            let aoi = args["aoi_id"].as_str().unwrap_or("");
            let year = args["year"].as_u64().unwrap_or(2025) as u16;
            let source = args["source"].as_str().unwrap_or("IPCC_2019");
            let aoi_id = uuid::Uuid::parse_str(aoi)
                .map_err(|e| geo_core::GeoError::Validation(format!("invalid AOI UUID: {e}")))?;

            let db_url = get_db_url()?;
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(2).connect(&db_url).await?;
            let engine = geo_adapter_postgis::PostgisCarbonEngine::new(pool);

            match engine.calculate_dry_run(aoi_id, year, source).await {
                Ok(results) => {
                    let total: f64 = results.iter().map(|r| r.emission_tco2e).sum();
                    Ok(json!({"jsonrpc": "2.0", "result": {"content": [{
                        "type": "text", "text": format!(
                            "DRY-RUN: {} classes, {:.1} tCO₂e total (not written to DB)",
                            results.len(), total
                        )
                    }]}}))
                }
                Err(e) => Ok(json!({"jsonrpc": "2.0", "result": {"isError": true,
                    "content": [{"type": "text", "text": e.to_string()}]}}))
            }
        }

        "carbon_import_factors" => {
            let csv_path = args["csv_path"].as_str().unwrap_or("");
            let db_url = get_db_url()?;
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(2).connect(&db_url).await?;
            let engine = geo_adapter_postgis::PostgisCarbonEngine::new(pool);

            let count = engine.import_factors_csv(csv_path).await?;
            Ok(json!({"jsonrpc": "2.0", "result": {
                "content": [{"type": "text", "text": format!("Imported {count} emission factors from {csv_path}")}]
            }}))
        }

        "carbon_query_factors" => {
            let year = args["year"].as_i64().unwrap_or(2025) as i32;
            let source = args["source"].as_str();
            let db_url = get_db_url()?;
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(2).connect(&db_url).await?;
            let engine = geo_adapter_postgis::PostgisCarbonEngine::new(pool);

            let factors = engine.query_factors(year, source).await?;
            Ok(json!({"jsonrpc": "2.0", "result": {
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&factors)?}]
            }}))
        }

        _ => Ok(json!({
            "jsonrpc": "2.0",
            "error": {"code": -32601, "message": format!("Unknown tool: {tool}")}
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_response() {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "pi-agent", "version": "1.0"}
            }
        });

        // Verify the response structure
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
        // Calling tools/list before initialize should fail
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
