//! Emission factor types — multi-gas, GWP-aware, with uncertainty.
//!
//! Extends the simple tCO₂e/ha/yr model to the full GHG Protocol framework:
//! 1. Per-gas emission factors (CO₂, CH₄, N₂O, HFCs, PFCs, SF₆, NF₃)
//! 2. GWP conversion to CO₂-equivalent
//! 3. Uncertainty range (±X%) for Monte Carlo propagation

mod csv_loader;
mod emission_factor;
mod fuel;
mod gas;

#[cfg(test)]
mod tests;

pub use csv_loader::load_factors_from_csv;
pub use emission_factor::EmissionFactor;
pub use fuel::{FuelType, GridEmissionFactor};
pub use gas::{gwp100, EmissionScope, GasFactor, GreenhouseGas, GwpVersion};
