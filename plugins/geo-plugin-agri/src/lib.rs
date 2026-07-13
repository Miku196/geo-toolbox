//! geo-plugin-agri: 农业插件。
#![allow(missing_docs)]
pub mod agri;
pub mod config;
pub mod dssat;
pub mod soil;
pub mod tools;
pub use agri::AgriPlugin;
pub use config::AgriConfig;
// pub use dssat 中的类型现在通过 geo_core::traits 直接导入。
pub use soil::{
    crop_residue_c_input, k_factor_texture, k_from_temperature, ls_factor, r_factor_annual,
    soil_carbon_dynamics, usle_erosion, SoilCarbonResult, UsleResult,
};
