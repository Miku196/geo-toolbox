//! Tool registration — Carbon math engine.
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "carbon-math", "Pure-Rust IPCC Tier 1 carbon accounting engine", PluginCategory::Carbon, [
        sync "carbon_calculate_raw" => "Calculate carbon emissions from GeoJSON features + CSV factors" ; serde_json::json!({"type":"object","properties":{"geojson":{"type":"string"},"csv":{"type":"string"},"year":{"type":"integer"}},"required":["geojson","csv","year"]}) => |args| -> ToolResult {
        let geojson = args["geojson"].as_str().unwrap_or("");
        let csv = args["csv"].as_str().unwrap_or("");
        let year = args["year"].as_u64().unwrap_or(2025) as u16;
        let factors = crate::load_factors_from_csv(csv).map_err(geo_core::GeoError::Validation)?;
        let engine = crate::CarbonEngine::new();
        let fc: serde_json::Value = serde_json::from_str(geojson).map_err(geo_core::GeoError::Serde)?;
        let features: Vec<crate::GeoFeature> = fc["features"].as_array().unwrap_or(&vec![]).iter()
            .filter_map(|f| { let s = serde_json::to_string(f).ok()?; crate::GeoFeature::from_feature_json(&s).ok() }).collect();
        let report = engine.calculate(&features, &factors, year).map_err(geo_core::GeoError::Validation)?;
        serde_json::to_value(report).map_err(geo_core::GeoError::Serde)
    }]);
}
