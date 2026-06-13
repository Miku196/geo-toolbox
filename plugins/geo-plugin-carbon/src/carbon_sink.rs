//! Remote sensing carbon sink estimation.
//!
//! Estimates carbon sequestration using NDVI + forest inventory data.
//! Placeholder — the actual algorithm requires raster processing
//! via GDAL (geo-gdal crate).

use geo_core::errors::GeoResult;

/// Placeholder for carbon sink estimation.
///
/// Future implementation will:
/// 1. Read COG raster from MinIO
/// 2. Compute NDVI threshold mask
/// 3. Intersect with forest inventory polygons
/// 4. Apply biomass allometric equations
/// 5. Convert biomass → tCO₂e sequestration
pub fn estimate_carbon_sink(ndvi_cog_path: &str, forest_inventory_path: &str) -> GeoResult<String> {
    tracing::info!(
        "Carbon sink estimation: NDVI={ndvi_cog_path}, inventory={forest_inventory_path}"
    );
    Ok(format!(
        "Carbon sink: WIP (requires geo-gdal raster processing for {ndvi_cog_path})"
    ))
}
