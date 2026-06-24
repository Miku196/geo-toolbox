//! High-level LiDAR classification workflows.
//!
//! Wraps common PDAL pipeline recipes into single-call operations.

use crate::adapter::{PdalAdapter, PdalError};
use crate::pipeline::PdalPipeline;

/// Run a full ground-to-DEM pipeline on a LiDAR point cloud:
///
/// 1. SMRF ground classification (separates ground from non-ground)
/// 2. Rasterize ground points to a GeoTIFF DEM
///
/// Returns `Ok(())` on success or `PdalError` on pipeline failure.
pub async fn ground_to_dem(
    adapter: &PdalAdapter,
    input_las: &str,
    output_dem: &str,
    resolution: f64,
) -> Result<(), PdalError> {
    let pipeline = PdalPipeline::new()
        .reader_las(input_las)
        .smrf()
        .writers_gdal(output_dem, resolution)
        .to_json()?;

    adapter.run_pipeline(&pipeline).await?;
    Ok(())
}

/// Run SMRF ground classification and write the classified point cloud.
pub async fn classify_and_save(
    adapter: &PdalAdapter,
    input_las: &str,
    output_las: &str,
) -> Result<(), PdalError> {
    let pipeline = PdalPipeline::new()
        .reader_las(input_las)
        .smrf()
        .writers_las(output_las)
        .to_json()?;

    adapter.run_pipeline(&pipeline).await?;
    Ok(())
}

/// Decimate a point cloud by keeping every Nth point, then optionally run SMRF.
pub async fn decimate_and_classify(
    adapter: &PdalAdapter,
    input_las: &str,
    output_las: &str,
    step: usize,
) -> Result<(), PdalError> {
    let pipeline = PdalPipeline::new()
        .reader_las(input_las)
        .filters_decimation(step)
        .smrf()
        .writers_las(output_las)
        .to_json()?;

    adapter.run_pipeline(&pipeline).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ground_to_dem_pipeline_builds() {
        let pipeline = PdalPipeline::new()
            .reader_las("test.las")
            .smrf()
            .writers_gdal("dem.tif", 1.0)
            .to_json();

        assert!(pipeline.is_ok());
        let json = pipeline.unwrap();
        assert!(json.contains("filters.smrf"));
        assert!(json.contains("writers.gdal"));
    }

    #[test]
    fn test_classify_and_save_pipeline_builds() {
        let pipeline = PdalPipeline::new()
            .reader_las("raw.las")
            .smrf()
            .writers_las("classified.laz")
            .to_json();

        assert!(pipeline.is_ok());
        let json = pipeline.unwrap();
        assert!(json.contains("writers.las"));
    }

    #[test]
    fn test_decimate_and_classify_pipeline_builds() {
        let pipeline = PdalPipeline::new()
            .reader_las("dense.las")
            .filters_decimation(10)
            .smrf()
            .writers_las("sparse.laz")
            .to_json();

        assert!(pipeline.is_ok());
        let json = pipeline.unwrap();
        assert!(json.contains("filters.decimation"));
        assert!(json.contains("filters.smrf"));
    }
}
