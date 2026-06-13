//! geo-adapter-postgis: PostGIS / TimescaleDB / MinIO data backbone.
#![allow(missing_docs)]
pub mod adapter;
pub mod audit;
pub mod batch_writer;
pub mod carbon_engine;
pub mod dvc;
#[cfg(feature = "minio")]
pub mod minio;
pub mod postgis;
#[cfg(feature = "timescale")]
pub mod timescale;
pub mod tools;
pub use audit::AuditTrail;
pub use batch_writer::BatchWriter;
pub use carbon_engine::{
    EmissionFactorRow, EmissionResult, FactorInfo, FactorInput, PostgisCarbonEngine,
};
pub use dvc::{dvc_available, dvc_hash, dvc_pull, dvc_repro, dvc_snapshot};
#[cfg(feature = "minio")]
pub use minio::ObjectStoreClient;
pub use postgis::{run_migrations, PostgisStore};
#[cfg(feature = "timescale")]
pub use timescale::{GpsRecord, IotRecord, TimescalePool};
