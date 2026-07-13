#![warn(missing_docs)]

//! geo-core: Shared types, geometry operations, and CRS registry.
//!
//! This crate is the foundation of geo-toolbox. All other crates depend on it.
//! It provides:
//!
//! - Unified error types ([`GeoError`])
//! - CRS registry with built-in common coordinate systems ([`crs::CrsRegistry`])
//! - Geometry type aliases and validation ([`types`])

pub mod config;
pub mod crs;
pub mod errors;
pub mod guard;
pub mod health;
pub mod observability;
pub mod plugin;
pub mod traits;
pub mod types;

pub use errors::{FacadeResult, GeoError, GeoResult, GeometryFacadeError};
pub use traits::{
    CultivarParams, DailyWeather, DssatGenerator, ModflowGenerator,
    ModflowGrid, ModflowStressPeriod, SoilLayer, SoilProfile, WeatherStation,
};
