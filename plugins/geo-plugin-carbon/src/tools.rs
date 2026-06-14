//! Tool registration — Carbon plugin.
use crate::{CarbonConfig, CarbonPlugin as Cp};
use geo_core::plugin::PluginCategory;
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "carbon", "IPCC Tier 1 carbon accounting", PluginCategory::Carbon, [
        sync "carbon_calculate_geojson" => "Calculate carbon from GeoJSON FeatureCollection" ; serde_json::json!({"type":"object","properties":{"geojson":{"type":"string"},"year":{"type":"integer"}},"required":["geojson","year"]}) => |args| -> ToolResult {
        let plugin = Cp::load(CarbonConfig::default());
        let report = plugin.calculate_from_geojson(args["geojson"].as_str().unwrap_or(""), args["year"].as_u64().unwrap_or(2025) as u16)?;
        Ok(serde_json::to_value(report).map_err(geo_core::GeoError::Serde)?)
    }]);
}
