use serde::{Deserialize, Serialize};

/// WMTS request types per OGC WMTS 1.0.0 spec.
#[derive(Debug, Clone)]
pub enum WmtsRequest {
    /// Get service metadata and tile matrix sets.
    GetCapabilities,
    /// Return a single tile.
    GetTile(WmtsGetTileParams),
    /// Query feature info at a tile pixel.
    GetFeatureInfo(WmtsGetFeatureInfoParams),
}

/// Parameters for a GetTile request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WmtsGetTileParams {
    /// Layer name.
    pub layer: String,
    /// Tile matrix set identifier (e.g., "EPSG:4326", "EPSG:3857").
    pub tile_matrix_set: String,
    /// Tile matrix (zoom level).
    pub tile_matrix: String,
    /// Tile column (x).
    pub tile_col: u32,
    /// Tile row (y).
    pub tile_row: u32,
    /// Output format (e.g., "image/png", "application/vnd.mapbox-vector-tile").
    #[serde(default = "default_tile_format")]
    pub format: String,
}

fn default_tile_format() -> String {
    "image/png".into()
}

/// Parameters for a GetFeatureInfo request within a tile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WmtsGetFeatureInfoParams {
    /// Same as GetTile params.
    pub tile_params: WmtsGetTileParams,
    /// X pixel coordinate within the tile.
    pub i: u32,
    /// Y pixel coordinate within the tile.
    pub j: u32,
    /// Output format.
    #[serde(default = "default_feature_info_format")]
    pub info_format: String,
    /// Max feature count.
    #[serde(default = "default_feature_count")]
    pub feature_count: u32,
}

fn default_feature_info_format() -> String {
    "application/json".into()
}
fn default_feature_count() -> u32 {
    10
}
