//! Raster processing subcommands: NDVI, Slope, etc.
//!
//! These bypass the pipeline and PluginRegistry for direct file→file processing.

use geo_raster::RasterBand;

/// Compute NDVI from two single-band GeoTIFFs (red + NIR).
pub fn handle_ndvi(
    red_path: &str,
    nir_path: &str,
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Reading red band from {red_path}...");
    let red = read_tiff_f64(red_path)?;
    eprintln!("Reading NIR band from {nir_path}...");
    let nir = read_tiff_f64(nir_path)?;

    if red.rows != nir.rows || red.cols != nir.cols {
        return Err(format!(
            "Dimension mismatch: red {}×{} vs nir {}×{}",
            red.cols, red.rows, nir.cols, nir.rows
        )
        .into());
    }

    eprintln!("Computing NDVI ({}×{} px)...", red.cols, red.rows);
    let result = geo_facade::raster::compute_ndvi(&red, &nir)?;

    let out_path = std::path::Path::new(output_path);
    let tiff_path = if out_path.extension().is_none() {
        out_path.with_extension("tif")
    } else {
        out_path.to_path_buf()
    };

    let tiff_info = geo_raster::tiff_writer::GeoTiffInfo::new(1.0, 1.0, 0.0, 0.0, None);
    geo_raster::tiff_writer::write_geotiff(&result.ndvi, &tiff_path, &tiff_info)?;
    eprintln!("✅ NDVI written to {}", tiff_path.display());
    eprintln!("   Mean NDVI: {:.4}", result.mean_ndvi.unwrap_or(f64::NAN));
    eprintln!(
        "   Healthy pixels: {:.1}%",
        result.healthy_ratio.unwrap_or(0.0) * 100.0
    );
    eprintln!(
        "   Degraded pixels: {:.1}%",
        result.degraded_ratio.unwrap_or(0.0) * 100.0
    );
    Ok(())
}

/// Compute terrain slope from a DEM GeoTIFF.
pub fn handle_slope(
    dem_path: &str,
    output_path: &str,
    cell_size_m: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Reading DEM from {dem_path}...");
    let dem = read_tiff_f64(dem_path)?;

    eprintln!(
        "Computing slope ({}×{} px, {cell_size_m}m/cell)...",
        dem.cols, dem.rows
    );
    let result = geo_raster::compute_slope_degrees(
        &dem.data,
        dem.rows,
        dem.cols,
        cell_size_m,
        Some(dem.nodata),
    );

    let band = RasterBand::new(
        "slope_degrees",
        result.rows,
        result.cols,
        result.slope_degrees,
        -9999.0,
    );
    let tiff_path = std::path::Path::new(output_path).with_extension("tif");
    let tiff_info = geo_raster::tiff_writer::GeoTiffInfo::new(1.0, 1.0, 0.0, 0.0, None);
    geo_raster::tiff_writer::write_geotiff(&band, &tiff_path, &tiff_info)?;

    eprintln!("✅ Slope written to {}", tiff_path.display());
    if let Some(mean) = result.mean_degrees {
        eprintln!("   Mean slope: {:.2}°", mean);
    }
    if let Some(max) = result.max_degrees {
        eprintln!("   Max slope: {:.2}°", max);
    }
    Ok(())
}

/// Simple GeoTIFF reader returning a RasterBand (f64).
fn read_tiff_f64(path: &str) -> Result<RasterBand, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let mut decoder = tiff::decoder::Decoder::new(file)?;
    let (width, height) = decoder.dimensions()?;
    let img = decoder.read_image()?;

    let data = match img {
        tiff::decoder::DecodingResult::F32(pixels) => pixels.iter().map(|v| *v as f64).collect(),
        tiff::decoder::DecodingResult::F64(pixels) => pixels,
        tiff::decoder::DecodingResult::U8(pixels) => pixels.iter().map(|v| *v as f64).collect(),
        tiff::decoder::DecodingResult::U16(pixels) => pixels.iter().map(|v| *v as f64).collect(),
        tiff::decoder::DecodingResult::U32(pixels) => pixels.iter().map(|v| *v as f64).collect(),
        tiff::decoder::DecodingResult::I8(pixels) => pixels.iter().map(|v| *v as f64).collect(),
        tiff::decoder::DecodingResult::I16(pixels) => pixels.iter().map(|v| *v as f64).collect(),
        tiff::decoder::DecodingResult::I32(pixels) => pixels.iter().map(|v| *v as f64).collect(),
        _ => return Err("Unsupported TIFF data type (U64/I64)".into()),
    };

    Ok(RasterBand::new(
        "band",
        height as usize,
        width as usize,
        data,
        -9999.0,
    ))
}
