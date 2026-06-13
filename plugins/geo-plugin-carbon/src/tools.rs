//! Tool registration — Carbon plugin.
use crate::{CarbonConfig, CarbonPlugin as Cp};
use geo_core::plugin::PluginCategory;
use geo_registry::registry::{ToolDef, ToolResult};
use geo_registry::PluginRegistry;
pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta {
        name: "carbon".into(),
        version: "0.1.0".into(),
        description: "IPCC Tier 1 carbon accounting".into(),
        category: PluginCategory::Carbon,
        healthy: true,
        extra: serde_json::json!({}),
    });
    registry.register_tool_sync("carbon", ToolDef { name: "carbon_calculate_geojson".into(), description: "Calculate carbon from GeoJSON FeatureCollection".into(), input_schema: serde_json::json!({"type":"object","properties":{"geojson":{"type":"string"},"year":{"type":"integer"}},"required":["geojson","year"]}) }, |args| -> ToolResult {
        let plugin = Cp::load(CarbonConfig::default());
        let report = plugin.calculate_from_geojson(args["geojson"].as_str().unwrap_or(""), args["year"].as_u64().unwrap_or(2025) as u16)?;
        Ok(serde_json::to_value(report).map_err(geo_core::GeoError::Serde)?)
    });
}
