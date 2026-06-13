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

/// 输出文件格式（对应 GDAL driver 简称）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputDriver {
    Cog,
    GeoTiff,
    Png,
    Jp2,
    NetCdf,
    Bmp,
}

impl OutputDriver {
    fn as_driver(&self) -> &'static str {
        match self {
            Self::Cog => "COG",
            Self::GeoTiff => "GTiff",
            Self::Png => "PNG",
            Self::Jp2 => "JP2OpenJPEG",
            Self::NetCdf => "netCDF",
            Self::Bmp => "BMP",
        }
    }
}

/// 重采样方法（对应 gdalwarp -r）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResamplingMethod {
    Nearest,
    Bilinear,
    Cubic,
    CubicSpline,
    Lanczos,
    Average,
    Mode,
    Max,
    Min,
    Med,
    Q1,
    Q3,
}

impl ResamplingMethod {
    fn as_arg(&self) -> &'static str {
        match self {
            Self::Nearest => "near",
            Self::Bilinear => "bilinear",
            Self::Cubic => "cubic",
            Self::CubicSpline => "cubicspline",
            Self::Lanczos => "lanczos",
            Self::Average => "average",
            Self::Mode => "mode",
            Self::Max => "max",
            Self::Min => "min",
            Self::Med => "med",
            Self::Q1 => "q1",
            Self::Q3 => "q3",
        }
    }
}

/// `gdalwarp` 完整选项。
#[derive(Debug, Clone)]
pub struct GdalWarpOptions {
    /// 输出格式驱动（默认 COG）。
    pub driver: OutputDriver,
    /// 目标 EPSG（如 `Some(4326)`）。None=不改变。
    pub target_epsg: Option<u16>,
    /// 分辨率 (x, y)，None 则保持原分辨率。
    pub resolution: Option<(f64, f64)>,
    /// 重采样方法（默认 Bilinear）。
    pub resampling: ResamplingMethod,
    /// 压缩算法（默认 DEFLATE）。
    pub compression: String,
    /// 压缩级别 (1–9)。
    pub compress_level: u8,
    /// 输出 NoData 值。
    pub dst_nodata: Option<f64>,
    /// 裁剪面（矢量路径）。
    pub cutline: Option<PathBuf>,
    /// 是否裁剪到裁剪面范围。
    pub crop_to_cutline: bool,
    /// 内存限制 (MB)，0=默认。
    pub warp_memory_mb: usize,
    /// 多线程 warp pass 数（0=ALL_CPUS）。
    pub multi: bool,
}

impl Default for GdalWarpOptions {
    fn default() -> Self {
        Self {
            driver: OutputDriver::Cog,
            target_epsg: None,
            resolution: None,
            resampling: ResamplingMethod::Bilinear,
            compression: "DEFLATE".into(),
            compress_level: 6,
            dst_nodata: None,
            cutline: None,
            crop_to_cutline: true,
            warp_memory_mb: 0,
            multi: true,
        }
    }
}

/// `gdal_translate` 完整选项。
#[derive(Debug, Clone)]
pub struct GdalTranslateOptions {
    /// 输出格式驱动（默认 COG）。
    pub driver: OutputDriver,
    /// 输出数据类型。None=保持输入类型。
    pub output_type: Option<DataType>,
    /// 波段选择（1-indexed）。None=输出全部波段。
    pub bands: Option<Vec<u16>>,
    /// 缩放参数 `(src_min, src_max, dst_min, dst_max)`。
    pub scale: Option<(f64, f64, f64, f64)>,
    /// 压缩算法（默认 DEFLATE）。
    pub compression: String,
    /// 压缩级别 (1–9)。
    pub compress_level: u8,
    /// 构建内部概览。
    pub overviews: bool,
    /// 块大小
    pub tile_size: u16,
}

