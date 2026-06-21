pub mod config;
pub mod gcm;
pub mod idf;
pub mod drought;
pub mod kriging;
pub mod trait_impl;
pub mod tools;

pub use config::ClimateConfig;
pub use gcm::{delta_downscale, quantile_mapping, GcmProjection, DownscaleResult};
pub use idf::{idf_curve, idf_return_period, IdfParams, IdfResult};
pub use drought::{compute_spi, compute_spei, compute_pdsi, DroughtIndex, SpiResult};
pub use kriging::{ordinary_kriging, simple_kriging, semivariogram, VariogramModel, KrigingResult, VariogramParams};
