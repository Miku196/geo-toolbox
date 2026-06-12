//! Tool registration for PluginRegistry.
//!
//! Provides `register_tools()` — called by CLI/MCP entry points
//! to register all tool handlers from this adapter into the Registry.
//! Extracted from build_registry() to give the adapter locality over its tools.

use geo_core::errors::GeoResult;
use geo_core::plugin::{ExternalAdapter, Plugin, PluginCategory};
use geo_registry::PluginRegistry;
use geo_registry::registry::{ToolDef, ToolResult};

use crate::adapter::PostgisAdapter;
use crate::{PostgisCarbonEngine, PostgisStore, run_migrations, dvc_hash, dvc_snapshot};

/// Register all PostGIS adapter tools into the PluginRegistry.
///
/// Registers the PostgisAdapter struct itself (for health_check, identity),
/// plus tool handlers for: store_migrate, store_query, dvc_snapshot, dvc_hash,
/// carbon_calculate, carbon_import_factors.
///
/// Called from CLI `build_registry()` (feature-gated behind `#[cfg(feature = "postgis")]`).
pub fn register_tools(registry: &mut PluginRegistry) -> GeoResult<()> {
    // ── Adapter identity + health check ──
    let db_url = std::env::var("DATABASE_URL").unwrap_or_default();
    let adapter = PostgisAdapter::new(&db_url);

    let healthy = if !db_url.is_empty() {
        // Do a real health check at registration time
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async { adapter.health_check().await }).unwrap_or(false)
    } else {
        false
    };

    registry.register(geo_core::plugin::PluginMeta {
        name: adapter.name().to_string(),
        version: adapter.version().to_string(),
        description: adapter.description().to_string(),
        category: PluginCategory::Adapter,
        healthy,
        extra: serde_json::json!({"endpoint": adapter.external_endpoint()}),
    });

    // ── DVC tools (sync, no DB needed) ──
    registry.register_tool_sync("postgis", ToolDef {
        name: "dvc_snapshot".into(),
        description: "Run DVC add + push on a file for version tracking".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"file":{"type":"string"}},"required":["file"]}),
    }, |args| -> ToolResult {
        let file = args["file"].as_str().unwrap_or("");
        let snap = dvc_snapshot(file).map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(serde_json::json!({"file": snap.file, "dvc_hash": snap.dvc_hash}))
    });

    registry.register_tool_sync("postgis", ToolDef {
        name: "dvc_hash".into(),
        description: "Get the DVC MD5 hash of a tracked file".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"file":{"type":"string"}},"required":["file"]}),
    }, |args| -> ToolResult {
        let file = args["file"].as_str().unwrap_or("");
        let hash = dvc_hash(file).map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(serde_json::json!({"dvc_hash": hash}))
    });

    // ── Store tools (async, needs DATABASE_URL) ──
    let db_url_for_store = db_url.clone();
    if !db_url_for_store.is_empty() {
        registry.register_tool_async("postgis", ToolDef {
            name: "store_migrate".into(),
            description: "Run PostGIS database migrations".into(),
            input_schema: serde_json::json!({"type":"object","properties":{},"required":[]}),
        }, move |_args| {
            let url = db_url_for_store.clone();
            Box::pin(async move {
                let store = PostgisStore::connect(&url).await
                    .map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
                run_migrations(store.pool()).await
                    .map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
                Ok(serde_json::json!("Migrations applied successfully"))
            })
        });

        let db_url_for_query = db_url.clone();
        registry.register_tool_async("postgis", ToolDef {
            name: "store_query".into(),
            description: "Execute a SQL query and return results as JSON".into(),
            input_schema: serde_json::json!({"type":"object","properties":{"sql":{"type":"string"}},"required":["sql"]}),
        }, move |args| {
            let url = db_url_for_query.clone();
            Box::pin(async move {
                let store = PostgisStore::connect(&url).await
                    .map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
                let sql = args["sql"].as_str().unwrap_or("SELECT 1");
                store.query_json(sql).await
                    .map_err(|e| geo_core::GeoError::Database(e.to_string()))
                    .map(|rows| serde_json::json!(rows))
            })
        });

        // ── Carbon tools ──
        let db_url_for_carbon = db_url.clone();
        registry.register_tool_async("postgis", ToolDef {
            name: "carbon_calculate".into(),
            description: "Calculate carbon emissions using emission factor method".into(),
            input_schema: serde_json::json!({"type":"object","properties":{"aoi_id":{"type":"string"},"year":{"type":"integer"},"source":{"type":"string","default":"IPCC_2019"}},"required":["aoi_id","year"]}),
        }, move |args| {
            let url = db_url_for_carbon.clone();
            Box::pin(async move {
                let aoi = args["aoi_id"].as_str().unwrap_or("");
                let year = args["year"].as_u64().unwrap_or(2025) as u16;
                let source = args["source"].as_str().unwrap_or("IPCC_2019");
                let aoi_id = uuid::Uuid::parse_str(aoi)
                    .map_err(|e| geo_core::GeoError::invalid_input("aoi_id", format!("invalid UUID: {e}")))?;
                let pool = sqlx::postgres::PgPoolOptions::new().max_connections(2).connect(&url).await
                    .map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
                let engine = PostgisCarbonEngine::new(pool);
                let results = engine.calculate_emission_factor(aoi_id, year, source).await
                    .map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
                let total: f64 = results.iter().map(|r| r.emission_tco2e).sum();
                let summary: Vec<serde_json::Value> = results.iter()
                    .map(|r| serde_json::json!({"landcover_class":r.landcover_class,"area_ha":r.area_ha,"emission_tco2e":r.emission_tco2e}))
                    .collect();
                Ok(serde_json::json!({"aoi_id":aoi,"year":year,"total_tco2e":total,"results":summary}))
            })
        });

        let db_url_for_import = db_url.clone();
        registry.register_tool_async("postgis", ToolDef {
            name: "carbon_import_factors".into(),
            description: "Import emission factors from a CSV file".into(),
            input_schema: serde_json::json!({"type":"object","properties":{"csv_path":{"type":"string"}},"required":["csv_path"]}),
        }, move |args| {
            let url = db_url_for_import.clone();
            Box::pin(async move {
                let csv_path = args["csv_path"].as_str().unwrap_or("");
                let pool = sqlx::postgres::PgPoolOptions::new().max_connections(2).connect(&url).await
                    .map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
                let engine = PostgisCarbonEngine::new(pool);
                let count = engine.import_factors_csv(csv_path).await
                    .map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
                Ok(serde_json::json!({"imported":count,"file":csv_path}))
            })
        });
    }

    Ok(())
}
