#![allow(missing_docs)]

pub mod config;
pub mod coordinates;
pub mod solar;
pub mod tools;
pub mod trait_impl;

pub use config::PlanetaryConfig;
pub use coordinates::{
    celestial_to_geographic, lunar_coordinate_transform, mars_coordinate_transform,
    PlanetaryCoordinate, PlanetaryFrame,
};
pub use solar::{
    declination, extraterrestrial_radiation, hour_angle, solar_elevation_azimuth, SolarPosition,
};
pub use trait_impl::PlanetaryPlugin;
