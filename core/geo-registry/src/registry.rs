//! 插件注册表 —— 统一管理工具注册、发现与执行分发。

use geo_core::errors::{GeoError, GeoResult};
use geo_core::plugin::PluginMeta;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(feature = "tokio")]
use std::future::Future;
#[cfg(feature = "tokio")]
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

/// 异步工具处理器。仅在 `tokio` feature 启用时可用。
#[cfg(feature = "tokio")]
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
    #[cfg(feature = "tokio")]
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
            #[cfg(feature = "tokio")]
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
    #[cfg(feature = "tokio")]
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
        #[cfg(feature = "tokio")]
        if self.async_handlers.contains_key(tool_name) {
            return Err(GeoError::Other(format!(
                "Tool '{tool_name}' is async; use dispatch()"
            )));
        }
        Err(GeoError::Other(format!("Unknown tool: {tool_name}")))
    }

    /// 分发执行一个工具调用。仅 tokio feature 启用时可用。
    #[cfg(feature = "tokio")]
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

    /// 生成 MCP resources/list 响应。
    pub fn generate_mcp_resources(&self) -> serde_json::Value {
        let resources: Vec<serde_json::Value> = vec![
            serde_json::json!({"uri": "geo://datasets/emission-factors", "name": "Emission Factors (IPCC 2019)", "description": "IPCC Tier 1 emission factors with Chinese provincial parameters", "mimeType": "text/csv"}),
            serde_json::json!({"uri": "geo://datasets/carbon-pools", "name": "Carbon Pool Defaults", "description": "Default carbon pool values per ecosystem zone (AGB/BGB/Deadwood/Litter/SOC)", "mimeType": "application/json"}),
            serde_json::json!({"uri": "geo://datasets/soil-groups", "name": "SCS Hydrologic Soil Groups", "description": "NRCS hydrologic soil group classification (A/B/C/D)", "mimeType": "application/json"}),
            serde_json::json!({"uri": "geo://datasets/landcover-cn", "name": "SCS-CN Land Cover Table", "description": "Curve numbers for 26 land use types per soil group", "mimeType": "application/json"}),
            serde_json::json!({"uri": "geo://datasets/id-thresholds", "name": "Rainfall ID Thresholds", "description": "Global rainfall intensity-duration thresholds (Caine 1980, Guzzetti 2008, Hong 2016, Ma 2015)", "mimeType": "application/json"}),
            serde_json::json!({"uri": "geo://datasets/coastal-carbon", "name": "Blue Carbon Defaults", "description": "Blue carbon ecosystem defaults: mangrove, saltmarsh, seagrass (IPCC Tier 1)", "mimeType": "application/json"}),
        ];
        serde_json::json!({ "result": { "resources": resources } })
    }

    /// 生成 MCP prompts/list 响应。
    pub fn generate_mcp_prompts(&self) -> serde_json::Value {
        let prompts: Vec<serde_json::Value> = vec![
            serde_json::json!({"name": "carbon-assessment", "description": "Carbon emission/sink assessment for an area of interest", "arguments": [{"name": "aoi_name", "description": "Area of interest name", "required": true}, {"name": "year", "description": "Assessment year", "required": true}, {"name": "source", "description": "Emission factor source (default: IPCC_2019)", "required": false}]}),
            serde_json::json!({"name": "ecological-restoration", "description": "Ecological restoration assessment with NDVI change + carbon sink", "arguments": [{"name": "aoi_name", "description": "Area of interest name", "required": true}, {"name": "baseline_year", "description": "Baseline year for NDVI comparison", "required": true}, {"name": "assessment_year", "description": "Assessment year", "required": true}]}),
            serde_json::json!({"name": "flood-risk", "description": "Flood risk assessment with SCS-CN runoff + watershed analysis", "arguments": [{"name": "aoi_name", "description": "Area of interest name", "required": true}, {"name": "rainfall_mm", "description": "24-hour rainfall in mm", "required": true}]}),
            serde_json::json!({"name": "geohazard-assessment", "description": "Geohazard assessment: landslide susceptibility + FS + Newmark displacement", "arguments": [{"name": "slope_deg", "description": "Average slope in degrees", "required": true}, {"name": "cohesion_kpa", "description": "Soil cohesion in kPa", "required": true}, {"name": "friction_deg", "description": "Friction angle in degrees", "required": true}, {"name": "pga_g", "description": "Peak ground acceleration in g", "required": false}]}),
            serde_json::json!({"name": "solar-suitability", "description": "Solar energy site suitability assessment", "arguments": [{"name": "site_name", "description": "Site name", "required": true}, {"name": "annual_radiation_kwh_m2", "description": "Annual solar radiation in kWh/m²", "required": true}]}),
            serde_json::json!({"name": "forest-carbon-stock", "description": "Forest carbon stock change assessment from NDVI time series", "arguments": [{"name": "forest_name", "description": "Forest area name", "required": true}, {"name": "baseline_year", "description": "Baseline year", "required": true}, {"name": "assessment_year", "description": "Assessment year", "required": true}]}),
        ];
        serde_json::json!({ "result": { "prompts": prompts } })
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

    /// 生成全部注册工具的 JSON Schema 文档 (MCP Tool Schema 标准格式)。
    pub fn generate_tool_schemas(&self) -> serde_json::Value {
        let schemas: Vec<serde_json::Value> = self
            .tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "$schema": "https://json-schema.org/draft/2020-12/schema",
                    "title": t.name,
                    "description": t.description,
                    "type": "object",
                    "properties": t.input_schema.get("properties").cloned().unwrap_or(serde_json::json!({})),
                    "required": t.input_schema.get("required").cloned().unwrap_or(serde_json::json!([])),
                })
            })
            .collect();

        serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": "geo-toolbox MCP Tool Schemas",
            "description": "Complete JSON Schema definitions for all registered geo-toolbox tools",
            "type": "object",
            "properties": {
                "tools": {
                    "type": "array",
                    "items": { "type": "object" },
                    "description": "All registered MCP tool schemas"
                },
                "count": { "type": "integer" }
            },
            "tools": schemas,
            "count": self.tools.len()
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

    #[cfg(feature = "tokio")]
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

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_dispatch_unknown_tool() {
        let reg = PluginRegistry::new();
        let result = reg.dispatch("nonexistent", serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[cfg(feature = "tokio")]
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

    #[test]
    fn test_generate_mcp_resources() {
        let reg = PluginRegistry::new();
        let r = reg.generate_mcp_resources();
        let resources = r["result"]["resources"].as_array().unwrap();
        assert!(resources.len() >= 6);
        assert!(resources
            .iter()
            .any(|r| r["uri"] == "geo://datasets/emission-factors"));
        assert!(resources
            .iter()
            .any(|r| r["uri"] == "geo://datasets/carbon-pools"));
        assert!(resources
            .iter()
            .any(|r| r["uri"] == "geo://datasets/coastal-carbon"));
    }

    #[test]
    fn test_generate_mcp_prompts() {
        let reg = PluginRegistry::new();
        let r = reg.generate_mcp_prompts();
        let prompts = r["result"]["prompts"].as_array().unwrap();
        assert_eq!(prompts.len(), 6);
        assert!(prompts.iter().any(|p| p["name"] == "carbon-assessment"));
        assert!(prompts
            .iter()
            .any(|p| p["name"] == "ecological-restoration"));
        assert!(prompts.iter().any(|p| p["name"] == "forest-carbon-stock"));
    }

    #[test]
    fn test_generate_tool_schemas() {
        let mut reg = PluginRegistry::new();
        reg.register_tool(
            "test",
            ToolDef {
                name: "hello".into(),
                description: "Say hello".into(),
                input_schema: serde_json::json!({"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}),
            },
        );
        let r = reg.generate_tool_schemas();
        assert!(r["tools"].is_array());
        assert!(r["count"] == serde_json::json!(1));
        let tool = &r["tools"][0];
        assert_eq!(tool["title"], "hello");
        assert_eq!(tool["description"], "Say hello");
        assert_eq!(tool["type"], "object");
        assert!(tool["properties"]["name"].is_object());
    }
}