impl Default for GdalTranslateOptions {
    fn default() -> Self {
        Self {
            driver: OutputDriver::Cog,
            output_type: None,
            bands: None,
            scale: None,
            compression: "DEFLATE".into(),
            compress_level: 6,
            overviews: true,
            tile_size: 256,
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

    /// 通用 `gdal_translate` 封装 — 支持格式转换、波段选择、缩放、数据类型。
    pub async fn gdal_translate(
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
        opts: GdalTranslateOptions,
    ) -> GeoResult<PathBuf> {
        let input = input.as_ref();
        let output = output.as_ref();

        if !input.exists() {
            return Err(GeoError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Input raster not found: {}", input.display()),
            )));
        }

        let mut args: Vec<String> = Vec::new();

        // 输出格式
        args.push("-of".into());
        args.push(opts.driver.as_driver().to_string());

        // 输出数据类型
        if let Some(dt) = &opts.output_type {
            args.push("-ot".into());
            args.push(format!("{:?}", dt));
        }

        // 波段选择
        if let Some(bands) = &opts.bands {
            for &b in bands {
                args.push("-b".into());
                args.push(b.to_string());
            }
        }

        // 缩放
        if let Some((smin, smax, dmin, dmax)) = opts.scale {
            args.push("-scale".into());
            args.extend_from_slice(&[
                smin.to_string(),
                smax.to_string(),
                dmin.to_string(),
                dmax.to_string(),
            ]);
        }

        // 压缩
        args.push("-co".into());
        args.push(format!("COMPRESS={}", opts.compression));
        args.push("-co".into());
        args.push(format!("LEVEL={}", opts.compress_level));

        // 概览
        if opts.overviews {
            args.push("-co".into());
            args.push("OVERVIEWS=AUTO".into());
        }

        args.push(input.to_string_lossy().to_string());
        args.push(output.to_string_lossy().to_string());

        Self::run_gdal("gdal_translate", &args).await?;

        tracing::info!("gdal_translate: {}", output.display());
        Ok(output.to_path_buf())
    }

    /// 通用 `gdalwarp` 封装 — 支持重投影、重采样、裁剪、NoData。
    pub async fn gdalwarp(
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
        opts: GdalWarpOptions,
    ) -> GeoResult<PathBuf> {
        let input = input.as_ref();
        let output = output.as_ref();

        if !input.exists() {
            return Err(GeoError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Input raster not found: {}", input.display()),
            )));
        }

        let mut args: Vec<String> = Vec::new();

        // 输出格式
        args.push("-of".into());
        args.push(opts.driver.as_driver().to_string());

        // 目标 CRS
        if let Some(epsg) = opts.target_epsg {
            args.push("-t_srs".into());
            args.push(format!("EPSG:{epsg}"));
        }

        // 分辨率
        if let Some((rx, ry)) = opts.resolution {
            args.push("-tr".into());
            args.push(rx.to_string());
            args.push(ry.to_string());
        }

        // 重采样
        args.push("-r".into());
        args.push(opts.resampling.as_arg().to_string());

        // 压缩
        args.push("-co".into());
        args.push(format!("COMPRESS={}", opts.compression));
        args.push("-co".into());
        args.push(format!("LEVEL={}", opts.compress_level));

        // NoData
        if let Some(nd) = opts.dst_nodata {
            args.push("-dstnodata".into());
            args.push(nd.to_string());
        }

        // 裁剪面
        if let Some(cut) = &opts.cutline {
            args.push("-cutline".into());
            args.push(cut.to_string_lossy().to_string());
            if opts.crop_to_cutline {
                args.push("-crop_to_cutline".into());
            }
        }

        // 内存限制
        if opts.warp_memory_mb > 0 {
            args.push("-wm".into());
            args.push(opts.warp_memory_mb.to_string());
        }

        // 多线程
        if opts.multi {
            args.push("-multi".into());
        }

        args.push(input.to_string_lossy().to_string());
        args.push(output.to_string_lossy().to_string());

        Self::run_gdal("gdalwarp", &args).await?;

        tracing::info!("gdalwarp: {}", output.display());
        Ok(output.to_path_buf())
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
