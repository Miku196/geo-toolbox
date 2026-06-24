//! PDAL pipeline types — structured definitions for JSON pipeline construction.

use serde::{Deserialize, Serialize};

/// A PDAL pipeline is a list of stages executed in order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdalPipeline {
    pub pipeline: Vec<PdalStage>,
}

/// A single stage in a PDAL pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PdalStage {
    /// Reader: input data source (readers.las, readers.bpf, etc.)
    #[serde(rename = "readers.las")]
    LasReader(PdalReader),
    /// Writer: output data sink (writers.las, writers.geojson, etc.)
    #[serde(rename = "writers.las")]
    LasWriter(PdalWriter),
    /// Writer: GeoJSON output
    #[serde(rename = "writers.geojson")]
    GeoJsonWriter(PdalWriter),
    /// Filter: transform/filter (filters.stats, filters.outlier, etc.)
    #[serde(rename = "filters.stats")]
    StatsFilter(PdalFilter),
    /// Filter: outlier removal
    #[serde(rename = "filters.outlier")]
    OutlierFilter(PdalFilter),
    /// Filter: reprojection
    #[serde(rename = "filters.reprojection")]
    ReprojectionFilter(PdalFilter),
    /// Filter: decimation
    #[serde(rename = "filters.decimation")]
    DecimationFilter(PdalFilter),
}

/// Reader stage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdalReader {
    pub filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_srs: Option<String>,
}

/// Writer stage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdalWriter {
    pub filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_dims: Option<String>,
}

/// Filter stage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdalFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dims: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mean_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiplier: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_srs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_srs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<u32>,
}

/// LAS file header metadata.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LasHeader {
    pub file_size: Option<u64>,
    pub file_source_id: Option<u16>,
    pub global_encoding: Option<u16>,
    pub project_id: Option<String>,
    pub version: Option<String>,
    pub system_id: Option<String>,
    pub software_id: Option<String>,
    pub creation_doy: Option<u16>,
    pub creation_year: Option<u16>,
    pub header_size: Option<u16>,
    pub point_count: Option<u64>,
    pub point_length: Option<u16>,
    pub scale_x: Option<f64>,
    pub scale_y: Option<f64>,
    pub scale_z: Option<f64>,
    pub offset_x: Option<f64>,
    pub offset_y: Option<f64>,
    pub offset_z: Option<f64>,
    pub maxx: Option<f64>,
    pub maxy: Option<f64>,
    pub maxz: Option<f64>,
    pub minx: Option<f64>,
    pub miny: Option<f64>,
    pub minz: Option<f64>,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// Per-dimension statistics.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DimensionStats {
    pub average: Option<f64>,
    pub count: Option<u64>,
    pub maximum: Option<f64>,
    pub minimum: Option<f64>,
    pub name: Option<String>,
    pub stddev: Option<f64>,
    pub variance: Option<f64>,
}

/// LAS file statistics (from filters.stats).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LasStats {
    pub filename: Option<String>,
    pub stats: Option<Vec<DimensionStats>>,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// A single LiDAR point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LasPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intensity: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_number: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number_of_returns: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scan_angle_rank: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gps_time: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub red: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub green: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blue: Option<u16>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_serde() {
        let pipeline = PdalPipeline {
            pipeline: vec![
                PdalStage::LasReader(PdalReader {
                    filename: "input.las".into(),
                    override_srs: Some("EPSG:4326".into()),
                }),
                PdalStage::StatsFilter(PdalFilter {
                    dims: Some("X,Y,Z".into()),
                    mean_k: None,
                    multiplier: None,
                    in_srs: None,
                    out_srs: None,
                    step: None,
                }),
            ],
        };

        let json = serde_json::to_string(&pipeline).unwrap();
        assert!(json.contains("readers.las"));
        assert!(json.contains("input.las"));
        assert!(json.contains("filters.stats"));
    }

    #[test]
    fn test_las_header_deser() {
        let json = r#"{
            "file_size": 123456,
            "point_count": 1000000,
            "minx": 100.0,
            "maxx": 200.0,
            "miny": 30.0,
            "maxy": 40.0,
            "minz": -10.0,
            "maxz": 500.0,
            "scale_x": 0.01,
            "scale_y": 0.01,
            "scale_z": 0.01,
            "offset_x": 150.0,
            "offset_y": 35.0,
            "offset_z": 0.0,
            "version": "1.4"
        }"#;

        let header: LasHeader = serde_json::from_str(json).unwrap();
        assert_eq!(header.point_count, Some(1_000_000));
        assert_eq!(header.minx, Some(100.0));
        assert_eq!(header.maxx, Some(200.0));
        assert_eq!(header.version.as_deref(), Some("1.4"));
    }
}
