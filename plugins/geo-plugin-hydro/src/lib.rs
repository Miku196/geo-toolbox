//! geo-plugin-hydro: 水文插件。
#![allow(missing_docs)]
pub mod config; pub mod hydro; pub mod tools; pub mod trait_impl;
pub use config::HydroConfig;
pub use hydro::HydroPlugin;
