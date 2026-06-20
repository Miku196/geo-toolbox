//! Structured observability conventions for geo-toolbox.
//!
//! This module defines standard patterns for structured tracing, spans,
//! and subscriber configuration. Every crate should follow these conventions.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use tracing::{info, warn, error, instrument};
//!
//! #[instrument(skip(input), fields(table = %table_name, feature_count = input.len()))]
//! fn ingest_features(table_name: &str, input: &[GeoFeature]) -> GeoResult<usize> {
//!     // ...
//!     info!(ingested = count, "features ingested");
//!     Ok(count)
//! }
//! ```
//!
//! # Conventions
//!
//! ### Event Levels
//! - `ERROR`: operation failed, user action needed
//! - `WARN`: degraded but running (e.g. env var missing, fallback used)
//! - `INFO`: normal operations (export, import, rebuild complete)
//! - `DEBUG`: internal state, SQL queries, raster params
//! - `TRACE`: per-feature/per-pixel details
//!
//! ### Structured Fields (use these keys consistently)
//!
//! | Key | Type | Meaning |
//! |-----|------|---------|
//! | `path` | `%path` | File system path |
//! | `table` | `%str` | Database / store table name |
//! | `count` | `u64` / `usize` | Entity / row / feature count |
//! | `latency_ms` | `f64` | Operation duration in ms |
//! | `error` | `%err` | Error details (use `%` for Display) |
//! | `bbox` | `%str` | Bounding box as string |
//! | `crs` | `%str` | Coordinate reference system identifier |
//! | `source` | `%str` | Data source name |
//! | `bytes` | `u64` | Data size in bytes |

/// Standard subscriber builder for geo-toolbox applications.
///
/// Only available when the `tracing-init` feature is enabled (executable crates).
/// Uses env-filter for runtime level control:
/// ```bash
/// RUST_LOG=geo_carbon::scenarios=debug,warn cargo run
/// ```
///
/// In production, switch to JSON output by setting `GEO_LOG_FORMAT=json`.
#[cfg(feature = "tracing-init")]
pub fn init_default_subscriber() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    if std::env::var("GEO_LOG_FORMAT").as_deref() == Ok("json") {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .with_file(false)
            .with_line_number(false)
            .init();
    }
}

/// Macro for capturing operation latency.
///
/// Usage:
/// ```rust,ignore
/// let (result, elapsed) = geo_core::observability::timed_op!(|| { do_work() });
/// info!(latency_ms = elapsed.as_millis(), "operation complete");
/// ```
#[macro_export]
macro_rules! timed_op {
    ($body:expr) => {{
        let start = std::time::Instant::now();
        let result = $body;
        let elapsed = start.elapsed();
        (result, elapsed)
    }};
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_timed_op() {
        let (v, _t) = timed_op!({ 42 });
        assert_eq!(v, 42);
    }
}
