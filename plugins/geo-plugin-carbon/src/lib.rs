//! geo-plugin-carbon: Carbon accounting engine.
//!
//! This plugin wraps geo-carbon-math (pure Rust) for WASM/embedded use.
//! For PostGIS-backed carbon calculations, use
//! `geo_adapter_postgis::PostgisCarbonEngine` instead.
#![allow(missing_docs)]
pub mod carbon_price;
pub mod carbon_sink;
pub mod ccer;
pub mod config;
pub mod lca;
pub mod plugin;
pub mod plume;
pub mod tools;
pub mod trait_impl;
pub mod vcs_gs;
pub use config::CarbonConfig;
pub use plugin::CarbonPlugin;
