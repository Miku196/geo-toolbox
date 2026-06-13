//! Vector format conversion — ogr2ogr-equivalent operations.
//!
//! Converts between GeoJSON, GPKG, CSV, Shapefile, and other OGR-supported formats
//! via the `ogr2ogr` subprocess. Also supports basic spatial operations like clip,
//! reproject, and simplify that can be expressed as ogr2ogr options.

use geo_core::errors::{GeoError, GeoResult};
use std::path::{Path, PathBuf};

/// Supported OGR vector formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorFormat {
    /// GeoPackage (.gpkg).
    Gpkg,
    /// GeoJSON (.geojson).
    GeoJson,
    /// CSV with geometry columns.
    Csv,
    /// ESRI Shapefile (directory).
    Shapefile,
    /// FlatGeobuf (.fgb).
    FlatGeobuf,
    /// GeoJSONSeq (newline-delimited GeoJSON).
    GeoJsonSeq,
    /// DXF for CAD.
    Dxf,
    /// Keyhole Markup Language.
    Kml,
}

impl VectorFormat {
    /// The OGR driver name.
    fn driver_name(&self) -> &str {
        match self {
            VectorFormat::Gpkg => "GPKG",
            VectorFormat::GeoJson => "GeoJSON",
            VectorFormat::Csv => "CSV",
            VectorFormat::Shapefile => "ESRI Shapefile",
            VectorFormat::FlatGeobuf => "FlatGeobuf",
            VectorFormat::GeoJsonSeq => "GeoJSONSeq",
            VectorFormat::Dxf => "DXF",
            VectorFormat::Kml => "KML",
        }
    }

    /// Guess format from file extension.
    pub fn from_path(path: impl AsRef<Path>) -> Option<Self> {
        let ext = path.as_ref().extension()?.to_str()?.to_lowercase();
        match ext.as_str() {
            "gpkg" => Some(VectorFormat::Gpkg),
            "geojson" | "json" => Some(VectorFormat::GeoJson),
            "csv" => Some(VectorFormat::Csv),
            "shp" => Some(VectorFormat::Shapefile),
            "fgb" => Some(VectorFormat::FlatGeobuf),
            "geojsonl" | "jsonl" => Some(VectorFormat::GeoJsonSeq),
            "dxf" => Some(VectorFormat::Dxf),
            "kml" => Some(VectorFormat::Kml),
            _ => None,
        }
    }
}

/// Options for ogr2ogr conversions.
#[derive(Debug, Clone, Default)]
pub struct Ogr2OgrOptions {
    /// Target EPSG code (reprojects).
    pub target_epsg: Option<u16>,
    /// Source EPSG code (if not auto-detected).
    pub source_epsg: Option<u16>,
    /// SQL where clause filter.
    pub where_clause: Option<String>,
    /// Layer name(s) to convert.
    pub layers: Option<Vec<String>>,
    /// Simplify tolerance in target units (using Douglas-Peucker).
    pub simplify: Option<f64>,
    /// Overwrite the output file if exists.
    pub overwrite: bool,
    /// Limit to N features.
    pub limit: Option<usize>,
    /// Geometry column names (for CSV).
    pub geometry_columns: Option<(String, String)>, // (longitude, latitude)
    /// Skip failures (continue on feature errors).
    pub skip_failures: bool,
}

/// Vector operations via `ogr2ogr`.
pub struct VectorOps;

