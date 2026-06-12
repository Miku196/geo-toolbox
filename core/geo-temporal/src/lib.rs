//! geo-temporal: 时空序列分析。
//!
//! 提供遥感时间序列的基础数学工具：
//! - 趋势分析（线性回归斜率 + Mann-Kendall 显著性检验）
//! - 突变检测（逐年差值 + 阈值判断）
//! - 季节分解（移动平均去季节项）
//!
//! ## 示例
//!
//! ```rust,ignore
//! use geo_temporal::trend::mann_kendall;
//!
//! let ndvi_series = vec![0.32, 0.35, 0.38, 0.41, 0.45]; // 5 年
//! let (tau, p_value) = mann_kendall(&ndvi_series);
//! if p_value < 0.05 { println!("显著恢复趋势"); }
//! ```

#![warn(missing_docs)]

pub mod trend;
pub mod decompose;
pub mod raster_ts;
pub mod tools;

pub use trend::{linear_trend, mann_kendall, TrendResult};
pub use decompose::{seasonal_decompose, DecomposeResult};
pub use raster_ts::{RasterTimeSeries, TimeStep};
