//! 地震插件 — PGA/PGV 衰减、PSHA、地震目录工具。
pub mod config;
pub mod ground_motion;
pub mod psha;
pub mod seismicity;
pub mod tools;
pub mod trait_impl;

pub use config::SeismologyConfig;
pub use trait_impl::SeismologyPlugin;
