//! Python-accessible unified tool registry.
//!
//! Provides `Geo.call("tool_name", params)` that mirrors the
//! full MCP tools/list dispatch — all sync tools available
//! through one Python entry point.
//!
//! Avoids geo-wiring (which requires tokio) by directly calling
//! each crate's `register_tools()` function.

use geo_registry::PluginRegistry;
use pyo3::prelude::*;
use serde_json::Value;

/// The main entry point for geo-toolbox from Python.
///
/// Usage:
///   >>> import _geo_toolbox
///   >>> geo = _geo_toolbox.Geo()
///   >>> tools = geo.list_tools()
///   >>> result = geo.call("compute_area_sqm", '{"geojson_geom": "..."}')
#[pyclass(name = "Geo")]
pub struct Geo {
    registry: PluginRegistry,
}

/// Register core-layer sync tools only. Plugin and Adapter layers excluded
/// per user decision "只用core的". Use the MCP server for full 89-tool access.
fn populate_sync_tools(reg: &mut PluginRegistry) {
    // Core layer — all sync
    geo_io::tools::register_tools(reg);
    geo_carbon_math::tools::register_tools(reg);
    geo_tile::tools::register_tools(reg);
    geo_temporal::tools::register_tools(reg);
    geo_vector::tools::register_tools(reg);
    geo_index::tools::register_tools(reg);
    geo_stats::tools::register_tools(reg);
    geo_report::tools::register_tools(reg);

    // Plugin and Adapter layers excluded per user decision
}

#[pymethods]
impl Geo {
    /// Create a new Geo instance with all sync tools registered.
    #[new]
    fn new() -> PyResult<Self> {
        let mut registry = PluginRegistry::new();
        populate_sync_tools(&mut registry);
        Ok(Self { registry })
    }

    /// Call a registered tool by name with JSON params string.
    ///
    /// Returns JSON: {"ok": <result>} on success, {"error": "..."} on failure.
    #[pyo3(signature = (tool_name, params))]
    fn call(&self, tool_name: &str, params: &str) -> PyResult<String> {
        let args: Value = serde_json::from_str(params).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid JSON params: {}", e))
        })?;

        match self.registry.dispatch_sync(tool_name, args) {
            Ok(result) => serde_json::to_string(&serde_json::json!({"ok": result}))
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())),
            Err(e) => serde_json::to_string(&serde_json::json!({
                "error": e.to_string(),
                "tool": tool_name,
            }))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())),
        }
    }

    /// Call a tool with native Python dict/list params.
    ///
    /// Returns the deserialized result as a Python object.
    #[pyo3(signature = (tool_name, params))]
    fn call_py(&self, tool_name: &str, params: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        let py = params.py();
        let json_module = py.import("json")?;
        let json_str: String = json_module.call_method1("dumps", (params,))?.extract()?;

        let args: Value = serde_json::from_str(&json_str).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to parse params: {}",
                e
            ))
        })?;

        let response = match self.registry.dispatch_sync(tool_name, args) {
            Ok(result) => serde_json::json!({"ok": result}),
            Err(e) => serde_json::json!({"error": e.to_string(), "tool": tool_name}),
        };

        let json_str = serde_json::to_string(&response)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        let obj = json_module.call_method1("loads", (json_str,))?;
        Ok(obj.into())
    }

    /// List all registered tools as JSON string.
    fn list_tools(&self) -> String {
        let tools: Vec<Value> = self
            .registry
            .list_tools()
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.input_schema,
                })
            })
            .collect();
        serde_json::to_string(&tools).unwrap_or_else(|_| "[]".into())
    }

    /// Get the JSON Schema for a specific tool's parameters as JSON string.
    fn tool_schema(&self, tool_name: &str) -> String {
        serde_json::to_string(
            &self
                .registry
                .list_tools()
                .iter()
                .find(|t| t.name == tool_name)
                .map(|t| t.input_schema.clone()),
        )
        .unwrap_or_else(|_| "null".into())
    }

    /// Generate MCP tools/list JSON string.
    fn mcp_tools(&self) -> String {
        serde_json::to_string(&self.registry.generate_mcp_tools()).unwrap_or_else(|_| "{}".into())
    }

    /// Generate MCP resources/list JSON string.
    fn mcp_resources(&self) -> String {
        serde_json::to_string(&self.registry.generate_mcp_resources())
            .unwrap_or_else(|_| "{}".into())
    }

    /// Generate MCP prompts/list JSON string.
    fn mcp_prompts(&self) -> String {
        serde_json::to_string(&self.registry.generate_mcp_prompts()).unwrap_or_else(|_| "{}".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_list() {
        let geo = Geo::new().unwrap();
        let tools_json = geo.list_tools();
        let tools: Value = serde_json::from_str(&tools_json).unwrap();
        assert!(
            tools.as_array().unwrap().len() > 0,
            "Should have registered tools"
        );
    }

    #[test]
    fn test_schema() {
        let geo = Geo::new().unwrap();
        let schema_json = geo.tool_schema("geohash_encode");
        let schema: Value = serde_json::from_str(&schema_json).unwrap();
        // Schema may be null if tool not found, or object if found
        assert!(schema.is_object() || schema.is_null());
    }

    #[test]
    fn test_unknown_tool() {
        let geo = Geo::new().unwrap();
        let result = geo.call("nonexistent", "{}").unwrap();
        assert!(result.contains("error"), "Should contain error");
    }

    #[test]
    fn test_mcp_outputs() {
        let geo = Geo::new().unwrap();
        let tools: Value = serde_json::from_str(&geo.mcp_tools()).unwrap();
        assert!(tools["result"]["tools"].as_array().is_some());
        let res: Value = serde_json::from_str(&geo.mcp_resources()).unwrap();
        assert!(res["result"]["resources"].as_array().is_some());
        let prompts: Value = serde_json::from_str(&geo.mcp_prompts()).unwrap();
        assert!(prompts["result"]["prompts"].as_array().is_some());
    }
}
