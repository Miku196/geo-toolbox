//! 插件注册表。

use geo_core::errors::GeoResult;
use geo_core::plugin::PluginMeta;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 已注册的工具定义（用于 MCP tools/list）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// 工具执行结果。
pub type ToolResult = GeoResult<serde_json::Value>;

/// 插件注册与调度中心。
///
/// 管理已注册的插件元数据和工具列表。
/// 实际执行由 CLI 层路由到具体 crate。
pub struct PluginRegistry {
    /// 已注册的插件/适配器元数据。
    plugins: Vec<PluginMeta>,
    /// 所有已注册的 MCP 工具。
    tools: Vec<ToolDef>,
    /// 插件名 → 工具列表映射。
    plugin_tools: HashMap<String, Vec<String>>,
}

impl PluginRegistry {
    /// 创建空注册表。
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            tools: Vec::new(),
            plugin_tools: HashMap::new(),
        }
    }

    /// 注册一个插件/适配器。
    pub fn register(&mut self, meta: PluginMeta) {
        self.plugin_tools.entry(meta.name.clone()).or_default();
        self.plugins.push(meta);
    }

    /// 注册一个工具（挂载到指定插件名下）。
    pub fn register_tool(&mut self, plugin_name: &str, tool: ToolDef) {
        self.plugin_tools
            .entry(plugin_name.to_string())
            .or_default()
            .push(tool.name.clone());
        self.tools.push(tool);
    }

    /// 批量注册工具的便捷方法。
    pub fn register_tools(&mut self, plugin_name: &str, tools: Vec<ToolDef>) {
        for tool in tools {
            self.register_tool(plugin_name, tool);
        }
    }

    /// 列出所有已注册插件。
    pub fn list_plugins(&self) -> &[PluginMeta] {
        &self.plugins
    }

    /// 列出所有工具。
    pub fn list_tools(&self) -> &[ToolDef] {
        &self.tools
    }

    /// 生成 MCP tools/list JSON。
    pub fn generate_mcp_tools(&self) -> serde_json::Value {
        let tools: Vec<serde_json::Value> = self.tools.iter().map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
                "inputSchema": t.input_schema,
            })
        }).collect();

        serde_json::json!({
            "jsonrpc": "2.0",
            "result": { "tools": tools }
        })
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_plugin() {
        let mut reg = PluginRegistry::new();
        let meta = PluginMeta {
            name: "test".into(),
            version: "0.1.0".into(),
            description: "test plugin".into(),
            category: geo_core::plugin::PluginCategory::Carbon,
            healthy: true,
            extra: serde_json::json!({}),
        };
        reg.register(meta.clone());
        assert_eq!(reg.list_plugins().len(), 1);
    }

    #[test]
    fn test_register_tool_and_generate_mcp() {
        let mut reg = PluginRegistry::new();

        reg.register(PluginMeta {
            name: "carbon".into(),
            version: "0.1.0".into(),
            description: "Carbon accounting".into(),
            category: geo_core::plugin::PluginCategory::Carbon,
            healthy: true,
            extra: serde_json::json!({}),
        });

        reg.register_tool("carbon", ToolDef {
            name: "carbon_calculate".into(),
            description: "Calculate carbon emissions".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "aoi": { "type": "string" },
                    "year": { "type": "integer" }
                },
                "required": ["aoi", "year"]
            }),
        });

        let tools_json = reg.generate_mcp_tools();
        let tools = tools_json["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "carbon_calculate");
    }
}
