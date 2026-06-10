//! geo-adapter-duckdb: 嵌入式本地空间分析引擎。
//!
//! 当前后端：SQLite（零编译依赖，2MB 二进制）。
//! 未来可切换 DuckDB 以获得完整 PostGIS 兼容性。
//!
//! 支持：内存模式、文件持久化、GeoJSON 导入、空间范围查询。

#![allow(missing_docs)]

mod adapter;
mod store;

pub use adapter::DuckDbAdapter;
pub use store::DuckDbStore;
