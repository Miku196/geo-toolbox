//! geo-plugin-forestry: 林业碳汇计量。
//!
//! 基于 IPCC 方法学的蓄积量估算、碳汇计量、CCER 报告。

#![allow(missing_docs)]

pub mod config;
pub mod forestry;
pub mod tools;
pub mod trait_impl;

pub use config::ForestryConfig;
pub use forestry::{ForestryPlugin, CarbonStockAssessment};
