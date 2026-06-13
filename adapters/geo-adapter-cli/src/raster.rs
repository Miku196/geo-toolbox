//! Raster operations: COG conversion, algebra, band extraction, reprojection.
//!
//! All operations default to subprocess calls to `gdal_translate`, `gdal_calc.py`,
//! `gdalwarp`, etc. When the `gdal-bindings` feature is enabled, some operations
//! use the Rust `gdal` crate for better performance.

use geo_core::errors::{GeoError, GeoResult};
use std::path::{Path, PathBuf};
use std::process::Output;

/// Raster format for conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RasterFormat {
    /// Cloud-Optimized GeoTIFF (COG).
    Cog,
    /// Standard GeoTIFF.
    GeoTiff,
    /// JPEG 2000.
    Jp2,
    /// PNG.
    Png,
    /// Erdas Imagine (.img).
    Imagine,
}

/// Band data type for raster calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    /// 8-bit unsigned integer.
    Byte,
    /// 16-bit unsigned integer.
    UInt16,
    /// 16-bit signed integer.
    Int16,
    /// 32-bit unsigned integer.
    UInt32,
    /// 32-bit signed integer.
    Int32,
    /// 32-bit floating point.
    Float32,
    /// 64-bit floating point.
    Float64,
}

/// COG creation options.
#[derive(Debug, Clone)]
pub struct CogOptions {
    /// Compression type (default: DEFLATE).
    pub compression: String,
    /// Compression level (1-9, default: 6).
    pub compress_level: u8,
    /// Build internal overviews (default: true).
    pub overviews: bool,
    /// Tile size (default: 256).
    pub tile_size: u16,
    /// Block size in rows (default: 256).
    pub block_size: u16,
}

impl Default for CogOptions {
    fn default() -> Self {
        Self {
            compression: "DEFLATE".into(),
            compress_level: 6,
            overviews: true,
            tile_size: 256,
            block_size: 256,
        }
    }
}

/// Raster operation utilities via GDAL CLI.
pub struct RasterOps;

