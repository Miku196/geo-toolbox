#![warn(missing_docs)]

//! geo-qgis: QGIS processing delegate.
//!
//! Two operating modes:
//!
//! 1. **gRPC / REST client** — connects to a long-running PyQGIS service
//!    for low-latency interactive processing (no cold start).
//!
//! 2. **Subprocess runner** — calls `qgis_process` CLI for batch processing,
//!    suitable for infrequent, large merges.
//!
//! ## Architecture
//!
//! ```text
//! geo-toolbox process qgis submit
//!   → HTTP POST /process → PyQGIS service → result
//!
//! geo-toolbox process qgis batch
//!   → qgis_process run → GPKG output
//! ```

pub mod grpc_client;
pub mod process_runner;

pub use grpc_client::QgisClient;
pub use process_runner::{BatchQgisRunner, QgisProcessConfig, QgisTool};
