//! geo-server: HTTP API for geo-toolbox.
//!
//! Routes all 44 MCP tools behind a REST interface.
//! Usage: `cargo run -p geo-server --release`
//! Server listens on http://0.0.0.0:9378

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use geo_registry::PluginRegistry;
use std::sync::Arc;

mod registry;
use registry::build_registry;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let registry = Arc::new(build_registry());
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/tools", get(list_tools))
        .route("/api/call/{tool}", post(call_tool))
        .with_state(registry);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:9378").await.unwrap();
    tracing::info!("geo-server listening on http://0.0.0.0:9378");
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "ok"
}

async fn list_tools(State(reg): State<Arc<PluginRegistry>>) -> Json<serde_json::Value> {
    Json(reg.generate_mcp_tools())
}

async fn call_tool(
    State(reg): State<Arc<PluginRegistry>>,
    Path(tool): Path<String>,
    Json(args): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    match reg.dispatch(&tool, args).await {
        Ok(result) => Json(serde_json::json!({"ok": true, "data": result})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}
