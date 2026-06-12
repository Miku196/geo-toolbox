//! Tool registration — Time series.
use geo_core::plugin::PluginCategory;
use geo_registry::PluginRegistry;
use geo_registry::registry::{ToolDef, ToolResult};

/// Register temporal tools into the PluginRegistry.
pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta {
        name: "temporal".into(), version: env!("CARGO_PKG_VERSION").into(),
        description: "Time series trend analysis (Mann-Kendall + linear regression)".into(),
        category: PluginCategory::Process, healthy: true, extra: serde_json::json!({}),
    });
    registry.register_tool_sync("temporal", ToolDef {
        name: "temporal_trend".into(), description: "Compute Mann-Kendall trend + linear slope".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"values":{"type":"array","items":{"type":"number"}}},"required":["values"]}),
    }, |args| -> ToolResult {
        let values: Vec<f64> = args["values"].as_array().unwrap_or(&vec![]).iter().filter_map(|v| v.as_f64()).collect();
        if values.len() < 3 { return Err(geo_core::GeoError::invalid_input("values","need >=3 values")); }
        let trend = crate::linear_trend(&values);
        let (tau, p_value) = crate::mann_kendall(&values);
        Ok(serde_json::json!({"count":values.len(),"sen_slope":trend.sen_slope,"intercept":trend.ols_intercept,"significant":trend.significant,"mann_kendall_tau":tau,"p_value":p_value}))
    });
}
