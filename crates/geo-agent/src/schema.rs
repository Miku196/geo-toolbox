use serde::{Deserialize, Serialize};
use std::collections::HashMap;
/// OpenAI-compatible function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// OpenAI-compatible tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDef,
}

/// Collection of tools ready for LLM consumption
#[derive(Debug, Clone)]
pub struct ToolRegistry {
    pub tools: Vec<ToolDef>,
    /// name → description index for keyword matching
    #[allow(dead_code)]
    pub keywords: HashMap<String, Vec<String>>,
}

impl ToolRegistry {
    /// Load tools from an embedded tools_schema.json string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let wrapper: serde_json::Value = serde_json::from_str(json)?;
        let tools_array = wrapper
            .get("tools")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let tools: Vec<ToolDef> = tools_array
            .into_iter()
            .map(|v| ToolDef {
                tool_type: "function".to_string(),
                function: FunctionDef {
                    name: v["name"].as_str().unwrap_or("").to_string(),
                    description: v["description"].as_str().unwrap_or("").to_string(),
                    parameters: v["input_schema"].clone(),
                },
            })
            .collect();

        Ok(ToolRegistry {
            tools,
            keywords: HashMap::new(),
        })
    }

    /// Build from a pre-parsed Vec of tool definitions
    #[allow(dead_code)]
    pub fn from_tools(tools: Vec<ToolDef>) -> Self {
        let mut keywords: HashMap<String, Vec<String>> = HashMap::new();
        for t in &tools {
            let desc = t.function.description.to_lowercase();
            for word in desc.split_whitespace() {
                let w = word.trim_matches(|c: char| !c.is_alphanumeric());
                if w.len() > 2 {
                    keywords
                        .entry(w.to_string())
                        .or_default()
                        .push(t.function.name.clone());
                }
            }
        }
        ToolRegistry { tools, keywords }
    }

    /// Find tools matching keywords (case-insensitive)
    #[allow(dead_code)]
    pub fn search_keywords(&self, query: &str) -> Vec<String> {
        let lower = query.to_lowercase();
        let mut scores: HashMap<String, usize> = HashMap::new();

        for word in lower.split_whitespace() {
            let w = word.trim_matches(|c: char| !c.is_alphanumeric());
            if let Some(matches) = self.keywords.get(w) {
                for name in matches {
                    *scores.entry(name.clone()).or_default() += 1;
                }
            }
        }

        let mut scored: Vec<(String, usize)> = scores.into_iter().collect();
        scored.sort_by_key(|score| std::cmp::Reverse(score.1));
        scored.into_iter().map(|(name, _)| name).take(5).collect()
    }

    /// Number of tools
    pub fn len(&self) -> usize {
        self.tools.len()
    }
}

/// Wrapper for the full tools schema file
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ToolsSchema {
    pub tools: Vec<ToolSchemaEntry>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ToolSchemaEntry {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Generate OpenAI-compatible tools array from ToolRegistry
pub fn to_openai_tools(registry: &ToolRegistry) -> Vec<serde_json::Value> {
    registry
        .tools
        .iter()
        .map(|t| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": t.function.name,
                    "description": t.function.description,
                    "parameters": t.function.parameters
                }
            })
        })
        .collect()
}

/// Generate Claude-compatible tools array
pub fn to_claude_tools(registry: &ToolRegistry) -> Vec<serde_json::Value> {
    registry
        .tools
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.function.name,
                "description": t.function.description,
                "input_schema": t.function.parameters
            })
        })
        .collect()
}
