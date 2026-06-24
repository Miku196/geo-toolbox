#![allow(missing_docs)]

pub mod ash_dispersion;
pub mod config;
pub mod hazard_zoning;
pub mod lava_flow;
pub mod tools;
pub mod trait_impl;

pub use ash_dispersion::{
    ash_dispersion_assessment, plume_concentration, settling_velocity, AshDispersionResult,
};
pub use config::VolcanologyConfig;
pub use hazard_zoning::{
    hazard_zone_classification, volcanic_hazard_zoning, HazardLevel, HazardZoneResult,
};
pub use lava_flow::{lava_flow_path, lava_flow_simulation, LavaFlowCell, LavaFlowSimulation};
pub use trait_impl::VolcanologyPlugin;
