use super::WmtsService;
use crate::common::{OgcError, ServiceType};

// ── PMTiles archive building ──

impl WmtsService {
    /// Build a PMTiles archive for a named layer across all zoom levels 0-10.
    ///
    /// Uses the layer's `mvt_source` to generate vector tiles at each tile coordinate
    /// for zoom levels 0 through 10. The archive is written to the provided writer.
    ///
    /// # Errors
    /// Returns an error if the layer doesn't exist or doesn't have an MVT source.
    pub fn build_pmtiles_archive<W: std::io::Write + std::io::Seek>(
        &self,
        layer_name: &str,
        writer: W,
    ) -> Result<geo_tile::PmtilesWriter<W>, OgcError> {
        let layer = self
            .layers
            .iter()
            .find(|l| l.name == layer_name)
            .ok_or_else(|| {
                OgcError::new(
                    ServiceType::WMTS,
                    "1.0.0",
                    "LayerNotDefined",
                    format!("Layer '{}' not found", layer_name),
                )
            })?;

        let provider = layer.mvt_source.as_ref().ok_or_else(|| {
            OgcError::new(
                ServiceType::WMTS,
                "1.0.0",
                "InvalidParameterValue",
                format!(
                    "Layer '{}' does not have an MVT source. PMTiles requires an MVT source.",
                    layer_name
                ),
            )
        })?;

        let mut pm_writer = geo_tile::PmtilesWriter::new(
            writer,
            geo_tile::TileType::Mvt,
            geo_tile::Compression::None,
        );

        for z in 0..=10u8 {
            let n = 2u32.pow(z as u32);
            for x in 0..n {
                for y in 0..n {
                    let features = provider.features_for_tile(z, x, y);
                    if !features.is_empty() {
                        let encoder = geo_tile::MvtEncoder::new(4096);
                        let tile_data = encoder
                            .encode_tile(layer_name, &features, x, y, z)
                            .map_err(|e| {
                                OgcError::new(
                                    ServiceType::WMTS,
                                    "1.0.0",
                                    "InternalError",
                                    format!("MVT encode error at ({z},{x},{y}): {e}"),
                                )
                            })?;
                        pm_writer.add_tile(z, x, y, tile_data);
                    }
                }
            }
        }

        Ok(pm_writer)
    }

    /// Count MVT tiles that would be generated for a layer across all zoom levels 0-10.
    /// Useful for estimating PMTiles archive size.
    pub fn estimate_mvt_tile_count(&self, layer_name: &str) -> Option<usize> {
        let layer = self.layers.iter().find(|l| l.name == layer_name)?;
        let provider = layer.mvt_source.as_ref()?;

        let mut count = 0;
        for z in 0..=10u8 {
            let n = 2u32.pow(z as u32);
            for x in 0..n {
                for y in 0..n {
                    let features = provider.features_for_tile(z, x, y);
                    if !features.is_empty() {
                        count += 1;
                    }
                }
            }
        }
        Some(count)
    }
}
