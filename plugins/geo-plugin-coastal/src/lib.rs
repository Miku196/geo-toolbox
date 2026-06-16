//! geo-plugin-coastal: 海岸带变化监测。

pub mod blue_carbon;
pub mod coastal;
pub mod storm_surge;
pub mod tools;
pub mod trait_impl;
pub use coastal::{CoastalPlugin, ShorelineReport};
