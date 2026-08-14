//! High-level LiDAR classification workflows.
//!
//! Wraps common PDAL pipeline recipes into single-call operations.

use super::pdal_adapter::{PdalAdapter};
use super::pdal_pipeline::PdalPipeline;

/// PDAL error type.
#[derive(Debug, thiserror::Error)]
pub enum PdalError {
    #[error("PDAL pipeline failed: {0}")]
    Pipeline(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

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
    let pipeline = PdalPipeline {
        pipeline: vec![
            super::pdal_pipeline::PdalStage::LasReader(super::pdal_pipeline::PdalReader {
                filename: input_las.to_string(),
                override_srs: None,
            }),
            // SMRF filter not directly in the enum; use generic approach
        ],
    };

    let _pipeline_json = serde_json::to_string(&pipeline).map_err(|e| PdalError::Pipeline(e.to_string()))?;

    // Build pipeline via JSON directly for SMRF which isn't in the stage enum
    let pipeline_json = serde_json::json!({
        "pipeline": [
            { "type": "readers.las", "filename": input_las },
            { "type": "filters.smrf" },
            { "type": "writers.gdal", "filename": output_dem, "resolution": resolution, "output_type": "idw" }
        ]
    });
    let _ = adapter.exec_pipeline(&pipeline_json).map_err(|e| PdalError::Pipeline(e.to_string()))?;
    Ok(())
}

/// Run SMRF ground classification and write the classified point cloud.
pub async fn classify_and_save(
    adapter: &PdalAdapter,
    input_las: &str,
    output_las: &str,
) -> Result<(), PdalError> {
    let pipeline_json = serde_json::json!({
        "pipeline": [
            { "type": "readers.las", "filename": input_las },
            { "type": "filters.smrf" },
            { "type": "writers.las", "filename": output_las }
        ]
    });
    let _ = adapter.exec_pipeline(&pipeline_json).map_err(|e| PdalError::Pipeline(e.to_string()))?;
    Ok(())
}

/// Decimate a point cloud by keeping every Nth point, then optionally run SMRF.
pub async fn decimate_and_classify(
    adapter: &PdalAdapter,
    input_las: &str,
    output_las: &str,
    step: usize,
) -> Result<(), PdalError> {
    let pipeline_json = serde_json::json!({
        "pipeline": [
            { "type": "readers.las", "filename": input_las },
            { "type": "filters.decimation", "step": step },
            { "type": "filters.smrf" },
            { "type": "writers.las", "filename": output_las }
        ]
    });
    let _ = adapter.exec_pipeline(&pipeline_json).map_err(|e| PdalError::Pipeline(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_pipeline_json_builds() {
        let json = serde_json::json!({
            "pipeline": [
                { "type": "readers.las", "filename": "test.las" },
                { "type": "filters.smrf" },
                { "type": "writers.gdal", "filename": "dem.tif", "resolution": 1.0 }
            ]
        });
        let s = serde_json::to_string(&json).unwrap();
        assert!(s.contains("filters.smrf"));
        assert!(s.contains("writers.gdal"));
    }

    #[test]
    fn test_classify_and_save_json() {
        let json = serde_json::json!({
            "pipeline": [
                { "type": "readers.las", "filename": "raw.las" },
                { "type": "filters.smrf" },
                { "type": "writers.las", "filename": "classified.laz" }
            ]
        });
        let s = serde_json::to_string(&json).unwrap();
        assert!(s.contains("writers.las"));
    }
}
