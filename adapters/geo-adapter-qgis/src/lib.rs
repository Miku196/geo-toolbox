//! geo-adapter-qgis: QGIS processing bridge.
#![allow(missing_docs)]
pub mod grpc_client;
pub mod process_runner;
pub mod tools;
pub use grpc_client::QgisClient;
pub use process_runner::{BatchQgisRunner, QgisProcessConfig, QgisTool};
