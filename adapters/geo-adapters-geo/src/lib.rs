//! Geospatial adapters: PostGIS, GDAL CLI, PDAL, GEE.

#[cfg(feature = "postgis")]
pub mod postgis;

#[cfg(feature = "gdal")]
pub mod gdal;

#[cfg(feature = "pdal")]
pub mod pdal;

#[cfg(feature = "gee")]
pub mod gee;
