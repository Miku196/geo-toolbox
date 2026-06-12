//! Tool registration — Carbon math engine.
use geo_core::plugin::PluginCategory;
use geo_registry::PluginRegistry;
use geo_registry::registry::{ToolDef, ToolResult};

/// Register carbon-math tools into the PluginRegistry.
pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta {
        name: "carbon-math".into(), version: env!("CARGO_PKG_VERSION").into(),
        description: "Pure-Rust IPCC Tier 1 carbon accounting engine".into(),
        category: PluginCategory::Carbon, healthy: true, extra: serde_json::json!({}),
    });
    registry.register_tool_sync("carbon-math", ToolDef {
        name: "carbon_calculate_raw".into(),
        description: "Calculate carbon emissions from GeoJSON features + CSV factors".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"geojson":{"type":"string"},"csv":{"type":"string"},"year":{"type":"integer"}},"required":["geojson","csv","year"]}),
    }, |args| -> ToolResult {
        let geojson = args["geojson"].as_str().unwrap_or("");
        let csv = args["csv"].as_str().unwrap_or("");
        let year = args["year"].as_u64().unwrap_or(2025) as u16;
        let factors = crate::load_factors_from_csv(csv).map_err(|e| geo_core::GeoError::Validation(e))?;
        let engine = crate::CarbonEngine::new();
        let fc: serde_json::Value = serde_json::from_str(geojson).map_err(geo_core::GeoError::Serde)?;
        let features: Vec<crate::GeoFeature> = fc["features"].as_array().unwrap_or(&vec![]).iter()
            .filter_map(|f| { let s = serde_json::to_string(f).ok()?; crate::GeoFeature::from_feature_json(&s).ok() }).collect();
        let report = engine.calculate(&features, &factors, year).map_err(|e| geo_core::GeoError::Validation(e))?;
        Ok(serde_json::to_value(report).map_err(geo_core::GeoError::Serde)?)
    });
}
