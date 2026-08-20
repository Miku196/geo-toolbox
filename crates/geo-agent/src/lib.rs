mod agent;
mod fallback;
mod metrics;
mod providers;
mod schema;

use agent::Agent;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use fallback::KeywordRouter;
use metrics::MetricsStore;
use providers::{ProviderConfig, ToolCall};
use schema::ToolRegistry;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use tracing::{info, warn};

#[derive(Debug, Deserialize)]
struct AgentRequest {
    query: String,
    #[serde(default)]
    force_fallback: bool,
    #[serde(default)]
    #[allow(dead_code)]
    provider: Option<String>,
}

#[derive(Debug, Serialize)]
struct AgentResponse {
    fallback: bool,
    provider: String,
    model: String,
    tool_calls: Vec<ToolCall>,
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

struct AppState {
    agent: Agent,
    fallback: KeywordRouter,
    metrics: MetricsStore,
    tools: Arc<ToolRegistry>,
    log_dir: PathBuf,
}

/// Build the GeoAgent router without binding a network socket.
///
/// This boundary keeps route and state assembly available to integration tests and
/// alternative hosts while run owns only process startup and listener lifetime.
pub fn build_app() -> Result<Router, Box<dyn std::error::Error + Send + Sync>> {
    let tools_json = include_str!("../tools_schema.json");
    let tool_registry = ToolRegistry::from_json(tools_json).map(Arc::new)?;
    let keywords_yaml = include_str!("../keywords.yaml");
    let keyword_router = KeywordRouter::from_yaml(keywords_yaml)?;
    let config = ProviderConfig::from_env();
    let log_dir = PathBuf::from(std::env::var("LOG_DIR").unwrap_or_else(|_| "logs".to_string()));
    let metrics = MetricsStore::new(log_dir.clone());
    let agent = Agent::new(config, tool_registry.clone());
    let state = Arc::new(AppState {
        agent,
        fallback: keyword_router,
        metrics,
        tools: tool_registry,
        log_dir,
    });
    Ok(Router::new()
        .route("/agent", post(handle_agent))
        .route("/metrics", get(handle_metrics))
        .route("/health", get(handle_health))
        .with_state(state))
}

/// Start the GeoAgent HTTP service. The binary only handles process-level errors.
pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv::dotenv().ok();
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
    info!("geo-toolbox-agent starting");

    let app = build_app()?;
    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("SERVER_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(3000);
    let addr = format!("{host}:{port}");
    info!(%addr, "GeoAgent listening");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn handle_agent(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AgentRequest>,
) -> Result<Json<AgentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let fallback_enabled =
        std::env::var("FALLBACK_ENABLED").unwrap_or_else(|_| "true".to_string()) == "true";
    if !req.force_fallback && fallback_enabled {
        match state.agent.route(&req.query).await {
            Ok(result) if !result.tool_calls.is_empty() => {
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
            Ok(_) => info!("LLM returned no tool calls, trying fallback"),
            Err(error) => warn!(error = %error, "LLM call failed, falling back to keyword router"),
        }
    }
    if let Some((tool, params)) = state.fallback.match_query(&req.query) {
        state
            .metrics
            .record("fallback", "keyword", &Default::default(), true);
        metrics::log_to_file(
            &state.log_dir,
            &format!(
                "[{}] provider=fallback tool={tool} fallback=true",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
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
    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "No matching tool found. Try a different query.".to_string(),
        }),
    ))
}

async fn handle_metrics(State(state): State<Arc<AppState>>) -> Json<MetricsResponse> {
    let snapshot = state.metrics.snapshot();
    Json(MetricsResponse {
        date: snapshot.date,
        calls: snapshot.calls,
        prompt_tokens: snapshot.prompt_tokens,
        completion_tokens: snapshot.completion_tokens,
        total_tokens: snapshot.total_tokens,
        estimated_cost_usd: snapshot.estimated_cost_usd,
        by_provider: snapshot
            .by_provider
            .into_iter()
            .map(|provider| ProviderBreakdown {
                provider: provider.provider,
                model: provider.model,
                calls: provider.calls,
                prompt_tokens: provider.prompt_tokens,
                completion_tokens: provider.completion_tokens,
                total_tokens: provider.total_tokens,
                estimated_cost_usd: provider.estimated_cost_usd,
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

#[cfg(test)]
mod tests {
    use super::build_app;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_route_is_available_without_binding_a_socket() {
        let app = build_app().expect("embedded Agent configuration should load");
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }
}
