use crate::providers::{LlmRequest, LlmResult, Message, ProviderConfig, TokenUsage, ToolCall};
use crate::schema::ToolRegistry;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

/// Core agent: sends user query + tools to LLM, returns parsed tool calls
pub struct Agent {
    config: ProviderConfig,
    tools: Arc<ToolRegistry>,
    client: Client,
}

impl Agent {
    pub fn new(config: ProviderConfig, tools: Arc<ToolRegistry>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to build HTTP client");

        Agent {
            config,
            tools,
            client,
        }
    }

    /// Send query to LLM and get tool_calls back.
    /// Returns Ok(None) if LLM returned no tool_calls (plain text response).
    pub async fn route(&self, query: &str) -> Result<LlmResult, AgentError> {
        let system_prompt = format!(
            "You are a geo-toolbox function-calling assistant. \
             You have {} geospatial analysis tools available. \
             Given a user's request in natural language, select the most appropriate tool(s) \
             and provide the required parameters. \
             If multiple tools are needed, return all relevant tool calls. \
             Always fill in reasonable defaults for optional parameters when the user doesn't specify them.",
            self.tools.len()
        );

        let request = LlmRequest {
            model: self.config.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: system_prompt,
                },
                Message {
                    role: "user".to_string(),
                    content: query.to_string(),
                },
            ],
            tools: crate::schema::to_openai_tools(&self.tools),
            tool_choice: Some("auto".to_string()),
            temperature: Some(0.1),
            max_tokens: Some(1024),
        };

        match self.config.provider.as_str() {
            "claude" => self.call_claude(&request).await,
            "deepseek" => self.call_openai_compatible(&request).await,
            _ => self.call_openai_compatible(&request).await,
        }
    }

    /// Call OpenAI-compatible API (OpenAI, DeepSeek, etc.)
    async fn call_openai_compatible(&self, request: &LlmRequest) -> Result<LlmResult, AgentError> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() || e.is_connect() {
                    AgentError::NetworkUnreachable(e.to_string())
                } else {
                    AgentError::Http(e.to_string())
                }
            })?;

        let body: Value = response.json().await.map_err(|e| AgentError::Http(e.to_string()))?;

        // Extract usage
        let usage = body
            .get("usage")
            .map(|u| TokenUsage {
                prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0),
                completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0),
                total_tokens: u["total_tokens"].as_u64().unwrap_or(0),
            })
            .unwrap_or_default();

        // Extract tool_calls
        let choices = body["choices"].as_array().ok_or_else(|| {
            AgentError::UnexpectedResponse("No choices in response".to_string())
        })?;

        let mut tool_calls = Vec::new();

        for choice in choices {
            if let Some(tc_array) = choice["message"]["tool_calls"].as_array() {
                for tc in tc_array {
                    let function = &tc["function"];
                    let tool_name = function["name"].as_str().unwrap_or("").to_string();

                    let params: Value = function["arguments"]
                        .as_str()
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or(serde_json::json!({}));

                    tool_calls.push(ToolCall {
                        tool: tool_name,
                        params,
                    });
                }
            }
        }

        info!(
            provider = %self.config.provider,
            model = %self.config.model,
            tool_calls = tool_calls.len(),
            prompt_tokens = usage.prompt_tokens,
            completion_tokens = usage.completion_tokens,
            "LLM call complete"
        );

        Ok(LlmResult {
            tool_calls,
            usage,
            provider: self.config.provider.clone(),
            model: self.config.model.clone(),
            fallback: false,
        })
    }

    /// Call Claude API (Anthropic Messages format)
    async fn call_claude(&self, request: &LlmRequest) -> Result<LlmResult, AgentError> {
        let url = format!("{}/messages", self.config.base_url);

        // Convert OpenAI format messages to Claude format
        let messages: Vec<Value> = request
            .messages
            .iter()
            .filter(|m| m.role != "system") // Claude handles system separately
            .map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content
                })
            })
            .collect();

        let system_msg = request
            .messages
            .iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.clone());

        let claude_tools = crate::schema::to_claude_tools(&self.tools);

        let mut body = serde_json::json!({
            "model": request.model,
            "max_tokens": request.max_tokens.unwrap_or(1024),
            "messages": messages,
            "tools": claude_tools,
        });

        if let Some(s) = system_msg {
            body["system"] = Value::String(s);
        }

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() || e.is_connect() {
                    AgentError::NetworkUnreachable(e.to_string())
                } else {
                    AgentError::Http(e.to_string())
                }
            })?;

        let resp_body: Value =
            response.json().await.map_err(|e| AgentError::Http(e.to_string()))?;

        let mut tool_calls = Vec::new();

        for content_block in resp_body["content"].as_array().unwrap_or(&vec![]) {
            if content_block["type"].as_str() == Some("tool_use") {
                tool_calls.push(ToolCall {
                    tool: content_block["name"].as_str().unwrap_or("").to_string(),
                    params: content_block["input"].clone(),
                });
            }
        }

        let usage = resp_body
            .get("usage")
            .map(|u| TokenUsage {
                prompt_tokens: u["input_tokens"].as_u64().unwrap_or(0),
                completion_tokens: u["output_tokens"].as_u64().unwrap_or(0),
                total_tokens: u["input_tokens"].as_u64().unwrap_or(0)
                    + u["output_tokens"].as_u64().unwrap_or(0),
            })
            .unwrap_or_default();

        info!(
            provider = "claude",
            model = %request.model,
            tool_calls = tool_calls.len(),
            prompt_tokens = usage.prompt_tokens,
            "Claude call complete"
        );

        Ok(LlmResult {
            tool_calls,
            usage,
            provider: "claude".to_string(),
            model: request.model.clone(),
            fallback: false,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Network unreachable: {0}")]
    NetworkUnreachable(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Unexpected response: {0}")]
    UnexpectedResponse(String),
}
