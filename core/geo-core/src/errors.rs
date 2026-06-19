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
    let upper = sql.to_uppercase();
    let forbidden = [
        "DROP", "DELETE", "INSERT", "UPDATE", "ALTER", "CREATE", "TRUNCATE", "GRANT", "REVOKE",
        "COPY", "EXECUTE", "CALL",
    ];
    for kw in &forbidden {
        if upper.contains(kw) {
            return Err(GeoError::Validation(format!(
                "SQL query rejected: contains forbidden keyword '{kw}'. Only SELECT queries are allowed."
            )));
        }
    }
    if upper.contains('\\') || upper.contains("PROGRAM") {
        return Err(GeoError::Validation(
            "SQL query rejected: contains unsafe characters".into(),
        ));
    }
    Ok(())
}

/// Validate that a file path is safe for use in subprocess commands.
///
/// Rejects paths containing directory traversal, shell metacharacters,
/// or absolute system paths that should not be accessible.
pub fn validate_safe_path(path: &str) -> GeoResult<()> {
    // 禁止路径遍历
    if path.contains("..") {
        return Err(GeoError::Validation(
            "Path rejected: contains '..' (directory traversal)".into(),
        ));
    }
    // 禁止 shell 元字符
    let forbidden = [';', '|', '&', '$', '`', '(', ')', '<', '>', '\n', '\r'];
    if path.contains(forbidden.as_slice()) {
        return Err(GeoError::Validation(
            "Path rejected: contains shell metacharacters".into(),
        ));
    }
    // 禁止绝对系统路径（/etc /proc /sys /dev）
    let lower = path.to_lowercase();
    for sensitive in &[
        "/etc/",
        "/proc/",
        "/sys/",
        "/dev/",
        "c:\\windows",
        "c:\\windows\\system32",
    ] {
        if lower.starts_with(sensitive) || lower.contains(sensitive) {
            return Err(GeoError::Validation(
                "Path rejected: references sensitive system location".into(),
            ));
        }
    }
    Ok(())
}

/// Validate a SQL identifier (table name, column name) to prevent SQL injection.
///
/// Only allows `[a-zA-Z_][a-zA-Z0-9_]*` optionally qualified with `.` or `::`.
/// Rejects spaces, semicolons, quotes, and other SQL metacharacters.
pub fn validate_sql_identifier(name: &str) -> GeoResult<()> {
    if name.is_empty() {
        return Err(GeoError::Validation("SQL identifier is empty".into()));
    }

    // Reject any metacharacters that could break out of an identifier context
    let forbidden = [
        ';', '\'', '"', ' ', '\t', '\n', '\r', '-', '/', '(', ')', ',', '=',
    ];
    for ch in name.chars() {
        if forbidden.contains(&ch) {
            return Err(GeoError::Validation(format!(
                "SQL identifier '{name}' rejected: contains forbidden character '{ch}'"
            )));
        }
    }

    // Allow: alphanumeric, underscore, dot (schema.table), colon (for schema::table)
    for ch in name.chars() {
        if !ch.is_alphanumeric() && ch != '_' && ch != '.' && ch != ':' {
            return Err(GeoError::Validation(format!(
                "SQL identifier '{name}' rejected: contains illegal character '{ch}'"
            )));
        }
    }

    // Must start with alpha or underscore
    let first = name.chars().next().unwrap();
    if !first.is_alphabetic() && first != '_' {
        return Err(GeoError::Validation(format!(
            "SQL identifier '{name}' rejected: must start with letter or underscore"
        )));
    }

    Ok(())
}

// Higher-level crates (geo-store, geo-ingest, etc.) provide their own
// From<sqlx::Error>, From<object_store::Error>, etc. conversions via
// a helper macro or manual impls. This keeps geo-core dependency-free.
//
// Example (in geo-store):
//   impl From<sqlx::Error> for GeoError {
//       fn from(e: sqlx::Error) -> Self { GeoError::Database(e.to_string()) }
//   }
