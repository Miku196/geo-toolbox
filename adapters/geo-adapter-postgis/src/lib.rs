//! geo-adapter-postgis: PostGIS / TimescaleDB / MinIO data backbone.
#![allow(missing_docs)]
pub mod audit;
pub mod batch_writer;
pub mod carbon_engine;
pub mod dvc;
pub mod postgis;
#[cfg(feature = "timescale")]
pub mod timescale;
#[cfg(feature = "minio")]
pub mod minio;
pub use audit::AuditTrail;
pub use batch_writer::BatchWriter;
pub use carbon_engine::{EmissionFactorRow, EmissionResult, FactorInfo, FactorInput, PostgisCarbonEngine};
pub use dvc::{dvc_available, dvc_hash, dvc_pull, dvc_repro, dvc_snapshot};
pub use postgis::{run_migrations, PostgisStore};
#[cfg(feature = "timescale")]
pub use timescale::{TimescalePool, GpsRecord, IotRecord};
#[cfg(feature = "minio")]
pub use minio::ObjectStoreClient;
