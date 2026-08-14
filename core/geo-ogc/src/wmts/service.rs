use super::renderers;
use super::request::{WmtsGetFeatureInfoParams, WmtsGetTileParams, WmtsRequest};
use super::response::WmtsResponse;
use super::tile_matrix::{TileMatrixSet, TileRendererFn, WmtsLayer};
use super::TileCache;
use crate::common::{OgcError, ServiceType};

/// WMTS service implementation.
pub struct WmtsService {
    /// Service title.
    pub title: String,
    /// Service endpoint URL.
    pub online_resource: String,
    /// Registered layers.
    pub layers: Vec<WmtsLayer>,
    /// Tile matrix sets.
    pub tile_matrix_sets: Vec<TileMatrixSet>,
    /// In-memory tile cache.
    pub cache: TileCache,
    /// Default tile renderer for layers without their own renderer.
    #[allow(clippy::type_complexity)]
    pub default_renderer: Option<TileRendererFn>,
}

impl WmtsService {
    /// Create a new WMTS service.
    pub fn new(title: impl Into<String>, online_resource: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            online_resource: online_resource.into(),
            layers: Vec::new(),
            tile_matrix_sets: Vec::new(),
            cache: TileCache::default(),
            default_renderer: None,
        }
    }

    /// Add a layer.
    pub fn add_layer(&mut self, layer: WmtsLayer) {
        self.layers.push(layer);
    }

    /// Add a tile matrix set.
    pub fn add_tile_matrix_set(&mut self, tms: TileMatrixSet) {
        self.tile_matrix_sets.push(tms);
    }

    /// Handle a WMTS request.
    pub fn handle(&self, request: &WmtsRequest) -> Result<WmtsResponse, OgcError> {
        match request {
            WmtsRequest::GetCapabilities => Ok(WmtsResponse::Xml(self.build_capabilities_xml())),
            WmtsRequest::GetTile(params) => self.handle_get_tile(params),
            WmtsRequest::GetFeatureInfo(params) => self.handle_get_feature_info(params),
        }
    }

    fn handle_get_tile(&self, params: &WmtsGetTileParams) -> Result<WmtsResponse, OgcError> {
        // Validate layer exists
        let layer = self
            .layers
            .iter()
            .find(|l| l.name == params.layer)
            .ok_or_else(|| {
                OgcError::new(
                    ServiceType::WMTS,
                    "1.0.0",
                    "LayerNotDefined",
                    format!("Layer '{}' not found", params.layer),
                )
            })?;

        // Validate tile matrix set exists
        if !self
            .tile_matrix_sets
            .iter()
            .any(|t| t.identifier == params.tile_matrix_set)
        {
            return Err(OgcError::new(
                ServiceType::WMTS,
                "1.0.0",
                "InvalidParameterValue",
                format!("TileMatrixSet '{}' not found", params.tile_matrix_set),
            ));
        }

        // Check cache first
        if let Some(data) = self.cache.get(
            &params.layer,
            &params.tile_matrix_set,
            &params.tile_matrix,
            params.tile_col,
            params.tile_row,
        ) {
            return Ok(WmtsResponse::Tile {
                data: data.to_vec(),
                mime_type: params.format.clone(),
            });
        }

        // Dispatch MVT format vs raster format
        let is_mvt = params
            .format
            .starts_with("application/vnd.mapbox-vector-tile")
            || params.format == "application/x-protobuf";

        if is_mvt {
            return self.handle_mvt_tile(layer, params);
        }

        // Generate raster tile using layer renderer, fallback to default, then checkerboard
        let tm: u32 = params.tile_matrix.parse().unwrap_or(0);
        let renderer = layer.renderer.as_ref().or(self.default_renderer.as_ref());
        let data = match renderer {
            Some(r) => r(tm, params.tile_col, params.tile_row),
            None => renderers::checkerboard(tm, params.tile_col, params.tile_row),
        };

        Ok(WmtsResponse::Tile {
            data,
            mime_type: params.format.clone(),
        })
    }

    /// Handle an MVT (vector tile) request.
    fn handle_mvt_tile(
        &self,
        layer: &WmtsLayer,
        params: &WmtsGetTileParams,
    ) -> Result<WmtsResponse, OgcError> {
        let provider = layer.mvt_source.as_ref().ok_or_else(|| {
            OgcError::new(
                ServiceType::WMTS,
                "1.0.0",
                "InvalidParameterValue",
                format!("Layer '{}' does not support MVT format", params.layer),
            )
        })?;

        let zoom: u8 = params.tile_matrix.parse().unwrap_or(0);
        let features = provider.features_for_tile(zoom, params.tile_col, params.tile_row);

        if features.is_empty() {
            // Return an empty MVT tile (valid protobuf)
            let layer = geo_tile::MvtLayer {
                name: params.layer.clone(),
                extent: 4096,
                features: vec![],
            };
            let encoder = geo_tile::MvtEncoder::new(4096);
            let data = encoder.encode(&[layer]).map_err(|e| {
                OgcError::new(
                    ServiceType::WMTS,
                    "1.0.0",
                    "InternalError",
                    format!("MVT encode error: {e}"),
                )
            })?;
            return Ok(WmtsResponse::Tile {
                data,
                mime_type: "application/vnd.mapbox-vector-tile".to_string(),
            });
        }

        let encoder = geo_tile::MvtEncoder::new(4096);
        let data = encoder
            .encode_tile(
                &params.layer,
                &features,
                params.tile_col,
                params.tile_row,
                zoom,
            )
            .map_err(|e| {
                OgcError::new(
                    ServiceType::WMTS,
                    "1.0.0",
                    "InternalError",
                    format!("MVT encode error: {e}"),
                )
            })?;

        Ok(WmtsResponse::Tile {
            data,
            mime_type: "application/vnd.mapbox-vector-tile".to_string(),
        })
    }

    fn handle_get_feature_info(
        &self,
        params: &WmtsGetFeatureInfoParams,
    ) -> Result<WmtsResponse, OgcError> {
        // Validate layer is queryable
        let layer = self
            .layers
            .iter()
            .find(|l| l.name == params.tile_params.layer);
        match layer {
            Some(_l) => {}
            None => {
                return Err(OgcError::new(
                    ServiceType::WMTS,
                    "1.0.0",
                    "LayerNotDefined",
                    format!("Layer '{}' not found", params.tile_params.layer),
                ));
            }
        }

        // Placeholder: query features at tile pixel
        let features = serde_json::json!({
            "type": "FeatureCollection",
            "features": [],
            "totalFeatures": 0
        });
        let json_str = serde_json::to_string_pretty(&features).unwrap_or_default();
        Ok(WmtsResponse::Xml(json_str))
    }
}
