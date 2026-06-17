//! Plugin system traits — the foundation of geo-toolbox's core + plugin + adapter architecture.
//!
//! # Architecture
//!
//! ```text
//! ┌── Core (geo-core) ─────────────────────────────────────┐
//! │  Plugin trait  ← base trait for all plugins             │
//! │  StorePlugin   ← spatial storage (PostGIS, Parquet, S3) │
//! │  IngestPlugin  ← data ingestion (CamoFox, NMEA, MQTT)   │
//! │  ProcessPlugin ← geoprocessing (internal algorithms)    │
//! │  OutputPlugin  ← export formats (DXF, Excel, GeoJSON)   │
//! │  CarbonPlugin  ← carbon accounting engines              │
//! │  ExternalAdapter ← bridge to external tools (GEE, GDAL) │
//! └────────────────────────────────────────────────────────┘
//! ```
//!
//! All plugins and adapters implement one or more of these traits,
//! then register with `geo_registry::PluginRegistry` for discovery.

#![allow(async_fn_in_trait)]

use crate::errors::GeoResult;
use serde::{Deserialize, Serialize};

// ── Base Plugin trait ─────────────────────────────────────────────────────

/// Base trait that every plugin and adapter must implement.
///
/// Provides identity and lifecycle hooks used by the [`PluginRegistry`]
/// for discovery, configuration, and teardown.
///
/// # Extension safety
/// New default methods should be added here instead of required methods
/// to avoid breaking all 70+ implementors across the workspace.
pub trait Plugin: Send + Sync {
    /// The configuration type for this plugin (must implement [`PluginConfig`]).
    type Config: PluginConfig;

    /// Create a new plugin instance from the given configuration.
    fn new(config: Self::Config) -> Self;

    /// Unique name for this plugin (e.g. `"postgis"`, `"gee-adapter"`).
    fn name(&self) -> &str;

    /// Semantic version string.
    fn version(&self) -> &str;

    /// Human-readable description.
    fn description(&self) -> &str;

    /// Plugin category: `"store"`, `"ingest"`, `"process"`, `"output"`, `"carbon"`, `"adapter"`.
    fn category(&self) -> PluginCategory;

    /// Build a [`PluginMeta`] from the default configuration.
    fn make_default_config() -> Self::Config {
        Self::Config::default()
    }

    /// Deserialize configuration from a JSON string.
    fn config_from_string(s: &str) -> GeoResult<Self::Config> {
        serde_json::from_str(s).map_err(crate::errors::GeoError::Serde)
    }

    /// Create a plugin instance with default configuration.
    fn default_plugin() -> Self
    where
        Self: Sized,
    {
        Self::new(Self::Config::default())
    }

    /// Called once after registration. Use for connection pools, config loading.
    fn init(&mut self) -> GeoResult<()> {
        Ok(())
    }

    /// Called before the registry drops this plugin. Use for graceful shutdown.
    fn shutdown(&mut self) -> GeoResult<()> {
        Ok(())
    }

    /// Whether this plugin is currently healthy / available.
    fn is_healthy(&self) -> bool {
        true
    }

    /// Build a [`PluginMeta`] snapshot from this plugin's identity fields.
    ///
    /// Default implementation delegates to `name()`, `version()`, `description()`,
    /// `category()`, and `is_healthy()`. Override to supply `extra` fields (e.g.
    /// adapter endpoints, output format hints).
    fn metadata(&self) -> PluginMeta {
        PluginMeta {
            name: self.name().to_string(),
            version: self.version().to_string(),
            description: self.description().to_string(),
            category: self.category(),
            healthy: self.is_healthy(),
            extra: serde_json::Value::Null,
        }
    }

    /// Returns true when this plugin is an adapter bridging an external tool.
    /// Default: checks `category() == PluginCategory::Adapter`.
    fn is_adapter(&self) -> bool {
        self.category() == PluginCategory::Adapter
    }

    /// Returns true when this plugin is a carbon accounting engine.
    /// Default: checks `category() == PluginCategory::Carbon`.
    fn is_carbon(&self) -> bool {
        self.category() == PluginCategory::Carbon
    }
}

