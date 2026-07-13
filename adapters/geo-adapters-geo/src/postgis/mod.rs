//! PostGIS / TimescaleDB / MinIO data backbone.
#![allow(missing_docs)]
pub mod postgis_adapter;
pub mod postgis_audit;
pub mod postgis_batch_writer;
pub mod postgis_carbon_engine;
pub mod postgis_dvc;
#[cfg(feature = "minio")]
pub mod postgis_minio;
pub mod postgis_postgis;
#[cfg(feature = "timescale")]
pub mod postgis_timescale;
#[cfg(feature = "postgis")]
pub mod postgis_tools;
pub use postgis_tools::register_tools;

pub use postgis_adapter::PostgisAdapter;
pub use postgis_audit::AuditTrail;
pub use postgis_batch_writer::BatchWriter;
pub use postgis_carbon_engine::{
    EmissionFactorRow, EmissionResult, FactorInfo, FactorInput, PostgisCarbonEngine,
};
pub use postgis_dvc::{dvc_available, dvc_hash, dvc_pull, dvc_repro, dvc_snapshot};
#[cfg(feature = "minio")]
pub use postgis_minio::ObjectStoreClient;
pub use postgis_postgis::{run_migrations, PostgisStore};
#[cfg(feature = "timescale")]
pub use postgis_timescale::{GpsRecord, IotRecord, TimescalePool};
