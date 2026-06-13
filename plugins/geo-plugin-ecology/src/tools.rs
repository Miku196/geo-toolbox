//! Tool registration for PluginRegistry — Ecology assessment tool.
//!
//! Calls EcologyPlugin through its ProcessPlugin trait interface,
//! not through hardcoded struct methods.

use geo_core::plugin::{Plugin, ProcessPlugin};
use geo_registry::registry::ToolDef;
use geo_registry::PluginRegistry;

/// Register ecology tools into the PluginRegistry.
pub fn register_tools(registry: &mut PluginRegistry) {
    // Try rules.toml from common locations; fall back to default config
    let config_paths = [
        std::path::PathBuf::from("plugins/geo-plugin-ecology/rules.toml"),
        std::path::PathBuf::from("../../plugins/geo-plugin-ecology/rules.toml"),
    ];
    let plugin = config_paths
        .iter()
        .find_map(|p| crate::EcologyPlugin::load_from_file(p).ok())
        .unwrap_or_else(|| crate::EcologyPlugin::new(Default::default()));

    registry.register(geo_core::plugin::PluginMeta {
        name: plugin.name().to_string(),
        version: plugin.version().to_string(),
        description: plugin.description().to_string(),
        category: plugin.category(),
        healthy: plugin.is_healthy(),
        extra: serde_json::json!({}),
    });

    registry.register_tool_async(
        "ecology",
        ToolDef {
            name: "ecology_assess".into(),
            description: plugin.description().to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "aoi_name": {"type": "string"},
                    "baseline_year": {"type": "integer"},
                    "assessment_year": {"type": "integer"},
                    "aoi_geojson": {"type": "string"},
                    "config_path": {"type": "string"},
                },
                "required": ["aoi_name", "baseline_year", "assessment_year"],
            }),
        },
        {
            let plugin = std::sync::Arc::new(plugin);
            move |args| {
                let plugin = std::sync::Arc::clone(&plugin);
                Box::pin(async move { plugin.execute(args).await })
            }
        },
    );
}