/// Plugin category enum used for registry grouping and dispatch routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginCategory {
    /// Spatial data storage (PostGIS, Parquet, MinIO)
    Store,
    /// Data ingestion (CamoFox, NMEA, MQTT)
    Ingest,
    /// Geoprocessing algorithms (internal, pure Rust)
    Process,
    /// Output / export formats (DXF, Excel, GeoJSON, Report)
    Output,
    /// Carbon accounting engines
    Carbon,
    /// External tool adapters (GEE, GDAL, QGIS)
    Adapter,
}

impl PluginCategory {
    /// Parse from a string (case-insensitive).
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "store" => Some(Self::Store),
            "ingest" => Some(Self::Ingest),
            "process" => Some(Self::Process),
            "output" => Some(Self::Output),
            "carbon" => Some(Self::Carbon),
            "adapter" => Some(Self::Adapter),
            _ => None,
        }
    }

    /// Human-readable label.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Store => "store",
            Self::Ingest => "ingest",
            Self::Process => "process",
            Self::Output => "output",
            Self::Carbon => "carbon",
            Self::Adapter => "adapter",
        }
    }
}

// ── Domain-specific Plugin traits ─────────────────────────────────────────

/// A spatial feature with geometry and properties, used as the
/// universal interchange format between Store/Ingest/Output plugins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoFeature {
    /// Feature identifier (UUID or external ID).
    pub id: String,
    /// GeoJSON geometry as a JSON value.
    pub geometry: serde_json::Value,
    /// Key-value properties.
    pub properties: serde_json::Value,
}

/// Result of an ingest operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResult {
    /// Number of records accepted.
    pub accepted: u64,
    /// Number of records rejected (e.g. invalid coordinates).
    pub rejected: u64,
    /// Ingest source identifier (file path, topic, etc.).
    pub source: String,
    /// ISO-8601 timestamp.
    pub timestamp: String,
}

/// Input to an output plugin export operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportInput {
    /// Output format hint (e.g. "dxf", "xlsx", "geojson", "md").
    pub format: String,
    /// SQL query or data source reference.
    pub query: Option<String>,
    /// Direct GeoJSON data (for WASM/local paths).
    /// 可选 GeoJSON 数据
    pub geojson: Option<String>,
    /// Optional CRS transformations.
    pub from_epsg: Option<u16>,
    /// Target EPSG code.
    pub to_epsg: Option<u16>,
    /// Arbitrary extra parameters.
    #[serde(default)]
    pub params: serde_json::Value,
}

/// Parameters for carbon accounting calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonParams {
    /// AOI identifier.
    pub aoi_id: String,
    /// Accounting year.
    pub year: u16,
    /// Emission factor source name (e.g. "IPCC_2019").
    pub source: String,
    /// Whether to do a dry run (no DB writes).
    #[serde(default)]
    pub dry_run: bool,
    /// Extra parameters.
    #[serde(default)]
    pub extra: serde_json::Value,
}

/// Result of a carbon calculation (simplified, for interchange).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonResult {
    /// Total emission in tCO₂e.
    pub total_tco2e: f64,
    /// Number of landcover classes evaluated.
    pub class_count: usize,
    /// Per-class breakdown.
    pub classes: Vec<ClassBreakdown>,
    /// ISO-8601 timestamp.
    pub calculated_at: String,
}

/// Per-class carbon breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassBreakdown {
    /// Landcover class name.
    pub landcover_class: String,
    /// Area in hectares.
    pub area_ha: f64,
    /// Net emission in tCO₂e.
    pub emission_tco2e: f64,
}

// ── Plugin traits ─────────────────────────────────────────────────────────

/// Plugins that provide spatial data storage.
///
/// Implemented by: PostGIS store, GeoParquet store, MinIO/S3 store.
///
/// **Forward-looking**: PostgisAdapter is the first implementor (see
/// `geo-adapter-postgis`). Other store backends (DuckDB, TiDB) planned.
pub trait StorePlugin: Plugin {
    /// Execute database migrations.
    async fn migrate(&self) -> GeoResult<()>;

    /// Write features to a named table/collection.
    async fn write_features(&self, table: &str, features: &[GeoFeature]) -> GeoResult<u64>;

