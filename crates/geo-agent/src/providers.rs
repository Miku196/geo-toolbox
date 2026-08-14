use serde::{Deserialize, Serialize};

/// LLM request abstraction (OpenAI Chat Completions format)
#[derive(Debug, Clone, Serialize)]
pub struct LlmRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Parsed tool call from LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool: String,
    pub params: serde_json::Value,
}

/// Provider configuration
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub provider: String,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub timeout_seconds: u64,
}

impl ProviderConfig {
    pub fn from_env() -> Self {
        let provider = std::env::var("AI_PROVIDER").unwrap_or_else(|_| "openai".to_string());

        let (api_key, base_url, model) = match provider.as_str() {
            "claude" => (
                std::env::var("CLAUDE_API_KEY").unwrap_or_default(),
                std::env::var("CLAUDE_BASE_URL")
                    .unwrap_or_else(|_| "https://api.anthropic.com/v1".to_string()),
                std::env::var("CLAUDE_MODEL")
                    .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string()),
            ),
            "deepseek" => (
                std::env::var("DEEPSEEK_API_KEY").unwrap_or_default(),
                std::env::var("DEEPSEEK_BASE_URL")
                    .unwrap_or_else(|_| "https://api.deepseek.com/v1".to_string()),
                std::env::var("DEEPSEEK_MODEL")
                    .unwrap_or_else(|_| "deepseek-chat".to_string()),
            ),
            _ => (
                // openai (default)
                std::env::var("OPENAI_API_KEY").unwrap_or_default(),
                std::env::var("OPENAI_BASE_URL")
                    .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
                std::env::var("OPENAI_MODEL")
                    .unwrap_or_else(|_| "gpt-4o".to_string()),
            ),
        };

        let timeout = std::env::var("AI_TIMEOUT_SECONDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);

        ProviderConfig {
            provider,
            api_key,
            base_url,
            model,
            timeout_seconds: timeout,
        }
    }
}

/// Token usage from API response
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

/// Result from LLM call
#[derive(Debug, Clone)]
pub struct LlmResult {
    pub tool_calls: Vec<ToolCall>,
    pub usage: TokenUsage,
    pub provider: String,
    pub model: String,
    #[allow(dead_code)]
    pub fallback: bool,
}
