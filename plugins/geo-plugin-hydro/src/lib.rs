#![allow(missing_docs)]
pub mod config;
pub mod hydro;
pub mod invest;
pub mod snowmelt;
pub mod tools;
pub mod trait_impl;
pub mod unit_hydrograph;
pub mod watershed;
pub mod groundwater;
pub mod scs_cn;
pub use config::HydroConfig;
pub use hydro::HydroPlugin;
pub use invest::{
    assess_carbon_storage, assess_invest, assess_water_yield, budyko_aet_p_ratio, compute_omega,
    compute_water_yield, default_carbon_pools, CarbonPoolDensity, CarbonStorageAssessment,
    InvestAssessment, WaterYieldAssessment,
};
pub use scs_cn::{
    adjust_cn_for_amc, assess_runoff, compute_runoff, compute_runoff_grid, compute_runoff_with_s,
    compute_s, get_cn_ii, ScsCnAssessment, SoilGroup, AMC,
};
pub use unit_hydrograph::{
    clark_uh, convolve_rainfall, scs_uh, snyder_uh, ScsUhParams, SnyderParams, UnitHydrograph,
};
pub use watershed::{d8_offset, extract_watershed, watershed_to_geojson, WatershedResult};
