//! Tool registration — Report generation.
use geo_core::plugin::PluginCategory;
use geo_registry::registry::{ToolDef, ToolResult};
use geo_registry::PluginRegistry;
use std::path::PathBuf;
pub fn register_tools(registry: &mut PluginRegistry) {
    // Register plugin manually (for non-trivial metadata or if we need control)
    registry.register(geo_core::plugin::PluginMeta {
        name: "report".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        description: "Markdown/HTML report generation".into(),
        category: PluginCategory::Output,
        healthy: true,
        extra: serde_json::json!({}),
    });
    // Tool 1: report_carbon
    registry.register_tool_sync("report", ToolDef {
        name: "report_carbon".into(), description: "Generate a carbon accounting report (Markdown)".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"title":{"type":"string"},"aoi_name":{"type":"string"},"year":{"type":"integer"},"source":{"type":"string","default":"IPCC_2019"},"total_tco2e":{"type":"number"},"breakdown":{"type":"array"}},"required":["title","aoi_name","year","total_tco2e"]}),
    }, |args| -> ToolResult {
        let gen = crate::ReportGenerator::new()?;
        let bd: Vec<crate::report::LandcoverBreakdown> = args["breakdown"].as_array().unwrap_or(&vec![]).iter().map(|b| crate::report::LandcoverBreakdown{
            class:b["class"].as_str().unwrap_or("").into(), area_ha:b["area_ha"].as_f64().unwrap_or(0.0),
            factor:b["factor"].as_f64().unwrap_or(0.0), tco2e:b["tco2e"].as_f64().unwrap_or(0.0),
        }).collect();
        let data = crate::report::CarbonReportData {
            title: args["title"].as_str().unwrap_or("Report").into(),
            aoi_name: args["aoi_name"].as_str().unwrap_or("").into(),
            year: args["year"].as_u64().unwrap_or(2025) as u16,
            generated_at: chrono::Utc::now().to_rfc3339(),
            source: args["source"].as_str().unwrap_or("IPCC_2019").into(),
            total_tco2e: args["total_tco2e"].as_f64().unwrap_or(0.0),
            breakdown: bd, audit_trails: vec![],
        };
        Ok(serde_json::json!({"markdown":gen.carbon_report(&data)?}))
    });
    // Tool 2: report_render
    registry.register_tool_sync("report", ToolDef {
        name: "report_render".into(),
        description: "Render a Tera template from a plugin's templates/ directory with JSON data".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"template_dir":{"type":"string"},"template_name":{"type":"string"},"data":{"type":"object"}},"required":["template_dir","template_name","data"]}),
    }, |args| -> ToolResult {
        let template_dir = PathBuf::from(args["template_dir"].as_str().unwrap_or(""));
        let template_name = args["template_name"].as_str().unwrap_or("").to_string();
        let data = args["data"].clone();
        let mut engine = crate::ReportEngine::new()?;
        engine.register_templates("plugin", &template_dir)?;
        let output = engine.render(&template_name, &data)?;
        Ok(serde_json::json!({"output": output}))
    });
}
