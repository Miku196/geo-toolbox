//! geo-carbon-math: 纯 Rust 碳核算公式引擎。
//!
//! 实现 IPCC Tier 1 排放因子方法，零数据库/网络/文件系统依赖。
//!
//! 可用于：
//! - WASM/浏览器环境
//! - 嵌入式系统
//! - 服务端作为库调用
//! - 作为 geo-plugin-carbon 的底层引擎
//!
//! ## 示例
//!
//! ```rust,no_run
//! use geo_carbon_math::{CarbonEngine, EmissionFactor, GeoFeature, CarbonReport};
//!
//! let engine = CarbonEngine::new();
//! let factors = vec![
//!     EmissionFactor::new("forest", 5.0, "IPCC_2019"),
//!     EmissionFactor::new("grassland", -1.0, "IPCC_2019"),
//! ];
//! let geom = r#"{"type":"Polygon","coordinates":[[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}"#;
//! let features = vec![
//!     GeoFeature::new("forest", geom).unwrap(),
//!     GeoFeature::new("grassland", geom).unwrap(),
//! ];
//! let report = engine.calculate(&features, &factors, 2025).unwrap();
//! ```

#![warn(missing_docs)]

mod config;
mod engine;
mod factor;
mod feature;
pub mod pools;
mod report;
pub mod scenarios;
pub mod tools;
pub mod vcs;

pub use config::CarbonParams;
pub use engine::CarbonEngine;
pub use factor::{
    gwp100, load_factors_from_csv, EmissionFactor, EmissionScope, FuelType, GasFactor,
    GreenhouseGas, GridEmissionFactor, GwpVersion,
};
pub use feature::{compute_area_ha, GeoFeature};
pub use pools::{
    compute_agb_tco2e_ha, compute_bgb_tco2e_ha, compute_deadwood_decay, compute_deadwood_tco2e_ha,
    compute_litter_tco2e_ha, compute_litter_turnover, compute_soc_tc_ha, compute_soc_tco2e_ha,
    compute_soc_transition, tc_to_tco2e, tco2e_to_tc, BiomassParams, CarbonPool, MultiPoolChange,
    MultiPoolStock, PoolChange, PoolStock, SocParams,
};
pub use report::{
    AuditEntry, CarbonReport, ClassResult, FactorSourceUnit, GasBreakdown, ScopeSummary,
};
pub use scenarios::{
    compute_scenario, CarbonScenario, EcoZone, LandState, ScenarioInput, ScenarioResult,
};
pub use vcs::{
    default_methodology, match_methodologies, CcbBenefit, CcbCertification, MethodologyConfig,
    VcsMethodology, VcsProjectSummary,
};
