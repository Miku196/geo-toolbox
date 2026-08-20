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
    fallback_enabled: bool,
    metrics: MetricsStore,
    tools: Arc<ToolRegistry>,
    log_dir: PathBuf,
}

/// Runtime configuration for an embedded GeoAgent application.
///
/// It separates process environment parsing from router construction, so hosts and
/// tests can select a provider, log directory, and fallback policy explicitly.
#[derive(Debug, Clone)]
pub struct AgentAppConfig {
    pub provider: ProviderConfig,
    pub log_dir: PathBuf,
    pub fallback_enabled: bool,
}

impl Default for AgentAppConfig {
    /// Load the compatibility defaults used by the standalone binary.
    fn default() -> Self {
        Self {
            provider: ProviderConfig::from_env(),
            log_dir: PathBuf::from(std::env::var("LOG_DIR").unwrap_or_else(|_| "logs".to_string())),
            fallback_enabled: std::env::var("FALLBACK_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                == "true",
        }
    }
}

impl AgentAppConfig {
    /// Load configuration from the environment for compatibility with the binary.
    pub fn from_env() -> Self {
        Self::default()
    }
}

/// Build the GeoAgent router without binding a network socket.
///
/// This boundary keeps route and state assembly available to integration tests and
/// alternative hosts while run owns only process startup and listener lifetime.
pub fn build_app() -> Result<Router, Box<dyn std::error::Error + Send + Sync>> {
    build_app_with_config(AgentAppConfig::default())
}

/// Build the GeoAgent router from explicit runtime configuration.
pub fn build_app_with_config(
    config: AgentAppConfig,
) -> Result<Router, Box<dyn std::error::Error + Send + Sync>> {
    let tools_json = include_str!("../tools_schema.json");
    let tool_registry = ToolRegistry::from_json(tools_json).map(Arc::new)?;
    let keywords_yaml = include_str!("../keywords.yaml");
    let keyword_router = KeywordRouter::from_yaml(keywords_yaml)?;
    let metrics = MetricsStore::new(config.log_dir.clone());
    let agent = Agent::new(config.provider, tool_registry.clone());
    let state = Arc::new(AppState {
        agent,
        fallback: keyword_router,
        fallback_enabled: config.fallback_enabled,
        metrics,
        tools: tool_registry,
        log_dir: config.log_dir,
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
    if !req.force_fallback && state.fallback_enabled {
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
    use super::{build_app_with_config, AgentAppConfig};
    use crate::providers::ProviderConfig;
    use axum::{body::Body, http::Request};
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };
    use tower::ServiceExt;

    fn test_config() -> AgentAppConfig {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        AgentAppConfig {
            provider: ProviderConfig {
                provider: "test".to_string(),
                api_key: String::new(),
                base_url: "http://127.0.0.1".to_string(),
                model: "test-model".to_string(),
                timeout_seconds: 1,
            },
            log_dir: std::env::temp_dir()
                .join(format!("geo-agent-test-{}-{nonce}", std::process::id())),
            fallback_enabled: false,
        }
    }

    #[tokio::test]
    async fn health_route_is_available_without_binding_a_socket() {
        let config = test_config();
        let log_dir = config.log_dir.clone();
        let app = build_app_with_config(config).expect("test Agent configuration should load");
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
        fs::remove_dir_all(log_dir).unwrap();
    }

    #[tokio::test]
    async fn forced_fallback_bypasses_disabled_provider_routing() {
        let config = test_config();
        let log_dir = config.log_dir.clone();
        let app = build_app_with_config(config).expect("test Agent configuration should load");
        let request = Request::builder()
            .method("POST")
            .uri("/agent")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"query":"calculate NDVI for this area","force_fallback":true}"#,
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["fallback"], true);
        assert!(!payload["tool_calls"].as_array().unwrap().is_empty());
        fs::remove_dir_all(log_dir).unwrap();
    }

    #[tokio::test]
    async fn metrics_route_reports_empty_usage_for_a_new_app() {
        let config = test_config();
        let log_dir = config.log_dir.clone();
        let app = build_app_with_config(config).expect("test Agent configuration should load");
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["calls"], 0);
        assert_eq!(payload["total_tokens"], 0);
        assert_eq!(payload["by_provider"], serde_json::json!([]));
        fs::remove_dir_all(log_dir).unwrap();
    }
}
