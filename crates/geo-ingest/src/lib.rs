//! geo-ingest: CamoFox / NMEA / MQTT data ingestion pipeline.
//!
//! Reads raw data from various sources, validates coordinates,
//! and feeds into [`geo_store::BatchWriter`] for PostgreSQL COPY insert.

#![warn(missing_docs)]

pub mod camofox;
pub mod nmea;
pub mod validator;

#[cfg(feature = "mqtt")]
pub mod mqtt;
