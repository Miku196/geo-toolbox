//! geo-carbon: Carbon accounting engine.
//!
//! Implements IPCC emission factor methodology (Pipeline A),
//! LCA via brightway2 subprocess (Pipeline B), and remote sensing
//! carbon sink estimation (Pipeline C).
//!
//! ## Core: Emission Factor Method
//!
//! One SQL query completes the entire calculation:
//! 1. Read landcover data from `spatial_assets`
//! 2. Join with emission factors from `factor_registry`
//! 3. Compute area in equal-area projection (EPSG:3405)
//! 4. Multiply area × factor → tCO₂e
//! 5. Write results to `carbon_accounting_results`
//! 6. Return full audit trail

#![warn(missing_docs)]

pub mod audit;
pub mod carbon_sink;
pub mod emission_factor;
pub mod lca;

pub use emission_factor::{CarbonEngine, EmissionFactorRow, EmissionResult, FactorInfo, FactorInput};