    /// Execute a query and return results as JSON.
    async fn query_json(&self, sql: &str) -> GeoResult<serde_json::Value>;

    /// Check connectivity.
    async fn ping(&self) -> GeoResult<bool>;
}

/// Plugins that ingest data from external sources.
///
/// Implemented by: CamoFox parser, NMEA parser, MQTT stream subscriber.
pub trait IngestPlugin: Plugin {
    /// Type of source this plugin handles (e.g. "camofox", "nmea", "mqtt").
    fn source_type(&self) -> &str;

    /// Ingest from a file path.
    async fn ingest_file(&self, path: &str) -> GeoResult<IngestResult>;

    /// Ingest from raw text content (for WASM / streaming).
    async fn ingest_content(&self, content: &str, source_hint: &str) -> GeoResult<IngestResult>;
}

/// Plugins that generate output formats for human consumption.
///
/// Implemented by: DXF exporter, Excel dashboard, GeoJSON exporter, Report generator.
pub trait OutputPlugin: Plugin {
    /// Output format identifier (e.g. "dxf", "xlsx", "geojson", "md", "html").
    fn output_format(&self) -> &str;

    /// Generate output from the given input, returning bytes.
    async fn export(&self, input: &ExportInput) -> GeoResult<Vec<u8>>;

    /// Generate output and write directly to a file path.
    async fn export_to_file(&self, input: &ExportInput, output_path: &str) -> GeoResult<()>;
}

/// Plugins that perform carbon accounting.
///
/// Implemented by: IPCC emission factor engine, LCA engine, carbon sink estimator.
pub trait CarbonPlugin: Plugin {
    /// Calculate emissions for the given parameters.
    async fn calculate(&self, params: &CarbonParams) -> GeoResult<CarbonResult>;

    /// Import emission factors from a CSV file.
    async fn import_factors_csv(&self, csv_path: &str) -> GeoResult<u64>;

    /// Query emission factors valid for a given year.
    async fn query_factors(&self, year: u16, source: Option<&str>) -> GeoResult<serde_json::Value>;
}

/// Plugins that execute internal geoprocessing algorithms.
///
/// Implemented by: pure-Rust spatial operations, OGC service handlers.
pub trait ProcessPlugin: Plugin {
    /// Process type identifier (e.g. "buffer", "intersection", "wms", "wfs").
    fn process_type(&self) -> &str;

    /// Execute a processing operation with the given parameters.
    async fn execute(&self, params: serde_json::Value) -> GeoResult<serde_json::Value>;
}

// ── External Adapter trait ────────────────────────────────────────────────

/// Marker trait for plugins that bridge to **external tools** (Python subprocess,
/// REST API, CLI wrapper). These adapters are inherently less reliable than
/// internal plugins and must implement health checks.
///
/// Implemented by: GEE adapter, GDAL adapter, QGIS adapter.
pub trait ExternalAdapter: Plugin {
    /// The external command or service URL this adapter fronts.
    fn external_endpoint(&self) -> &str;

    /// Check if the external service is reachable and healthy.
    async fn health_check(&self) -> GeoResult<bool>;

    /// Get the external tool version (e.g. `gdal_translate --version`).
    async fn external_version(&self) -> GeoResult<String>;

    /// Whether this adapter requires network access.
    fn requires_network(&self) -> bool {
        true
    }

    // ── 双向通信 ──

    /// 推送数据到外部系统。
    async fn push(&self, table: &str, data: &[GeoFeature]) -> GeoResult<u64>;

    /// 从外部系统拉取数据。
    async fn pull(&self, query: &str) -> GeoResult<Vec<GeoFeature>>;

    /// 执行外部命令。
    async fn execute(
        &self,
        command: &str,
        params: serde_json::Value,
    ) -> GeoResult<serde_json::Value>;
}

// ── Plugin metadata for registry introspection ────────────────────────────

/// Metadata about a registered plugin, exposed via the registry for tool listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMeta {
    /// Plugin name.
    pub name: String,
    /// Version string.
    pub version: String,
    /// Human-readable description.
    pub description: String,
    /// Category.
    pub category: PluginCategory,
    /// Whether the plugin is currently healthy.
    pub healthy: bool,
    /// Extra info (endpoint for adapters, format for outputs, etc.).
    pub extra: serde_json::Value,
}

