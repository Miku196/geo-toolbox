//! GeoTIFF 写入器。
//!
//! 将 `RasterBand` 写出为 GeoTIFF（Float32） + 附带的 `.tfw` 世界文件。
//! 支持单波段和 RGB 三波段写入。

use crate::grid::RasterBand;
use geo_core::errors::{GeoError, GeoResult};
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// GeoTIFF 地理参考信息。
#[derive(Debug, Clone)]
pub struct GeoTiffInfo {
    /// 像素宽度（经度/投影 x 方向）。
    pub pixel_width: f64,
    /// 像素高度（纬度/投影 y 方向，负值表示北在上）。
    pub pixel_height: f64,
    /// 左上角经度/x。
    pub x_ul: f64,
    /// 左上角纬度/y。
    pub y_ul: f64,
    /// CRS/投影（如 "EPSG:4326"），用于 .prj 文件。
    pub crs: Option<String>,
}

impl GeoTiffInfo {
    /// 创建 GeoTIFF 地理参考信息。
    ///
    /// * `pixel_width` — 像素宽度（度或米）。
    /// * `pixel_height` — 像素高度（负值表示北在上）。
    /// * `x_ul` — 左上角经度/x。
    /// * `y_ul` — 左上角纬度/y。
    /// * `crs` — 可选的 CRS 标识符。
    pub fn new(
        pixel_width: f64,
        pixel_height: f64,
        x_ul: f64,
        y_ul: f64,
        crs: Option<&str>,
    ) -> Self {
        Self {
            pixel_width,
            pixel_height: -pixel_height.abs(),
            x_ul,
            y_ul,
            crs: crs.map(|s| s.to_string()),
        }
    }

    /// 从 WGS84 bbox + 行列数计算 GeoTIFF 信息。
    pub fn from_bbox(
        west: f64,
        south: f64,
        east: f64,
        north: f64,
        rows: usize,
        cols: usize,
        crs: Option<&str>,
    ) -> Self {
        let pixel_width = (east - west) / cols as f64;
        let pixel_height = (north - south) / rows as f64;
        Self {
            pixel_width,
            pixel_height: -pixel_height.abs(),
            x_ul: west,
            y_ul: north,
            crs: crs.map(|s| s.to_string()),
        }
    }
}

/// 将单波段 `RasterBand` 写为 Float32 GeoTIFF + .tfw 世界文件。
pub fn write_geotiff(band: &RasterBand, path: &Path, info: &GeoTiffInfo) -> GeoResult<()> {
    let mut file = File::create(path).map_err(|e| {
        GeoError::Other(format!("Failed to create TIFF file '{}': {e}", path.display()))
    })?;

    let float_data: Vec<f32> = band.data.iter().map(|&v| v as f32).collect();

    let mut encoder = tiff::encoder::TiffEncoder::new(&mut file).map_err(|e| {
        GeoError::Other(format!("Failed to create TIFF encoder: {e}"))
    })?;

    encoder
        .write_image::<tiff::encoder::colortype::Gray32Float>(
            band.cols as u32,
            band.rows as u32,
            &float_data,
        )
        .map_err(|e| {
            GeoError::Other(format!("Failed to write TIFF image: {e}"))
        })?;

    write_tfw(path, info)?;
    Ok(())
}

/// 将三个波段写为 24-bit RGB GeoTIFF。
pub fn write_geotiff_rgb(
    red: &RasterBand,
    green: &RasterBand,
    blue: &RasterBand,
    path: &Path,
    info: &GeoTiffInfo,
) -> GeoResult<()> {
    if red.rows != green.rows || red.rows != blue.rows ||
       red.cols != green.cols || red.cols != blue.cols {
        return Err(GeoError::Validation(format!(
            "RGB band size mismatch: R {}x{} G {}x{} B {}x{}",
            red.rows, red.cols, green.rows, green.cols, blue.rows, blue.cols
        )));
    }

    fn stretch(band: &RasterBand) -> Vec<u8> {
        let min_val = band.min().unwrap_or(0.0);
        let max_val = band.max().unwrap_or(1.0);
        let range = (max_val - min_val).max(1e-6);
        band.data.iter()
            .map(|&v| {
                if v != band.nodata && !v.is_nan() {
                    ((v - min_val) / range * 255.0).clamp(0.0, 255.0) as u8
                } else { 0 }
            })
            .collect()
    }

    let r_data = stretch(red);
    let g_data = stretch(green);
    let b_data = stretch(blue);
    let n = r_data.len();
    let mut rgb_data: Vec<u8> = Vec::with_capacity(n * 3);
    for i in 0..n {
        rgb_data.push(r_data[i]);
        rgb_data.push(g_data[i]);
        rgb_data.push(b_data[i]);
    }

    let mut file = File::create(path).map_err(|e| {
        GeoError::Other(format!("Failed to create RGB TIFF file '{}': {e}", path.display()))
    })?;

    let mut encoder = tiff::encoder::TiffEncoder::new(&mut file).map_err(|e| {
        GeoError::Other(format!("Failed to create TIFF encoder: {e}"))
    })?;

    encoder
        .write_image::<tiff::encoder::colortype::RGB8>(
            red.cols as u32, red.rows as u32, &rgb_data,
        )
        .map_err(|e| {
            GeoError::Other(format!("Failed to write RGB TIFF image: {e}"))
        })?;

    write_tfw(path, info)?;
    Ok(())
}

