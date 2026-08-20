//! Unified error types for all geo-toolbox crates.
//!
//! Every crate uses `GeoResult<T>` as its return type, avoiding
//! per-crate error enum fragmentation.

use thiserror::Error;

/// The single error type shared across the entire geo-toolbox workspace.
#[derive(Error, Debug)]
pub enum GeoError {
    /// CRS lookup failed for the given (from, to) EPSG pair.
    #[error("CRS not found: from={0}, to={1}")]
    CrsNotFound(u16, u16),

    /// PROJ coordinate transformation error.
    #[error("CRS transform failed: {0}")]
    CrsTransform(String),

    /// Entity not found (AOI, task, feature).
    #[error("{entity} not found: {id}")]
    NotFound {
        /// Type of entity (AOI, task, tile, year, etc.).
        entity: String,
        /// Entity identifier.
        id: String,
    },

    /// Input parameter validation failed.
    #[error("Invalid {field}: {reason}")]
    InvalidInput {
        /// Name of the invalid field.
        field: String,
        /// Why the value is invalid.
        reason: String,
    },

    /// Configuration file error.
    #[error("Config error in {path}: {detail}")]
    ConfigError {
        /// Path to the config file.
        path: String,
        /// What went wrong.
        detail: String,
    },

    /// Geometry failed validation (e.g. out-of-range coordinates).
    #[error("Geometry validation: {0}")]
    Validation(String),

    /// Database error (wraps sqlx / PostGIS errors from higher-level crates).
    #[error("Database: {0}")]
    Database(String),

    /// Filesystem I/O error.
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization / deserialization error.
    #[error("Serialization: {0}")]
    Serde(#[from] serde_json::Error),

    /// Object store (S3 / MinIO / GCS) error.
    #[error("Object store: {0}")]
    ObjectStore(String),

    /// Message queue error (Kafka / MQTT).
    #[error("Message queue: {0}")]
    MessageQueue(String),

    /// GCS → MinIO bridge sync failure.
    #[error("GCS bridge: {0}")]
    GcsBridge(String),

    /// CSV format error.
    #[error("CSV: {0}")]
    Csv(String),

    /// A catch-all for errors from external processes (qgis_process, dvc CLI, etc.).
    #[error("External process '{command}': {message}")]
    ExternalProcess {
        /// The command that was run.
        command: String,
        /// Stderr output or error description.
        message: String,
    },

    /// Payload exceeds configured size limit.
    #[error("Payload too large: {actual} bytes (limit {limit} bytes)")]
    PayloadTooLarge {
        /// Actual payload size in bytes.
        actual: u64,
        /// Maximum allowed payload size in bytes.
        limit: u64,
    },

    /// Too many features in a single request.
    #[error("Too many features: {actual} (limit {limit})")]
    TooManyFeatures {
        /// Actual feature count.
        actual: usize,
        /// Maximum allowed feature count.
        limit: usize,
    },

    /// Raster dimension exceeds limit.
    #[error("Raster too large: {cols}×{rows} (max dimension {max_dim})")]
    RasterTooLarge {
        /// Actual column count.
        cols: usize,
        /// Actual row count.
        rows: usize,
        /// Maximum allowed dimension.
        max_dim: usize,
    },

    /// Raster pixel count exceeds limit.
    #[error("Raster too many pixels: {pixels} (limit {limit})")]
    RasterTooManyPixels {
        /// Actual pixel count.
        pixels: u64,
        /// Maximum allowed pixel count.
        limit: u64,
    },

    /// Not-yet-implemented feature.
    #[error("Not implemented: {0}")]
    Unimplemented(String),

    /// Catch-all for library-specific errors that don't have a dedicated variant.
    #[error("{0}")]
    Other(String),
}

/// Convenience alias: `Result<T, GeoError>`.
pub type GeoResult<T> = Result<T, GeoError>;

// ── Geometry Facade Error ─────────────────────────────────────────

/// Facade 层几何解析专用错误。
///
/// 与 `GeoError` 分离 — `GeoError` 为全 workspace 共享的通用错误,
/// `GeometryFacadeError` 仅用于 Plugin 层 Facade 入口的几何解析路径。
/// 防止 `GeoError` 枚举膨胀, 保持 facade 语义独立。
#[derive(Error, Debug)]
pub enum GeometryFacadeError {
    /// GeoJSON 反序列化失败 (透传 serde_json::Error)。
    #[error("GeoJSON 解析失败: {0}")]
    GeoJsonParse(#[from] serde_json::Error),

    /// 不支持的 GeoJSON 几何类型。
    #[error(
        "不支持的几何类型 '{actual}'. 支持: Point, MultiPoint, LineString, Polygon, MultiPolygon"
    )]
    UnsupportedGeometry {
        /// The unsupported geometry type string.
        actual: String,
    },

