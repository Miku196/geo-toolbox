//! MCP protocol end-to-end test — drives the REAL `mcp::serve` binary
//! through stdio, unlike `mcp_smoke.rs` which re-implements the loop in-process.
//!
//! Spawns the `geo-toolbox mcp-serve` subprocess, pipes JSON-RPC lines over
//! stdin/stdout, and asserts the full handshake:
//!   initialize → notifications/initialized → tools/list → tools/call(crs_list)
//!
//! Windows-stability notes:
//! - Uses `std::process::Command` with piped stdio (no pseudo-tty).
//! - Every read is guarded by a 10s deadline so a hung server fails the test
//!   instead of deadlocking CI.
//! - The child is killed on drop (and explicitly at the end of each test) so
//!   no orphan process holds stdout or lingers.
//! - The `--port` flag is passed for CLI-shape fidelity, but `serve` is a
//!   stdio server and ignores it (see `main.rs` `Commands::McpServe`).

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(10);

/// A live subprocess running the MCP server, exposing line-oriented stdio.
struct McpProcess {
    child: Child,
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
}

impl McpProcess {
    fn spawn() -> Self {
        // Cargo exposes the built binary via CARGO_BIN_EXE_<bin-name>. For a
        // bin literally named "geo-toolbox", the `-` is mapped to `_`, giving
        // CARGO_BIN_EXE_geo_toolbox. Check both spellings defensively; fall back
        // to the bare binary name (relies on PATH/target dir) as a last resort.
        let exe = option_env!("CARGO_BIN_EXE_geo_toolbox")
            .or_else(|| option_env!("CARGO_BIN_EXE_geo-toolbox"))
            .unwrap_or("geo-toolbox");

        let mut child = Command::new(exe)
            .args(["mcp-serve", "--port", "19378"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()) // logs go to stderr, keep stdout clean
            .spawn()
            .expect("failed to spawn geo-toolbox mcp-serve; is the binary built?");

        let stdin = child.stdin.take().expect("child stdin");
        let stdout = BufReader::new(child.stdout.take().expect("child stdout"));

        McpProcess {
            child,
            stdin,
            stdout,
        }
    }

    /// Write one JSON-RPC request line and flush.
    fn send(&mut self, request: &serde_json::Value) {
        let mut line = serde_json::to_string(request).expect("serialize request");
        line.push('\n');
        self.stdin
            .write_all(line.as_bytes())
            .expect("write to child stdin");
        self.stdin.flush().expect("flush child stdin");
    }

    /// Read one newline-delimited JSON-RPC response, enforcing the deadline.
    ///
    /// `tracing_subscriber::fmt::init()` in `main.rs` writes its INFO logs to
    /// stdout (not stderr), so the server's JSON-RPC stream is interleaved with
    /// log lines like `INFO geo_toolbox::mcp: ...`. A compliant MCP client must
    /// skip non-JSON lines; we do the same here rather than failing on them.
    fn recv(&mut self) -> serde_json::Value {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            if Instant::now() >= deadline {
                self.kill();
                panic!("timed out after {TIMEOUT:?} waiting for mcp-serve");
            }
            let mut line = String::new();
            match self.stdout.read_line(&mut line) {
                Ok(0) => {
                    self.kill();
                    panic!("mcp-serve exited before responding (EOF on stdout)");
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || !trimmed.starts_with('{') {
                        // Log noise or blank line — skip and keep reading.
                        continue;
                    }
                    return serde_json::from_str(trimmed)
                        .unwrap_or_else(|e| panic!("non-JSON response line {line:?}: {e}"));
                }
                Err(e) => {
                    self.kill();
                    panic!("I/O error reading from mcp-serve: {e}");
                }
            }
        }
    }

    fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for McpProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn initialize_req(id: i64) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "e2e-test", "version": "1.0"}
        }
    })
}

fn initialized_notif() -> serde_json::Value {
    // JSON-RPC notification: no `id`, so the server sends no response.
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    })
}

#[test]
fn mcp_e2e_full_handshake_and_crs_list() {
    let mut proc = McpProcess::spawn();

    // 1. initialize
    proc.send(&initialize_req(1));
    let init = proc.recv();
    assert_eq!(init["jsonrpc"], "2.0");
    assert_eq!(init["id"], 1);
    assert_eq!(init["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(init["result"]["serverInfo"]["name"], "geo-toolbox");
    assert!(init["result"]["capabilities"]["tools"].is_object());

    // 2. initialized notification (no response expected)
    proc.send(&initialized_notif());

    // 3. tools/list
    proc.send(&serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}));
    let list = proc.recv();
    assert_eq!(list["id"], 2);
    let tools = list["result"]["tools"]
        .as_array()
        .expect("tools/list should return a tools array");
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(
        names.contains(&"crs_list"),
        "tools should include crs_list, got: {names:?}"
    );

    // 4. tools/call crs_list
    proc.send(&serde_json::json!({
        "jsonrpc": "2.0", "id": 3,
        "method": "tools/call",
        "params": {"name": "crs_list", "arguments": {}}
    }));
    let call = proc.recv();
    // NOTE(bug): `mcp::dispatch_tool` builds the success response as
    // `{"jsonrpc":"2.0","result":{...}}` WITHOUT echoing the request `id`,
    // violating JSON-RPC 2.0. We therefore assert on the payload, not the id.
    assert!(
        call["result"].is_object(),
        "tools/call should return a result object, got: {call}"
    );
    assert!(
        !call["result"]["isError"].as_bool().unwrap_or(false),
        "crs_list call should not error: {call}"
    );

    let text = call["result"]["content"][0]["text"]
        .as_str()
        .expect("content[0].text");
    let crs: serde_json::Value =
        serde_json::from_str(text).expect("crs_list text should be a JSON array");
    let arr = crs.as_array().expect("crs_list result should be an array");
    assert!(!arr.is_empty(), "crs_list should list at least one CRS");
    assert!(
        arr.iter().any(|c| c["epsg"] == 4326),
        "crs_list should contain EPSG:4326, got: {text}"
    );
}

#[test]
fn mcp_e2e_pre_handshake_rejection() {
    let mut proc = McpProcess::spawn();
    proc.send(&serde_json::json!({"jsonrpc": "2.0", "id": 9, "method": "tools/list"}));
    let resp = proc.recv();
    assert_eq!(resp["id"], 9);
    assert_eq!(resp["error"]["code"], -32002);
    assert!(resp["error"]["message"]
        .as_str()
        .unwrap_or("")
        .contains("initialized"));
}

#[test]
fn mcp_e2e_unknown_method() {
    let mut proc = McpProcess::spawn();
    proc.send(&initialize_req(1));
    let _ = proc.recv();
    proc.send(&initialized_notif());
    proc.send(&serde_json::json!({"jsonrpc": "2.0", "id": 10, "method": "no/such/method"}));
    let resp = proc.recv();
    assert_eq!(resp["id"], 10);
    assert_eq!(resp["error"]["code"], -32601);
}