fn write_tfw(image_path: &Path, info: &GeoTiffInfo) -> GeoResult<()> {
    let tfw_path = image_path.with_extension("tfw");
    let mut file = File::create(&tfw_path).map_err(|e| {
        GeoError::Other(format!("Failed to create TFW file '{}': {e}", tfw_path.display()))
    })?;

    writeln!(file, "{:.15}", info.pixel_width)?;
    writeln!(file, "0")?;
    writeln!(file, "0")?;
    writeln!(file, "{:.15}", info.pixel_height)?;
    writeln!(file, "{:.15}", info.x_ul)?;
    writeln!(file, "{:.15}", info.y_ul)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_band(name: &str, rows: usize, cols: usize, value: f64) -> RasterBand {
        RasterBand::new(name, rows, cols, vec![value; rows * cols], -999.0)
    }

    #[test]
    fn test_write_geotiff_single_band() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.tiff");
        let band = make_band("test", 10, 20, 1.0);
        let info = GeoTiffInfo::new(0.01, 0.01, 100.0, 30.0, Some("EPSG:4326"));

        let result = write_geotiff(&band, &path, &info);
        assert!(result.is_ok(), "Failed: {:?}", result);
        assert!(path.exists());
        assert!(path.with_extension("tfw").exists());
    }

    #[test]
    fn test_write_geotiff_rgb() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rgb.tiff");
        let info = GeoTiffInfo::new(0.001, 0.001, 120.0, 40.0, Some("EPSG:4326"));

        let result = write_geotiff_rgb(
            &make_band("r", 5, 10, 0.1),
            &make_band("g", 5, 10, 0.5),
            &make_band("b", 5, 10, 0.9),
            &path, &info,
        );
        assert!(result.is_ok());
        assert!(path.exists());
    }

    #[test]
    fn test_geotiff_info_from_bbox() {
        let info = GeoTiffInfo::from_bbox(100.0, 20.0, 110.0, 30.0, 100, 200, Some("EPSG:4326"));
        assert!((info.pixel_width - 0.05).abs() < 1e-10);
        assert!((info.pixel_height + 0.1).abs() < 1e-10);
        assert!((info.x_ul - 100.0).abs() < 1e-10);
        assert!((info.y_ul - 30.0).abs() < 1e-10);
    }

    #[test]
    fn test_tfw_content_format() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("fmt.tiff");
        write_geotiff(&make_band("t", 3, 4, 0.0), &path,
            &GeoTiffInfo::new(0.5, 0.5, 100.0, 50.0, None)).unwrap();

        let content = std::fs::read_to_string(path.with_extension("tfw")).unwrap();
        let lines: Vec<&str> = content.trim().lines().collect();
        assert_eq!(lines[0], "0.500000000000000");
        assert_eq!(lines[4], "100.000000000000000");
        assert_eq!(lines[5], "50.000000000000000");
    }

    #[test]
    fn test_rgb_size_mismatch() {
        let dir = tempdir().unwrap();
        let info = GeoTiffInfo::new(1.0, 1.0, 0.0, 0.0, None);
        let result = write_geotiff_rgb(
            &make_band("r", 2, 2, 0.0),
            &make_band("g", 3, 3, 0.0),
            &make_band("b", 2, 2, 0.0),
            &dir.path().join("err.tiff"), &info,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_write_geotiff_nodata() {
        let dir = tempdir().unwrap();
        let band = RasterBand::new("n", 2, 2, vec![-999.0, 1.0, 2.0, -999.0], -999.0);
        let info = GeoTiffInfo::new(1.0, 1.0, 0.0, 2.0, None);
        assert!(write_geotiff(&band, &dir.path().join("n.tiff"), &info).is_ok());
    }

    #[test]
    fn test_write_geotiff_invalid_path() {
        let band = make_band("err", 1, 1, 1.0);
        let info = GeoTiffInfo::new(1.0, 1.0, 0.0, 0.0, None);
        let result = write_geotiff(&band, Path::new("/nonexistent/dir/t.tiff"), &info);
        assert!(result.is_err());
    }

    #[test]
    fn test_geotiff_info_negative_height() {
        let info = GeoTiffInfo::new(0.01, -0.01, 100.0, 30.0, None);
        assert!(info.pixel_height < 0.0);
        assert!((info.pixel_height + 0.01).abs() < 1e-10);
    }
}
