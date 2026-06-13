//! 插件注册表 —— 统一管理工具注册、发现与执行分发。

use geo_core::errors::{GeoError, GeoResult};
use geo_core::plugin::PluginMeta;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// 已注册的工具定义（用于 MCP tools/list）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// 工具执行结果。
pub type ToolResult = GeoResult<serde_json::Value>;

/// 异步工具处理器：接收 args JSON，返回结果 JSON。
pub type AsyncHandler = Arc<
    dyn Fn(serde_json::Value) -> Pin<Box<dyn Future<Output = ToolResult> + Send>> + Send + Sync,
>;

/// 同步工具处理器。
pub type SyncHandler = Arc<dyn Fn(serde_json::Value) -> ToolResult + Send + Sync>;

/// 插件注册与调度中心。
///
/// 管理已注册的插件元数据、工具列表与执行分发。
/// MCP Server、CLI 等入口统一通过 `dispatch()` 调用。
pub struct PluginRegistry {
    /// 已注册的插件/适配器元数据。
    plugins: Vec<PluginMeta>,
    /// 所有已注册的 MCP 工具定义。
    tools: Vec<ToolDef>,
    /// 插件名 → 工具列表映射。
    plugin_tools: HashMap<String, Vec<String>>,
    /// 工具名 → 异步处理器。
    async_handlers: HashMap<String, AsyncHandler>,
    /// 工具名 → 同步处理器。
    sync_handlers: HashMap<String, SyncHandler>,
}

impl PluginRegistry {
    /// 创建空注册表。
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            tools: Vec::new(),
            plugin_tools: HashMap::new(),
            async_handlers: HashMap::new(),
            sync_handlers: HashMap::new(),
        }
    }

    /// 注册一个插件/适配器。
    pub fn register(&mut self, meta: PluginMeta) {
        self.plugin_tools.entry(meta.name.clone()).or_default();
        self.plugins.push(meta);
    }

    /// 注册一个纯元数据工具（无 handler，仅用于 listing）。
    pub fn register_tool(&mut self, plugin_name: &str, tool: ToolDef) {
        self.plugin_tools
            .entry(plugin_name.to_string())
            .or_default()
            .push(tool.name.clone());
        self.tools.push(tool);
    }

    /// 注册一个工具并绑定同步处理器。
    pub fn register_tool_sync(
        &mut self,
        plugin_name: &str,
        tool: ToolDef,
        handler: impl Fn(serde_json::Value) -> ToolResult + Send + Sync + 'static,
    ) {
        let name = tool.name.clone();
        self.register_tool(plugin_name, tool);
        self.sync_handlers.insert(name, Arc::new(handler));
    }

    /// 注册一个工具并绑定异步处理器。
    pub fn register_tool_async(
        &mut self,
        plugin_name: &str,
        tool: ToolDef,
        handler: impl Fn(serde_json::Value) -> Pin<Box<dyn Future<Output = ToolResult> + Send>>
            + Send
            + Sync
            + 'static,
    ) {
        let name = tool.name.clone();
        self.register_tool(plugin_name, tool);
        self.async_handlers.insert(name, Arc::new(handler));
    }

    /// 批量注册工具的便捷方法（纯元数据）。
    pub fn register_tools(&mut self, plugin_name: &str, tools: Vec<ToolDef>) {
        for tool in tools {
            self.register_tool(plugin_name, tool);
        }
    }

    /// 同步分发——仅匹配同步 handler，异步 handler 返回错误。
    /// 用于 CLI 同步命令（CRS 等）。
    pub fn dispatch_sync(&self, tool_name: &str, args: serde_json::Value) -> ToolResult {
        if let Some(handler) = self.sync_handlers.get(tool_name) {
            return handler(args);
        }
        if self.async_handlers.contains_key(tool_name) {
            return Err(GeoError::Other(format!(
                "Tool '{tool_name}' is async; use dispatch()"
            )));
        }
        Err(GeoError::Other(format!("Unknown tool: {tool_name}")))
    }

    /// 分发执行一个工具调用。
    ///
    /// 优先匹配异步 handler，其次同步 handler，都没有则返回错误。
    pub async fn dispatch(&self, tool_name: &str, args: serde_json::Value) -> ToolResult {
        // 异步 handler 优先
        if let Some(handler) = self.async_handlers.get(tool_name) {
            return handler(args).await;
        }
        // 同步 handler
        if let Some(handler) = self.sync_handlers.get(tool_name) {
            return handler(args);
        }
        Err(GeoError::Other(format!("Unknown tool: {tool_name}")))
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
        let tools: Vec<serde_json::Value> = self
            .tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "inputSchema": t.input_schema,
                })
            })
            .collect();

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

        reg.register_tool(
            "carbon",
            ToolDef {
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
            },
        );

        let tools_json = reg.generate_mcp_tools();
        let tools = tools_json["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "carbon_calculate");
    }

    #[tokio::test]
    async fn test_dispatch_sync_handler() {
        let mut reg = PluginRegistry::new();
        reg.register(PluginMeta {
            name: "test".into(),
            version: "0.1.0".into(),
            description: "test".into(),
            category: geo_core::plugin::PluginCategory::Process,
            healthy: true,
            extra: serde_json::json!({}),
        });
        reg.register_tool_sync(
            "test",
            ToolDef {
                name: "hello".into(),
                description: "Say hello".into(),
                input_schema: serde_json::json!({"type":"object","properties":{},"required":[]}),
            },
            |args| {
                let name = args["name"].as_str().unwrap_or("world");
                Ok(serde_json::json!({"greeting": format!("Hello, {name}!")}))
            },
        );

        let result = reg
            .dispatch("hello", serde_json::json!({"name": "test"}))
            .await
            .unwrap();
        assert_eq!(result["greeting"], "Hello, test!");
    }

    #[tokio::test]
    async fn test_dispatch_unknown_tool() {
        let reg = PluginRegistry::new();
        let result = reg.dispatch("nonexistent", serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_dispatch_async_handler() {
        let mut reg = PluginRegistry::new();
        reg.register(PluginMeta {
            name: "test".into(),
            version: "0.1.0".into(),
            description: "test".into(),
            category: geo_core::plugin::PluginCategory::Process,
            healthy: true,
            extra: serde_json::json!({}),
        });
        reg.register_tool_async(
            "test",
            ToolDef {
                name: "async_echo".into(),
                description: "Echo with async".into(),
                input_schema: serde_json::json!({"type":"object","properties":{},"required":[]}),
            },
            |args| Box::pin(async move { Ok(serde_json::json!({"echo": args})) }),
        );

        let result = reg
            .dispatch("async_echo", serde_json::json!({"x": 42}))
            .await
            .unwrap();
        assert_eq!(result["echo"]["x"], 42);
    }
}
