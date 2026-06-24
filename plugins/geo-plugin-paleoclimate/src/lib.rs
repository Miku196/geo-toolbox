#![allow(missing_docs)]

pub mod config;
pub mod paleocoastline;
pub mod proxies;
pub mod sea_level;
pub mod tools;
pub mod trait_impl;

pub use config::PaleoclimateConfig;
pub use paleocoastline::paleocoastline_flooding;
pub use proxies::{d18o_to_sst, ice_core_temp_anomaly, proxy_temperature};
pub use sea_level::{
    glacial_isostatic_adjustment, lgm_sea_level_map, sea_level_reconstruction,
    SeaLevelReconstruction,
};
pub use trait_impl::PaleoclimatePlugin;
