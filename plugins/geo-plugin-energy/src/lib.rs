//! geo-plugin-energy: 新能源选址评估。
//!
//! 光伏选址：坡度 < 阈值 + 年太阳辐射 > 阈值
//! 风电选址：风速 + 坡度 + 离居民区距离

#![allow(missing_docs)]

pub mod config;
pub mod energy;
pub mod geothermal;
pub mod pvwatts;
pub mod tools;
pub mod trait_impl;
pub mod transmission;
pub mod turbine;
pub mod wake;

pub use config::EnergyConfig;
pub use energy::{EnergyPlugin, SolarAssessment, WindAssessment};