impl VectorOps {
    /// Convert between any two OGR-supported vector formats.
    ///
    /// Uses `ogr2ogr` with format auto-detection from file extensions.
    pub async fn convert(
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
        options: Option<Ogr2OgrOptions>,
    ) -> GeoResult<PathBuf> {
        let input = input.as_ref();
        let output = output.as_ref().to_path_buf();
        let opts = options.unwrap_or_default();

        if !input.exists() {
            return Err(GeoError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Input vector not found: {}", input.display()),
            )));
        }

        let output_format = VectorFormat::from_path(&output).ok_or_else(|| {
            GeoError::Validation(format!(
                "Cannot determine output format from extension: {}",
                output.display()
            ))
        })?;

        // Build ogr2ogr arguments.
        let mut args: Vec<String> = Vec::new();

        if opts.overwrite {
            args.push("-overwrite".into());
        }

        if opts.skip_failures {
            args.push("-skipfailures".into());
        }

        args.push("-f".into());
        args.push(output_format.driver_name().to_string());

        if let Some(simplify) = opts.simplify {
            args.push("-simplify".into());
            args.push(simplify.to_string());
        }

        if let Some(tgt_epsg) = opts.target_epsg {
            args.push("-t_srs".into());
            args.push(format!("EPSG:{tgt_epsg}"));
        }

        if let Some(src_epsg) = opts.source_epsg {
            args.push("-s_srs".into());
            args.push(format!("EPSG:{src_epsg}"));
        }

        if let Some(where_clause) = &opts.where_clause {
            args.push("-where".into());
            args.push(where_clause.clone());
        }

        if let Some(limit) = opts.limit {
            args.push("-limit".into());
            args.push(limit.to_string());
        }

        if let Some((lon_col, lat_col)) = &opts.geometry_columns {
            args.push("-oo".into());
            args.push(format!("GEOM_POSSIBLE_NAMES={lon_col},{lat_col}"));
            args.push("-oo".into());
            args.push("AUTODETECT_TYPE=YES".into());
        }

        // Layers
        if let Some(layers) = &opts.layers {
            for layer in layers {
                args.push(layer.clone());
            }
            args.push(input.to_string_lossy().to_string());
        }
        // If no layers specified, add output and input at the end.
        // Actually ogr2ogr syntax is: ogr2ogr [options] dst_datasource src_datasource [layers]
        // We already added dst at position 2 (after -f driver). Now add src.
        if opts.layers.is_none() {
            args.push(input.to_string_lossy().to_string());
        }

        Self::run_ogr2ogr(&args).await?;

        tracing::info!(
            "Vector converted: {} → {} ({})",
            input.display(),
            output.display(),
            output_format.driver_name()
        );

        Ok(output)
    }

    /// Merge multiple input files into a single output.
    pub async fn merge(
        inputs: &[impl AsRef<Path>],
        output: impl AsRef<Path>,
        target_epsg: Option<u16>,
    ) -> GeoResult<PathBuf> {
        let output = output.as_ref().to_path_buf();
        let output_format = VectorFormat::from_path(&output)
            .ok_or_else(|| GeoError::Validation("Unknown output format".into()))?;

        // Use the first file as primary, then append others.
        // Actually, ogr2ogr can't easily merge with append mode in one call.
        // Use a workaround: convert first file, then append others.
        if inputs.is_empty() {
            return Err(GeoError::Validation("No input files".into()));
        }

        // First, convert the first input (creates the output).
        let mut base_args = vec![
            "-f".to_string(),
            output_format.driver_name().to_string(),
            output.to_string_lossy().to_string(),
            inputs[0].as_ref().to_string_lossy().to_string(),
        ];

        if let Some(epsg) = target_epsg {
            base_args.insert(2, format!("EPSG:{epsg}"));
            base_args.insert(2, "-t_srs".to_string());
        }

        Self::run_ogr2ogr(&base_args).await?;

        // Append remaining inputs.
        for input in &inputs[1..] {
            let mut append_args = vec![
                "-f".to_string(),
                output_format.driver_name().to_string(),
                "-append".to_string(),
                output.to_string_lossy().to_string(),
                input.as_ref().to_string_lossy().to_string(),
            ];

            if let Some(epsg) = target_epsg {
                append_args.insert(3, format!("EPSG:{epsg}"));
                append_args.insert(3, "-t_srs".to_string());
            }

            Self::run_ogr2ogr(&append_args).await?;
        }

        tracing::info!("Merged {} inputs → {}", inputs.len(), output.display());
        Ok(output)
    }

    /// Get basic vector layer info (feature count, extent, CRS, fields).
    pub async fn info(input: impl AsRef<Path>) -> GeoResult<VectorInfo> {
        let input = input.as_ref();
        let output = Self::run_ogrinfo_capture(&[
            "-json",
            "-so", // summary only
            &input.to_string_lossy(),
        ])
        .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let info: serde_json::Value = serde_json::from_str(stdout.trim())
            .map_err(|e| GeoError::Other(format!("Failed to parse ogrinfo JSON: {e}")))?;

        let layers = info["layers"].as_array().cloned().unwrap_or_default();
        let layer_info: Vec<LayerInfo> = layers
            .iter()
            .map(|l| LayerInfo {
                name: l["name"].as_str().unwrap_or("unknown").to_string(),
                feature_count: l["featureCount"].as_u64().unwrap_or(0) as usize,
                geometry_type: l["geometryType"].as_str().unwrap_or("Unknown").to_string(),
                crs: l["geometryFields"]
                    .as_array()
                    .and_then(|gf| gf.first())
                    .and_then(|g| g["coordinateSystem"]["wkt"].as_str())
                    .map(|s| s.to_string()),
                fields: l["fields"]
                    .as_array()
                    .map(|f| {
                        f.iter()
                            .map(|field| {
                                (
                                    field["name"].as_str().unwrap_or("").to_string(),
                                    field["type"].as_str().unwrap_or("").to_string(),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            })
            .collect();

        Ok(VectorInfo {
            path: input.to_path_buf(),
            layers: layer_info,
        })
    }

    /// Run ogr2ogr and return success.
    async fn run_ogr2ogr(args: &[String]) -> GeoResult<()> {
        let output = tokio::process::Command::new("ogr2ogr")
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GeoError::ExternalProcess {
                command: format!("ogr2ogr {}", args.join(" ")),
                message: stderr.trim().to_string(),
            });
        }

        Ok(())
    }

    /// Run ogrinfo and return raw output.
    async fn run_ogrinfo_capture(args: &[&str]) -> GeoResult<std::process::Output> {
        let output = tokio::process::Command::new("ogrinfo")
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GeoError::ExternalProcess {
                command: format!("ogrinfo {}", args.join(" ")),
                message: stderr.trim().to_string(),
            });
        }

        Ok(output)
    }
}

