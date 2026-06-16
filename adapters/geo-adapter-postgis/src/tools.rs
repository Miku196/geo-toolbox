//! Tool registration — PostGIS adapter (mixed sync/async, requires DATABASE_URL).
use crate::adapter::PostgisAdapter;
use crate::{dvc_hash, dvc_snapshot, run_migrations, PostgisCarbonEngine, PostgisStore};
use geo_core::errors::GeoResult;
use geo_core::plugin::{ExternalAdapter, Plugin, PluginCategory};
use geo_registry::registry::ToolResult;
use geo_registry::{register_sync_tools, PluginRegistry};
pub fn register_tools(registry: &mut PluginRegistry) -> GeoResult<()> {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_default();
    let adapter = PostgisAdapter::new(&db_url);
    let healthy = if !db_url.is_empty() {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async { adapter.health_check().await })
            .unwrap_or(false)
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
    register_sync_tools!(registry, "postgis", [
        "dvc_snapshot" => "Run DVC add + push on a file for version tracking" ; serde_json::json!({"type":"object","properties":{"file":{"type":"string"}},"required":["file"]}) => |args| -> ToolResult {
        let snap = dvc_snapshot(args["file"].as_str().unwrap_or("")).map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(serde_json::json!({"file":snap.file,"dvc_hash":snap.dvc_hash}))
    },
        "dvc_hash" => "Get the DVC MD5 hash of a tracked file" ; serde_json::json!({"type":"object","properties":{"file":{"type":"string"}},"required":["file"]}) => |args| -> ToolResult {
        Ok(serde_json::json!({"dvc_hash": dvc_hash(args["file"].as_str().unwrap_or("")).map_err(|e| geo_core::GeoError::Other(e.to_string()))? }))
    }]);
    if !db_url.is_empty() {
        let url_store = db_url.clone();
        registry.register_tool_async(
            "postgis",
            geo_registry::registry::ToolDef {
                name: "store_migrate".into(),
                description: "Run PostGIS database migrations".into(),
                input_schema: serde_json::json!({"type":"object","properties":{},"required":[]}),
            },
            move |_args| {
                let url = url_store.clone();
                Box::pin(async move {
                    let store = PostgisStore::connect(&url)
                        .await
                        .map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
                    run_migrations(store.pool())
                        .await
                        .map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
                    Ok(serde_json::json!("Migrations applied successfully"))
                })
            },
        );
        let url_query = db_url.clone();
        registry.register_tool_async("postgis", geo_registry::registry::ToolDef {
            name: "store_query".into(), description: "Execute a SQL query and return results as JSON".into(),
            input_schema: serde_json::json!({"type":"object","properties":{"sql":{"type":"string"}},"required":["sql"]}),
        }, move |args| {
            let url = url_query.clone();
            Box::pin(async move {
                let store = PostgisStore::connect(&url).await.map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
                store.query_json(args["sql"].as_str().unwrap_or("SELECT 1")).await.map_err(|e| geo_core::GeoError::Database(e.to_string())).map(|rows| serde_json::json!(rows))
            })
        });
        let url_carbon = db_url.clone();
        registry.register_tool_async("postgis", geo_registry::registry::ToolDef {
            name: "carbon_calculate".into(), description: "Calculate carbon emissions using emission factor method".into(),
            input_schema: serde_json::json!({"type":"object","properties":{"aoi_id":{"type":"string"},"year":{"type":"integer"},"source":{"type":"string","default":"IPCC_2019"}},"required":["aoi_id","year"]}),
        }, move |args| {
            let url = url_carbon.clone();
            Box::pin(async move {
                let aoi = args["aoi_id"].as_str().unwrap_or("");
                let year = args["year"].as_u64().unwrap_or(2025) as u16;
                let source = args["source"].as_str().unwrap_or("IPCC_2019");
                let aoi_id = uuid::Uuid::parse_str(aoi).map_err(|e| geo_core::GeoError::invalid_input("aoi_id", format!("invalid UUID: {e}")))?;
                let pool = sqlx::postgres::PgPoolOptions::new().max_connections(2).connect(&url).await.map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
                let engine = PostgisCarbonEngine::new(pool);
                let results = engine.calculate_emission_factor(aoi_id, year, source).await.map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
                let total: f64 = results.iter().map(|r| r.emission_tco2e).sum();
                let summary: Vec<serde_json::Value> = results.iter().map(|r| serde_json::json!({"landcover_class":r.landcover_class,"area_ha":r.area_ha,"emission_tco2e":r.emission_tco2e})).collect();
                Ok(serde_json::json!({"aoi_id":aoi,"year":year,"total_tco2e":total,"results":summary}))
            })
        });
        let url_import = db_url.clone();
        registry.register_tool_async("postgis", geo_registry::registry::ToolDef {
            name: "carbon_import_factors".into(), description: "Import emission factors from a CSV file".into(),
            input_schema: serde_json::json!({"type":"object","properties":{"csv_path":{"type":"string"}},"required":["csv_path"]}),
        }, move |args| {
            let url = url_import.clone();
            Box::pin(async move {
                let pool = sqlx::postgres::PgPoolOptions::new().max_connections(2).connect(&url).await.map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
                let engine = PostgisCarbonEngine::new(pool);
                let count = engine.import_factors_csv(args["csv_path"].as_str().unwrap_or("")).await.map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
                Ok(serde_json::json!({"imported":count,"file":args["csv_path"].as_str().unwrap_or("")}))
            })
        });
    }
    Ok(())
}
