//! geo-adapter-iot: MQTT streaming ingestion.
#![allow(missing_docs)]
pub mod adapter;
#[cfg(feature = "mqtt")]
pub mod mqtt;
