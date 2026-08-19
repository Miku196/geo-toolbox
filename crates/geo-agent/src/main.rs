mod agent;
mod fallback;
mod metrics;
mod providers;
mod schema;

use agent::Agent;
use fallback::KeywordRouter;
use metrics::MetricsStore;
use providers::{ProviderConfig, ToolCall};
use schema::ToolRegistry;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, warn};

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};

// ── Request / Response types ──

#[derive(Debug, Deserialize)]
struct AgentRequest {
    query: String,
    /// Optional: force-fallback mode (skip API call)
    #[serde(default)]
    force_fallback: bool,
    /// Optional: override the provider
    #[serde(default)]
    #[allow(dead_code)]
    provider: Option<String>,
}

#[derive(Debug, Serialize)]
struct AgentResponse {
    /// True if keyword fallback was used
    fallback: bool,
    /// Provider used
    provider: String,
    /// Model used
    model: String,
    /// Parsed tool calls
    tool_calls: Vec<ToolCall>,
    /// Token usage (None if fallback)
    usage: Option<UsageInfo>,
}

#[derive(Debug, Serialize)]
struct UsageInfo {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

#[derive(Debug, Serialize)]
struct MetricsResponse {
    date: String,
    calls: u64,
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
    estimated_cost_usd: f64,
    by_provider: Vec<ProviderBreakdown>,
}

#[derive(Debug, Serialize)]
struct ProviderBreakdown {
    provider: String,
    model: String,
    calls: u64,
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
    estimated_cost_usd: f64,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    tools_count: usize,
    fallback_rules_count: usize,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

// ── App State ──

struct AppState {
    agent: Agent,
    fallback: KeywordRouter,
    metrics: MetricsStore,
    tools: Arc<ToolRegistry>,
    log_dir: PathBuf,
}

// ── Main ──

#[tokio::main]
async fn main() {
    // Load .env
    dotenv::dotenv().ok();

    // Initialize tracing
    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    let log_format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "pretty".to_string());

    if log_format == "json" {
        tracing_subscriber::fmt()
            .with_env_filter(&log_level)
            .json()
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(&log_level).init();
    }

    info!("🚀 geo-toolbox-agent starting...");

    // Load tools schema
    let tools_json = include_str!("../tools_schema.json");
    let tool_registry = match ToolRegistry::from_json(tools_json) {
        Ok(r) => {
            info!(count = r.len(), "Tools schema loaded");
            Arc::new(r)
        }
        Err(e) => {
            error!(error = %e, "Failed to load tools_schema.json");
            std::process::exit(1);
        }
    };

    // Load keywords
    let keywords_yaml = include_str!("../keywords.yaml");
    let keyword_router = match KeywordRouter::from_yaml(keywords_yaml) {
        Ok(r) => {
            info!(rules = r.rule_count(), "Keyword router loaded");
            r
        }
        Err(e) => {
            error!(error = %e, "Failed to load keywords.yaml");
            std::process::exit(1);
        }
    };

    // Config
    let config = ProviderConfig::from_env();
    info!(
        provider = %config.provider,
        model = %config.model,
        "AI provider configured"
    );

    // Metrics store
    let log_dir = PathBuf::from(std::env::var("LOG_DIR").unwrap_or_else(|_| "logs".to_string()));
    let metrics = MetricsStore::new(log_dir.clone());

    // Build agent
    let agent = Agent::new(config, tool_registry.clone());

    // App state
    let state = Arc::new(AppState {
        agent,
        fallback: keyword_router,
        metrics,
        tools: tool_registry,
        log_dir,
    });

    // Router
    let app = Router::new()
        .route("/agent", post(handle_agent))
        .route("/metrics", get(handle_metrics))
        .route("/health", get(handle_health))
        .with_state(state);

    // Bind
    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("SERVER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);

    let addr = format!("{}:{}", host, port);
    info!("🌐 Listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ── Handlers ──

async fn handle_agent(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AgentRequest>,
) -> Result<Json<AgentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let fallback_enabled =
        std::env::var("FALLBACK_ENABLED").unwrap_or_else(|_| "true".to_string()) == "true";

    // Try LLM first
    if !req.force_fallback && fallback_enabled {
        match state.agent.route(&req.query).await {
            Ok(result) => {
                if !result.tool_calls.is_empty() {
                    // Record metrics
                    state
                        .metrics
                        .record(&result.provider, &result.model, &result.usage, false);
                    metrics::log_to_file(
                        &state.log_dir,
                        &format!(
                            "[{}] provider={} model={} prompt={} completion={} total={} fallback=false",
                            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                            result.provider,
                            result.model,
                            result.usage.prompt_tokens,
                            result.usage.completion_tokens,
                            result.usage.total_tokens
                        ),
                    );

                    return Ok(Json(AgentResponse {
                        fallback: false,
                        provider: result.provider,
                        model: result.model,
                        tool_calls: result.tool_calls,
                        usage: Some(UsageInfo {
                            prompt_tokens: result.usage.prompt_tokens,
                            completion_tokens: result.usage.completion_tokens,
                            total_tokens: result.usage.total_tokens,
                        }),
                    }));
                }
                // LLM returned no tool calls — fall through to fallback
                info!("LLM returned no tool calls, trying fallback");
            }
            Err(e) => {
                warn!(error = %e, "LLM call failed, falling back to keyword router");
            }
        }
    }

    // Fallback: keyword routing
    if let Some((tool, params)) = state.fallback.match_query(&req.query) {
        state
            .metrics
            .record("fallback", "keyword", &Default::default(), true);
        metrics::log_to_file(
            &state.log_dir,
            &format!(
                "[{}] provider=fallback tool={tool} fallback=true",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            ),
        );

        return Ok(Json(AgentResponse {
            fallback: true,
            provider: "fallback".to_string(),
            model: "keyword-router".to_string(),
            tool_calls: vec![ToolCall { tool, params }],
            usage: None,
        }));
    }

    // No match at all
    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "No matching tool found. Try a different query.".to_string(),
        }),
    ))
}

async fn handle_metrics(State(state): State<Arc<AppState>>) -> Json<MetricsResponse> {
    let snap = state.metrics.snapshot();
    Json(MetricsResponse {
        date: snap.date,
        calls: snap.calls,
        prompt_tokens: snap.prompt_tokens,
        completion_tokens: snap.completion_tokens,
        total_tokens: snap.total_tokens,
        estimated_cost_usd: snap.estimated_cost_usd,
        by_provider: snap
            .by_provider
            .into_iter()
            .map(|p| ProviderBreakdown {
                provider: p.provider,
                model: p.model,
                calls: p.calls,
                prompt_tokens: p.prompt_tokens,
                completion_tokens: p.completion_tokens,
                total_tokens: p.total_tokens,
                estimated_cost_usd: p.estimated_cost_usd,
            })
            .collect(),
    })
}

async fn handle_health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        tools_count: state.tools.len(),
        fallback_rules_count: state.fallback.rule_count(),
    })
}