impl RasterOps {
    /// Convert any raster to Cloud-Optimized GeoTIFF (COG).
    ///
    /// Uses `gdal_translate -of COG` with optional compression and overviews.
    pub async fn to_cog(
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
        options: Option<CogOptions>,
    ) -> GeoResult<PathBuf> {
        let opts = options.unwrap_or_default();
        let output = output.as_ref().to_path_buf();
        let input = input.as_ref().to_path_buf();

        if !input.exists() {
            return Err(GeoError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Input raster not found: {}", input.display()),
            )));
        }

        let mut args = vec![
            "-of".to_string(),
            "COG".to_string(),
            "-co".to_string(),
            format!("COMPRESS={}", opts.compression),
            "-co".to_string(),
            format!("LEVEL={}", opts.compress_level),
            input.to_string_lossy().to_string(),
            output.to_string_lossy().to_string(),
        ];

        if opts.overviews {
            args.insert(2, "-co".to_string());
            args.insert(3, "OVERVIEWS=AUTO".to_string());
        }

        Self::run_gdal("gdal_translate", &args).await?;

        tracing::info!("COG created: {}", output.display());
        Ok(output)
    }

    /// Reproject a raster to a different CRS.
    pub async fn reproject(
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
        target_epsg: u16,
        resolution: Option<f64>,
    ) -> GeoResult<PathBuf> {
        let input = input.as_ref();
        let output = output.as_ref();

        if !input.exists() {
            return Err(GeoError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Input raster not found: {}", input.display()),
            )));
        }

        let mut args = vec![
            "-t_srs".to_string(),
            format!("EPSG:{target_epsg}"),
            "-of".to_string(),
            "COG".to_string(),
            "-co".to_string(),
            "COMPRESS=DEFLATE".to_string(),
            input.to_string_lossy().to_string(),
            output.to_string_lossy().to_string(),
        ];

        if let Some(res) = resolution {
            args.insert(0, "-tr".to_string());
            args.insert(1, res.to_string());
            args.insert(2, res.to_string());
        }

        Self::run_gdal("gdalwarp", &args).await?;

        tracing::info!(
            "Raster reprojected to EPSG:{target_epsg}: {}",
            output.display()
        );
        Ok(output.to_path_buf())
    }

    /// Extract a single band from a multi-band raster.
    pub async fn extract_band(
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
        band: u16,
    ) -> GeoResult<PathBuf> {
        let output = output.as_ref().to_path_buf();
        let input = input.as_ref();

        Self::run_gdal(
            "gdal_translate",
            &[
                "-b".to_string(),
                band.to_string(),
                "-of".to_string(),
                "COG".to_string(),
                "-co".to_string(),
                "COMPRESS=DEFLATE".to_string(),
                input.to_string_lossy().to_string(),
                output.to_string_lossy().to_string(),
            ],
        )
        .await?;

        tracing::info!("Band {band} extracted: {}", output.display());
        Ok(output)
    }

    /// Resample a raster to a new resolution.
    pub async fn resample(
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
        x_resolution: f64,
        y_resolution: f64,
        resampling: &str,
    ) -> GeoResult<PathBuf> {
        let output = output.as_ref().to_path_buf();

        Self::run_gdal(
            "gdalwarp",
            &[
                "-tr".to_string(),
                x_resolution.to_string(),
                y_resolution.to_string(),
                "-r".to_string(),
                resampling.to_string(),
                "-of".to_string(),
                "COG".to_string(),
                "-co".to_string(),
                "COMPRESS=DEFLATE".to_string(),
                input.as_ref().to_string_lossy().to_string(),
                output.to_string_lossy().to_string(),
            ],
        )
        .await?;

        tracing::info!("Raster resampled: {}", output.display());
        Ok(output)
    }

    /// Merge multiple rasters into a mosaic.
    pub async fn merge(
        inputs: &[impl AsRef<Path>],
        output: impl AsRef<Path>,
    ) -> GeoResult<PathBuf> {
        let output = output.as_ref().to_path_buf();

        let mut args: Vec<String> = vec![
            "-of".into(),
            "COG".into(),
            "-co".into(),
            "COMPRESS=DEFLATE".into(),
        ];

        args.push("-o".into());
        args.push(output.to_string_lossy().to_string());

        for input in inputs {
            args.push(input.as_ref().to_string_lossy().to_string());
        }

        Self::run_gdal("gdal_merge.py", &args).await?;

        tracing::info!(
            "Mosaic created: {} ({} inputs)",
            output.display(),
            inputs.len()
        );
        Ok(output)
    }

    /// Clip a raster to an extent geometry (e.g., GeoJSON or GPKG).
    pub async fn clip(
        input: impl AsRef<Path>,
        cutline: impl AsRef<Path>,
        output: impl AsRef<Path>,
        crop_to_cutline: bool,
    ) -> GeoResult<PathBuf> {
        let output = output.as_ref().to_path_buf();

        let mut args = vec![
            "-of".to_string(),
            "COG".to_string(),
            "-co".to_string(),
            "COMPRESS=DEFLATE".to_string(),
            "-cutline".to_string(),
            cutline.as_ref().to_string_lossy().to_string(),
            input.as_ref().to_string_lossy().to_string(),
            output.to_string_lossy().to_string(),
        ];

        if crop_to_cutline {
            args.insert(0, "-crop_to_cutline".to_string());
        }

        Self::run_gdal("gdalwarp", &args).await?;

        tracing::info!("Raster clipped: {}", output.display());
        Ok(output)
    }

    /// Get basic raster info (size, bands, CRS, extent).
    pub async fn info(input: impl AsRef<Path>) -> GeoResult<RasterInfo> {
        let input = input.as_ref();
        let output =
            Self::run_gdal_capture("gdalinfo", &["-json", &input.to_string_lossy()]).await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let info: serde_json::Value = serde_json::from_str(stdout.trim())
            .map_err(|e| GeoError::Other(format!("Failed to parse gdalinfo JSON: {e}")))?;

        Ok(RasterInfo {
            path: input.to_path_buf(),
            crs: info["coordinateSystem"]["wkt"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            width: info["size"][0].as_u64().unwrap_or(0) as usize,
            height: info["size"][1].as_u64().unwrap_or(0) as usize,
            bands: info["bands"].as_array().map(|b| b.len()).unwrap_or(0),
            pixel_size: (
                info["geoTransform"][1].as_f64().unwrap_or(1.0),
                info["geoTransform"][5]
                    .as_f64()
                    .map(|v| v.abs())
                    .unwrap_or(1.0),
            ),
            extent: [
                info["cornerCoordinates"]["lowerLeft"][0]
                    .as_f64()
                    .unwrap_or(0.0),
                info["cornerCoordinates"]["lowerLeft"][1]
                    .as_f64()
                    .unwrap_or(0.0),
                info["cornerCoordinates"]["upperRight"][0]
                    .as_f64()
                    .unwrap_or(0.0),
                info["cornerCoordinates"]["upperRight"][1]
                    .as_f64()
                    .unwrap_or(0.0),
            ],
        })
    }

    /// Run a GDAL command with its arguments, returning Ok if it succeeded.
    async fn run_gdal(tool: &str, args: &[String]) -> GeoResult<()> {
        let output = tokio::process::Command::new(tool)
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GeoError::ExternalProcess {
                command: format!("{tool} {}", args.join(" ")),
                message: stderr.trim().to_string(),
            });
        }

        Ok(())
    }

    /// Run a GDAL command and return the raw output.
    async fn run_gdal_capture(tool: &str, args: &[&str]) -> GeoResult<Output> {
        let output = tokio::process::Command::new(tool)
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GeoError::ExternalProcess {
                command: format!("{tool} {}", args.join(" ")),
                message: stderr.trim().to_string(),
            });
        }

        Ok(output)
    }
}

/// Metadata from `gdalinfo -json`.
#[derive(Debug, Clone)]
pub struct RasterInfo {
    /// Path to the raster file.
    pub path: PathBuf,
    /// CRS WKT string.
    pub crs: String,
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
    /// Number of bands.
    pub bands: usize,
    /// Pixel size as (x, y) where y is absolute value.
    pub pixel_size: (f64, f64),
    /// Extent as [min_x, min_y, max_x, max_y].
    pub extent: [f64; 4],
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: skip test if gdal_translate is not installed.
    #[allow(dead_code)]
    fn has_gdal_translate() -> bool {
        std::process::Command::new("gdal_translate")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[tokio::test]
    async fn test_missing_input() {
        let result = RasterOps::to_cog("/nonexistent/path.tif", "/tmp/out.tif", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cog_options_default() {
        let opts = CogOptions::default();
        assert_eq!(opts.compression, "DEFLATE");
        assert_eq!(opts.tile_size, 256);
        assert!(opts.overviews);
    }
}
