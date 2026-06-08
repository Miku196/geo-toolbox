//! geo-carbon-core: Pure-Rust carbon accounting engine.
//!
//! Implements IPCC Tier 1 emission factor methodology with
//! zero external dependencies at runtime — no database,
//! no network, no file system required.
//!
//! Designed for:
//! - WASM/browser environments
//! - Embedded systems
//! - Python/Node.js bindings (via PyO3 / napi-rs)
//! - Server-side use as a library
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use geo_carbon_core::{CarbonEngine, EmissionFactor, GeoFeature};
//!
//! let engine = CarbonEngine::new();
//!
//! let factors = vec![
//!     EmissionFactor::new("forest", 5.0, "IPCC_2019"),
//!     EmissionFactor::new("grassland", -1.0, "IPCC_2019"),
//! ];
//!
//! let geojson_poly = r#"{"type":"Polygon","coordinates":[[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}"#;
//!
//! let features = vec![
//!     GeoFeature::new("forest", geojson_poly).unwrap(),
//!     GeoFeature::new("grassland", geojson_poly).unwrap(),
//! ];
//!
//! let report = engine.calculate(&features, &factors, 2025)?;
//! // → CarbonReport { total_area_ha, total_emission_tco2e, classes, ... }
//! # Ok::<(), String>(())
//! ```

#![warn(missing_docs)]

mod factor;
mod feature;
mod engine;
mod report;

pub use factor::EmissionFactor;
pub use feature::GeoFeature;
pub use engine::CarbonEngine;
pub use report::{CarbonReport, ClassResult, FactorSourceUnit};
