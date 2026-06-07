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

    /// Not-yet-implemented feature.
    #[error("Not implemented: {0}")]
    Unimplemented(String),

    /// Catch-all for library-specific errors that don't have a dedicated variant.
    #[error("{0}")]
    Other(String),
}

/// Convenience alias: `Result<T, GeoError>`.
pub type GeoResult<T> = Result<T, GeoError>;

// Higher-level crates (geo-store, geo-ingest, etc.) provide their own
// From<sqlx::Error>, From<object_store::Error>, etc. conversions via
// a helper macro or manual impls. This keeps geo-core dependency-free.
//
// Example (in geo-store):
//   impl From<sqlx::Error> for GeoError {
//       fn from(e: sqlx::Error) -> Self { GeoError::Database(e.to_string()) }
//   }
