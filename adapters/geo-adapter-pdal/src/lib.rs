//! geo-adapter-pdal — PDAL point cloud adapter.
//!
//! Wraps the `pdal` CLI for LiDAR LAS/LAZ format read, write, and pipeline processing.
//! Uses subprocess execution with JSON pipeline definitions,
//! following the same subprocess pattern as geo-adapter-qgis.

pub mod adapter;
pub mod pipeline;

pub use adapter::PdalAdapter;
pub use pipeline::{
    LasHeader, LasPoint, LasStats, PdalFilter, PdalPipeline, PdalReader, PdalStage, PdalWriter,
};
