//! geo-plugin-agri: 农业插件。
#![allow(missing_docs)]
pub mod agri;
pub mod config;
pub mod dssat;
pub mod soil;
pub mod tools;
pub mod trait_impl;
pub use agri::AgriPlugin;
pub use config::AgriConfig;
pub use dssat::{
    generate_cul, generate_sol, generate_wth, monthly_to_daily_wth, soil_from_scs_group,
    CultivarParams, DailyWeather, SoilLayer, SoilProfile, WeatherStation,
};
pub use soil::{
    crop_residue_c_input, k_factor_texture, k_from_temperature, ls_factor, r_factor_annual,
    soil_carbon_dynamics, usle_erosion, SoilCarbonResult, UsleResult,
};
