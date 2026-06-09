//! geo-plugin-carbon: Carbon accounting engine.
#![allow(missing_docs)]
pub mod audit;
pub mod carbon_sink;
pub mod emission_factor;
pub mod lca;
pub use emission_factor::{CarbonEngine, EmissionFactorRow, EmissionResult, FactorInfo, FactorInput};
