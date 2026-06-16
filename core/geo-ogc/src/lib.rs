//! geo-ogc: OGC standard service interfaces.
//!
//! Pure-Rust implementations of:
//! - **WMS** (Web Map Service) — render maps as images
//! - **WFS** (Web Feature Service) — query and retrieve vector features
//! - **WPS** (Web Processing Service) — execute geospatial processing tasks
//!
//! ## Design
//!
//! Each service is a request → response pipeline:
//! 1. Parse OGC-standard request parameters (KVP or XML)
//! 2. Execute the operation (query data, render map, run process)
//! 3. Format the response per OGC specification
//!
//! Designed to work with any HTTP framework (Axum, Actix, etc.)
//! by producing serializable request/response types.
//!
//! ## Example (WMS GetCapabilities)
//!
//! ```rust,ignore
//! use geo_ogc::wms::{WmsService, WmsRequest};
//!
//! let service = WmsService::new("My Geo Server", "https://example.com/wms");
//! let request = WmsRequest::GetCapabilities;
//! let response = service.handle(request)?;
//! // → XML string with WMS 1.3.0 capabilities document
//! ```

#![warn(missing_docs)]

pub mod common;
pub mod wfs;
pub mod wms;
pub mod wmts;
pub mod wps;

pub use common::{OgcError, OgcException, ServiceType};
