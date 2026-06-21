pub mod config;
pub mod drought;
pub mod gcm;
pub mod idf;
pub mod kriging;
pub mod tools;
pub mod trait_impl;

pub use config::ClimateConfig;
pub use drought::{compute_pdsi, compute_spei, compute_spi, DroughtIndex, SpiResult};
pub use gcm::{delta_downscale, quantile_mapping, DownscaleResult, GcmProjection};
pub use idf::{idf_curve, idf_return_period, IdfParams, IdfResult};
pub use kriging::{
    ordinary_kriging, semivariogram, simple_kriging, KrigingResult, VariogramModel, VariogramParams,
};
