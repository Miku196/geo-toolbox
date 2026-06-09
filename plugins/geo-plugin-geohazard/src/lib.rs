//! geo-plugin-geohazard: 地质灾害插件。
#![allow(missing_docs)]
pub mod config; pub mod geohazard;
pub use config::GeohazardConfig;
pub use geohazard::GeohazardPlugin;