    /// 几何对象为空 — 无坐标、空数组、或所有 feature 解析失败。
    #[error("几何为空: {0}")]
    EmptyGeometry(String),
}

/// Facade 层专用 Result 别名。
pub type FacadeResult<T> = Result<T, GeometryFacadeError>;

// ── GeometryFacadeError 便捷构造函数 ──

impl GeometryFacadeError {
    /// 不支持的几何类型。
    pub fn unsupported_geometry(actual: impl Into<String>) -> Self {
        Self::UnsupportedGeometry {
            actual: actual.into(),
        }
    }

    /// 几何为空。
    pub fn empty_geometry(reason: impl Into<String>) -> Self {
        Self::EmptyGeometry(reason.into())
    }
}

// ── GeometryFacadeError → GeoError 转换 ──

impl From<GeometryFacadeError> for GeoError {
    fn from(e: GeometryFacadeError) -> Self {
        GeoError::Validation(e.to_string())
    }
}

// ── Convenience constructors ──

impl GeoError {
    /// Entity not found (AOI, task, feature, etc.).
    pub fn not_found(entity: impl Into<String>, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity: entity.into(),
            id: id.into(),
        }
    }
    /// Invalid input parameter.
    pub fn invalid_input(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidInput {
            field: field.into(),
            reason: reason.into(),
        }
    }
    /// Configuration file error.
    pub fn config_error(path: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::ConfigError {
            path: path.into(),
            detail: detail.into(),
        }
    }
}

/// Validate that a SQL string contains only SELECT-like statements.
/// Returns `Ok(())` if safe, `Err(Validation)` if destructive keywords found.
pub fn validate_select_sql(sql: &str) -> GeoResult<()> {
    let trimmed = sql.trim();
    let upper = trimmed.to_ascii_uppercase();
    if trimmed.is_empty() || !(upper.starts_with("SELECT") || upper.starts_with("WITH")) {
        return Err(GeoError::Validation(
            "SQL query rejected: only SELECT or WITH queries are allowed.".into(),
        ));
    }
    if trimmed.contains(';') || trimmed.contains('\\') || upper.contains("PROGRAM") {
        return Err(GeoError::Validation(
            "SQL query rejected: contains unsafe statement separators or characters".into(),
        ));
    }
    let forbidden = [
        "DROP", "DELETE", "INSERT", "UPDATE", "ALTER", "CREATE", "TRUNCATE", "GRANT", "REVOKE",
        "COPY", "EXECUTE", "CALL",
    ];
    for keyword in forbidden {
        if upper.contains(keyword) {
            return Err(GeoError::Validation(format!(
                "SQL query rejected: contains forbidden keyword '{keyword}'. Only SELECT queries are allowed."
            )));
        }
    }
    Ok(())
}

/// Validate that a file path is safe for use in subprocess commands.
///
/// Rejects paths containing directory traversal, shell metacharacters,
/// or absolute system paths that should not be accessible.
pub fn validate_safe_path(path: &str) -> GeoResult<()> {
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed.contains('\u{0}') {
        return Err(GeoError::Validation(
            "Path rejected: must be non-empty and cannot contain NUL".into(),
        ));
    }
    let normalized = trimmed.replace('\\', "/");
    if normalized.starts_with("//") {
        return Err(GeoError::Validation(
            "Path rejected: UNC paths are not allowed".into(),
        ));
    }
    if normalized.split('/').any(|segment| segment == "..") {
        return Err(GeoError::Validation(
            "Path rejected: contains '..' directory traversal".into(),
        ));
    }
    let forbidden = [';', '|', '&', '$', '`', '(', ')', '<', '>', '\n', '\r'];
    if trimmed.contains(forbidden.as_slice()) {
        return Err(GeoError::Validation(
            "Path rejected: contains shell metacharacters".into(),
        ));
    }
    let lower = normalized.to_ascii_lowercase();
    for sensitive in ["/etc", "/proc", "/sys", "/dev", "c:/windows"] {
        if lower == sensitive
            || lower.starts_with(&format!("{sensitive}/"))
            || lower.contains(&format!("{sensitive}/"))
        {
            return Err(GeoError::Validation(
                "Path rejected: references sensitive system location".into(),
            ));
        }
    }
    Ok(())
}

