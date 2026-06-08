//! geo-parquet: Cloud-native geospatial vector format.
//!
//! Reads and writes GeoParquet files with spatial metadata,
//! enabling predicate pushdown for spatial queries and seamless
//! integration with Arrow/DataFusion/Polars data pipelines.
//!
//! ## GeoParquet Specification
//!
//! GeoParquet extends Apache Parquet with geospatial metadata:
//! - Column-level geometry encoding metadata
//! - Bounding box coverage at row-group and file level
//! - CRS specification per geometry column
//!
//! ## Key Features
//!
//! - **Predicate pushdown**: Filter by spatial extent before reading data
//! - **Cloud-native**: Parquet's columnar layout + spatial index
//! - **Arrow integration**: Zero-copy into Polars/DataFusion pipelines
//! - **Performance**: 10-100x faster than Shapefile for large datasets
//!
//! ## Example
//!
//! ```rust,ignore
//! use geo_parquet::{GeoParquetReader, SpatialFilter};
//!
//! // Read only features intersecting Chengdu
//! let reader = GeoParquetReader::open("data.parquet")?;
//! let features = reader.read_with_filter(
//!     SpatialFilter::Bbox { min_x: 103.0, min_y: 30.0, max_x: 105.0, max_y: 31.0 }
//! )?;
//! ```

#![warn(missing_docs)]

pub mod metadata;
pub mod schema;
pub mod reader;
pub mod writer;
pub mod predicate;

pub use metadata::GeoParquetMetadata;
pub use schema::GeoSchema;
pub use reader::GeoParquetReader;
pub use writer::GeoParquetWriter;
pub use predicate::{SpatialFilter, SpatialPredicate};