/// Lightweight plugin header for `rules.toml` `[plugin]` section.
///
/// Each plugin's `rules.toml` starts with:
/// ```toml
/// [plugin]
/// name = "ecology"
/// version = "0.1.0"
/// description = "Ecological restoration assessment"
/// ```
///
/// Previously each plugin defined its own identical 3-field struct.
/// Using this shared type eliminates 9 duplicate definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHeader {
    /// Human-readable plugin name, e.g. "Carbon Accounting"
    pub name: String,
    /// Semantic version of the plugin
    pub version: String,
    /// Short description of what the plugin provides
    pub description: String,
}

// ── PluginConfig trait (Phase 2b architecture unification) ─────────────────

/// Trait for plugin configuration structs that can be loaded from `rules.toml`.
///
/// Each plugin defines a `Config` struct that derives `Deserialize` + `Default`,
/// then implements this trait. The `default()` impl typically reads from the
/// plugin's own `rules.toml` via `include_str!("../rules.toml")`.
pub trait PluginConfig: Default + for<'a> serde::Deserialize<'a> {
    /// Validate configuration values (e.g. weight sum to 1.0, thresholds in range).
    fn validate(&self) -> GeoResult<()> {
        Ok(())
    }
}

/// Empty config for plugins without configuration (e.g. [`CoastalPlugin`], CLI adapters).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmptyConfig;

impl PluginConfig for EmptyConfig {
    fn validate(&self) -> GeoResult<()> {
        Ok(())
    }
}

/// Register plugin tools using a declarative macro.
///
/// Each plugin that registers MCP tools currently has a ~100-200 line
/// `register_tools` function with repetitive boilerplate. This macro
/// reduces that to a compact declaration.
///
/// # Example
/// ```ignore
/// register_plugin!(registry, {
///     "hydro_runoff" => |args: serde_json::Value| -> GeoResult<serde_json::Value> { ... },
///     "hydro_inundation" => |args: serde_json::Value| -> GeoResult<serde_json::Value> { ... },
/// });
/// ```
#[macro_export]
macro_rules! register_plugin {
    ($registry:expr, { $( $name:literal => $handler:expr ),* $(,)? }) => {
        $( $crate::register_plugin!(@tool $registry, $name, $handler); )*
    };
    (@tool $registry:expr, $name:literal, $handler:expr) => {
        $registry.register_tool(
            $name,
            $name,
            std::sync::Arc::new(move |args: serde_json::Value| -> $crate::errors::GeoResult<serde_json::Value> {
                let handler: fn(serde_json::Value) -> $crate::errors::GeoResult<serde_json::Value> = $handler;
                handler(args)
            }),
        );
    };
}

/// Auto-generate `Default` for a config struct by loading from `rules.toml`.
///
/// Replaces the repetitive pattern:
/// ```ignore
/// impl Default for FooConfig {
///     fn default() -> Self {
///         toml::from_str(include_str!("../rules.toml")).expect("Default foo rules.toml is valid")
///     }
/// }
/// ```
///
/// Usage: `default_from_rules!(EcologyConfig, "ecology");`
#[macro_export]
macro_rules! default_from_rules {
    ($type:ty, $name:literal) => {
        impl Default for $type {
            fn default() -> Self {
                ::toml::from_str(include_str!("../rules.toml"))
                    .unwrap_or_else(|e| panic!("Default {} rules.toml is valid: {e}", $name))
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_category_roundtrip() {
        for cat in &[
            PluginCategory::Store,
            PluginCategory::Ingest,
            PluginCategory::Process,
            PluginCategory::Output,
            PluginCategory::Carbon,
            PluginCategory::Adapter,
        ] {
            let s = cat.as_str();
            let parsed = PluginCategory::parse(s).unwrap();
            assert_eq!(parsed, *cat);
        }
    }

    #[test]
    fn test_plugin_category_invalid() {
        assert!(PluginCategory::parse("unknown").is_none());
    }
}