/// Per-layer information from ogrinfo.
#[derive(Debug, Clone)]
pub struct LayerInfo {
    /// Layer name.
    pub name: String,
    /// Number of features.
    pub feature_count: usize,
    /// Geometry type (e.g., "Point", "Polygon").
    pub geometry_type: String,
    /// CRS WKT string, if available.
    pub crs: Option<String>,
    /// Field definitions as (name, type) pairs.
    pub fields: Vec<(String, String)>,
}

/// Summary information about a vector file.
#[derive(Debug, Clone)]
pub struct VectorInfo {
    /// Path to the file.
    pub path: PathBuf,
    /// Per-layer metadata.
    pub layers: Vec<LayerInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_format_detection() {
        assert_eq!(
            VectorFormat::from_path("data.gpkg"),
            Some(VectorFormat::Gpkg)
        );
        assert_eq!(
            VectorFormat::from_path("data.geojson"),
            Some(VectorFormat::GeoJson)
        );
        assert_eq!(VectorFormat::from_path("data.csv"), Some(VectorFormat::Csv));
        assert_eq!(
            VectorFormat::from_path("data.shp"),
            Some(VectorFormat::Shapefile)
        );
        assert_eq!(VectorFormat::from_path("data.unknown"), None);
    }

    #[test]
    fn test_ogr_options_default() {
        let opts = Ogr2OgrOptions::default();
        assert!(opts.target_epsg.is_none());
        assert!(!opts.overwrite);
        assert!(!opts.skip_failures);
    }
}