/// Validate a SQL identifier (table name, column name) to prevent SQL injection.
pub fn validate_sql_identifier(name: &str) -> GeoResult<()> {
    if name.is_empty() {
        return Err(GeoError::Validation("SQL identifier is empty".into()));
    }

    for namespace in name.split("::") {
        for segment in namespace.split('.') {
            let mut chars = segment.chars();
            let Some(first) = chars.next() else {
                return Err(GeoError::Validation(format!(
                    "SQL identifier '{name}' rejected: contains an empty qualifier"
                )));
            };
            if !first.is_ascii_alphabetic() && first != '_' {
                return Err(GeoError::Validation(format!(
                    "SQL identifier '{name}' rejected: each segment must start with ASCII letter or underscore"
                )));
            }
            if !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
                return Err(GeoError::Validation(format!(
                    "SQL identifier '{name}' rejected: contains illegal character"
                )));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_sql_accepts_select_and_cte_queries() {
        for query in [
            "SELECT id, name FROM parcels",
            "  select * from parcels where id = 1",
            "WITH recent AS (SELECT id FROM parcels) SELECT * FROM recent",
        ] {
            assert!(
                validate_select_sql(query).is_ok(),
                "query should be accepted: {query}"
            );
        }
    }

    #[test]
    fn select_sql_rejects_non_read_only_and_multi_statement_input() {
        for query in [
            "",
            "VACUUM",
            "PRAGMA table_info(parcels)",
            "SET role = admin",
            "SELECT * FROM parcels; DELETE FROM parcels",
            "SELECT * FROM parcels;",
            "SELECT * FROM parcels COPY data TO PROGRAM 'sh'",
        ] {
            assert!(
                validate_select_sql(query).is_err(),
                "query should be rejected: {query}"
            );
        }
    }

    #[test]
    fn safe_path_rejects_empty_nul_and_sensitive_system_locations() {
        for path in [
            "",
            "   ",
            "dataset\0.gpkg",
            "/etc",
            "/etc/passwd",
            "C:/Windows/System32/config",
            r"\\server\share\file.gpkg",
            "../outside.gpkg",
        ] {
            assert!(
                validate_safe_path(path).is_err(),
                "path should be rejected: {path:?}"
            );
        }
    }

    #[test]
    fn safe_path_accepts_relative_data_paths() {
        for path in [
            "data/input.gpkg",
            "outputs/2026/result.geojson",
            "file..name.gpkg",
        ] {
            assert!(
                validate_safe_path(path).is_ok(),
                "path should be accepted: {path}"
            );
        }
    }

    #[test]
    fn sql_identifier_rejects_empty_segments_and_accepts_qualified_names() {
        for identifier in ["schema.table", "schema::table", "_private.column_2"] {
            assert!(
                validate_sql_identifier(identifier).is_ok(),
                "identifier should be accepted: {identifier}"
            );
        }
        for identifier in ["schema..table", "schema.", "::table", "table;drop"] {
            assert!(
                validate_sql_identifier(identifier).is_err(),
                "identifier should be rejected: {identifier}"
            );
        }
    }
}

// Higher-level crates (geo-store, geo-ingest, etc.) provide their own
// From<sqlx::Error>, From<object_store::Error>, etc. conversions via
// a helper macro or manual impls. This keeps geo-core dependency-free.
//
// Example (in geo-store):
//   impl From<sqlx::Error> for GeoError {
//       fn from(e: sqlx::Error) -> Self { GeoError::Database(e.to_string()) }
//   }
