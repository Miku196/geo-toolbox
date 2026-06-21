//! Global configuration for geo-toolbox, loaded from `config.json`.
//!
//! # Loading order
//!
//! 1. Default values (hardcoded in struct impls)
//! 2. `config.json` file (if `--config` / `GEO_CONFIG_PATH` points to one)
//! 3. Environment variables (prefixed with `GEO_`)
//!
//! # Environment variable overrides
//!
//! Each config field can be overridden by an env var:
//! - Flat names: `GEO_MCP_SERVER__PORT`, `GEO_ADAPTERS__POSTGIS__URL`
//! - Double underscore `__` as path separator

#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use std::path::Path;

// ── Root Config ─────────────────────────────────────────────────────────

/// Top-level configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GeoConfig {
    pub mcp_server: McpServerConfig,
    pub adapters: AdapterConfigs,
    pub plugins: PluginConfigs,
    pub logging: LoggingConfig,
}

impl GeoConfig {
    /// Load from a JSON file at `path`.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, GeoConfigError> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| GeoConfigError::Io {
            path: path.as_ref().display().to_string(),
            detail: e.to_string(),
        })?;
        let mut cfg: Self = serde_json::from_str(&content).map_err(|e| GeoConfigError::Parse {
            path: path.as_ref().display().to_string(),
            detail: e.to_string(),
        })?;
        cfg.apply_env_overrides();
        Ok(cfg)
    }

    /// Load from the default locations, in order:
    /// 1. `GEO_CONFIG_PATH` env var
    /// 2. `./config.json`
    /// 3. `~/.geo-toolbox/config.json`
    /// 4. `./config/config.json`
    ///    Returns `Ok(None)` if no config file found.
    pub fn load_default() -> Result<Option<Self>, GeoConfigError> {
        // 1. env var explicit path
        if let Ok(path) = std::env::var("GEO_CONFIG_PATH") {
            if Path::new(&path).exists() {
                return Ok(Some(Self::from_file(&path)?));
            }
        }

        // 2. candidate locations
        let candidates = ["./config.json", "./config/config.json"];
        // 3. home directory
        let home_config = dirs_next::config_dir()
            .map(|d| d.join("geo-toolbox").join("config.json"))
            .filter(|p| p.exists());

        for candidate in candidates
            .iter()
            .map(std::path::Path::new)
            .chain(home_config.iter().map(|p| p.as_path()))
        {
            if candidate.exists() {
                return Ok(Some(Self::from_file(candidate)?));
            }
        }

        Ok(None) // no config file
    }

    /// Override fields from environment variables `GEO_SECTION__KEY`.
    fn apply_env_overrides(&mut self) {
        // MCP server
        if let Ok(v) = std::env::var("GEO_MCP_SERVER__PORT") {
            if let Ok(n) = v.parse() {
                self.mcp_server.port = n;
            }
        }
        if let Ok(v) = std::env::var("GEO_MCP_SERVER__HOST") {
            self.mcp_server.host = v;
        }

        // PostgreSQL
        if let Ok(url) = std::env::var("GEO_ADAPTERS__POSTGIS__URL") {
            self.adapters.postgis.url = Some(url);
            self.adapters.postgis.enabled = true;
        }
        if std::env::var("PG_URL").is_ok() && self.adapters.postgis.url.is_none() {
            self.adapters.postgis.url = Some(std::env::var("PG_URL").unwrap_or_default());
            self.adapters.postgis.enabled = true;
        }

        // GEE
        if let Ok(v) = std::env::var("GEO_ADAPTERS__GEE__PROJECT") {
            self.adapters.gee.project = Some(v);
            self.adapters.gee.enabled = true;
        }
        if let Ok(v) = std::env::var("GEE_PROJECT") {
            if self.adapters.gee.project.is_none() {
                self.adapters.gee.project = Some(v);
                self.adapters.gee.enabled = true;
            }
        }

        // STAC
        if let Ok(v) = std::env::var("GEO_ADAPTERS__STAC__ENDPOINT") {
            self.adapters.stac.endpoint = v;
        }

        // QGIS
        if let Ok(v) = std::env::var("GEO_ADAPTERS__QGIS__SERVER_URL") {
            self.adapters.qgis.server_url = Some(v);
        }
        if let Ok(v) = std::env::var("QGIS_BACKEND") {
            self.adapters.qgis.backend = v;
        }

        // Logging
        if let Ok(v) = std::env::var("GEO_LOGGING__LEVEL") {
            self.logging.level = v;
        }
    }
}

// ── MCP Server ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub host: String,
    pub port: u16,
    pub max_concurrent: usize,
    pub tool_timeout_secs: u64,
    pub connection_timeout_secs: u64,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 9378,
            max_concurrent: 8,
            tool_timeout_secs: 300,
            connection_timeout_secs: 3600,
        }
    }
}

// ── Adapters ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AdapterConfigs {
    pub postgis: PostgisConfig,
    pub gee: GeeConfig,
    pub stac: StacConfig,
    pub osm: OsmConfig,
    pub qgis: QgisConfig,
    pub gdal: GdalConfig,
    pub cad: CadConfig,
    pub duckdb: DuckdbConfig,
    pub iot: IotConfig,
    pub dssat: DssatConfig,
    pub modflow: ModflowConfig,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgisConfig {
    pub enabled: bool,
    pub url: Option<String>,
    pub max_connections: u32,
    pub min_connections: u32,
}

impl Default for PostgisConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: None,
            max_connections: 20,
            min_connections: 2,
        }
    }
}

/// Google Earth Engine 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeeConfig {
    pub enabled: bool,
    pub project: Option<String>,
    pub service_account: Option<String>,
    pub credentials_path: Option<String>,
    pub gcs_bucket: String,
    pub gcs_prefix: String,
}

