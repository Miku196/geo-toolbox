//! geo-adapter-stac: STAC (SpatioTemporal Asset Catalog) API 客户端。
//!
//! 对接微软 Planetary Computer、NASA CMR、Element84 等 STAC 目录，
//! 按 AOI + 时间范围搜索卫星影像。
//!
//! ## 示例
//!
//! ```rust,ignore
//! use geo_adapter_stac::StacClient;
//!
//! let client = StacClient::new("https://planetarycomputer.microsoft.com/api/stac/v1");
//! let items = client.search("sentinel-2-l2a", 104.0, 30.0, 105.0, 31.0, "2025-01-01", "2025-06-30", 10).await?;
//! for item in &items {
//!     println!("{} - {}%", item.id, item.cloud_cover.unwrap_or(0.0));
//! }
//! ```

#![allow(missing_docs)]

mod adapter;
mod client;
pub mod tools;

pub use adapter::StacAdapter;
pub use client::{StacClient, StacCollection, StacItem};