impl Default for GeeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            project: None,
            service_account: None,
            credentials_path: None,
            gcs_bucket: "gee-exports".into(),
            gcs_prefix: "exports/".into(),
        }
    }
}

/// STAC 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StacConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub default_collection: String,
    pub max_items: usize,
}

impl Default for StacConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: "https://planetarycomputer.microsoft.com/api/stac/v1".into(),
            default_collection: "sentinel-2-l2a".into(),
            max_items: 100,
        }
    }
}

/// OSM 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsmConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub timeout_secs: u64,
}

impl Default for OsmConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: "https://overpass-api.de/api/interpreter".into(),
            timeout_secs: 30,
        }
    }
}

/// QGIS 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QgisConfig {
    pub enabled: bool,
    pub backend: String,
    pub server_url: Option<String>,
    pub qgis_process_path: String,
}

impl Default for QgisConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backend: "subprocess".into(),
            server_url: None,
            qgis_process_path: "qgis_process".into(),
        }
    }
}

/// GDAL CLI 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GdalConfig {
    pub enabled: bool,
    pub gdal_translate_path: String,
    pub gdalwarp_path: String,
    pub ogr2ogr_path: String,
    pub gcs_bucket: String,
    pub temp_dir: String,
}

impl Default for GdalConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            gdal_translate_path: "gdal_translate".into(),
            gdalwarp_path: "gdalwarp".into(),
            ogr2ogr_path: "ogr2ogr".into(),
            gcs_bucket: "gee-exports".into(),
            temp_dir: "/tmp/geo-toolbox".into(),
        }
    }
}

/// CAD 配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CadConfig {
    pub enabled: bool,
    pub dxf_template_path: Option<String>,
}

/// DuckDB 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuckdbConfig {
    pub enabled: bool,
    pub path: Option<String>,
    pub tmp_dir: String,
}

impl Default for DuckdbConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: None,
            tmp_dir: "/tmp/geo-toolbox-duckdb".into(),
        }
    }
}

/// IoT 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IotConfig {
    pub enabled: bool,
    pub mqtt_broker: String,
    pub mqtt_port: u16,
    pub mqtt_topics: Vec<String>,
    pub mqtt_client_id: String,
}

impl Default for IotConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mqtt_broker: "localhost".into(),
            mqtt_port: 1883,
            mqtt_topics: vec!["geo/sensors/#".into()],
            mqtt_client_id: "geo-toolbox-iot".into(),
        }
    }
}

/// DSSAT 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DssatConfig {
    pub enabled: bool,
    pub dssat_path: String,
    pub temp_dir: String,
}

impl Default for DssatConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            dssat_path: "/opt/dssat/".into(),
            temp_dir: "/tmp/geo-toolbox-dssat".into(),
        }
    }
}

/// MODFLOW 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModflowConfig {
    pub enabled: bool,
    pub mf6_path: String,
    pub temp_dir: String,
}

impl Default for ModflowConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mf6_path: "mf6".into(),
            temp_dir: "/tmp/geo-toolbox-modflow".into(),
        }
    }
}

// ── Plugins ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PluginConfigs {
    pub carbon: CarbonPluginConfig,
    pub geohazard: GeohazardPluginConfig,
    pub coastal: CoastalPluginConfig,
    pub agri: AgriPluginConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonPluginConfig {
    pub default_ecozone: String,
    pub default_soc_ref: f64,
    pub default_source: String,
}

impl Default for CarbonPluginConfig {
    fn default() -> Self {
        Self {
            default_ecozone: "temperate_broadleaf".into(),
            default_soc_ref: 70.0,
            default_source: "IPCC_2019".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeohazardPluginConfig {
    pub default_soil_density_kn_m3: f64,
    pub default_water_table_ratio: f64,
}

impl Default for GeohazardPluginConfig {
    fn default() -> Self {
        Self {
            default_soil_density_kn_m3: 20.0,
            default_water_table_ratio: 0.3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoastalPluginConfig {
    pub default_ambient_pressure_hpa: f64,
    pub default_holland_b: f64,
}

impl Default for CoastalPluginConfig {
    fn default() -> Self {
        Self {
            default_ambient_pressure_hpa: 1013.0,
            default_holland_b: 1.3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgriPluginConfig {
    pub default_par_mj_m2_day: f64,
}

impl Default for AgriPluginConfig {
    fn default() -> Self {
        Self {
            default_par_mj_m2_day: 20.0,
        }
    }
}

// ── Logging ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub otel_endpoint: Option<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".into(),
            format: "text".into(),
            otel_endpoint: None,
        }
    }
}

// ── Errors ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum GeoConfigError {
    Io { path: String, detail: String },
    Parse { path: String, detail: String },
}

impl std::fmt::Display for GeoConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, detail } => write!(f, "config I/O error at {path}: {detail}"),
            Self::Parse { path, detail } => {
                write!(f, "config parse error at {path}: {detail}")
            }
        }
    }
}

impl std::error::Error for GeoConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = GeoConfig::default();
        assert_eq!(cfg.mcp_server.port, 9378);
        assert!(!cfg.adapters.postgis.enabled);
        assert!(cfg.adapters.stac.enabled);
        assert_eq!(
            cfg.adapters.stac.endpoint,
            "https://planetarycomputer.microsoft.com/api/stac/v1"
        );
    }

    #[test]
    fn test_serialize_roundtrip() {
        let cfg = GeoConfig::default();
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        let parsed: GeoConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.mcp_server.port, cfg.mcp_server.port);
        assert_eq!(parsed.adapters.stac.endpoint, cfg.adapters.stac.endpoint);
    }
}
