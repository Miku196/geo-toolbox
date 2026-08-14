use crate::common::Wgs84Bbox;
use crate::mvt_source::MvtFeatureProvider;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A tile matrix set (e.g., EPSG:4326 grid, EPSG:3857 Web Mercator).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileMatrixSet {
    /// Identifier (e.g., "EPSG:4326", "EPSG:3857").
    pub identifier: String,
    /// Bounding box in the CRS.
    pub bounding_box: Wgs84Bbox,
    /// Supported CRS.
    pub supported_crs: String,
    /// Tile matrix definitions per zoom level.
    pub tile_matrices: Vec<TileMatrix>,
}

/// A single zoom level within a tile matrix set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileMatrix {
    /// Zoom-level identifier (e.g., "0", "1", …).
    pub identifier: String,
    /// Scale denominator.
    pub scale_denominator: f64,
    /// Top-left corner X.
    pub top_left_x: f64,
    /// Top-left corner Y.
    pub top_left_y: f64,
    /// Tile width in pixels.
    pub tile_width: u32,
    /// Tile height in pixels.
    pub tile_height: u32,
    /// Matrix width in tiles.
    pub matrix_width: u32,
    /// Matrix height in tiles.
    pub matrix_height: u32,
}

/// A WMTS layer definition.
#[derive(Clone, Serialize, Deserialize)]
pub struct WmtsLayer {
    /// Unique layer name.
    pub name: String,
    /// Human-readable title.
    pub title: String,
    /// Optional abstract.
    pub abstract_: Option<String>,
    /// Keywords.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// WGS84 bounding box.
    pub wgs84_bbox: Option<Wgs84Bbox>,
    /// Supported CRS list.
    #[serde(default)]
    pub crs: Vec<String>,
    /// Tile matrix set(s) this layer uses.
    pub tile_matrix_sets: Vec<String>,
    /// Output formats supported (e.g. "image/png", "application/vnd.mapbox-vector-tile").
    #[serde(default = "default_formats")]
    pub formats: Vec<String>,
    /// Style identifiers.
    #[serde(default)]
    pub styles: Vec<String>,
    /// Resource URL template. Use {TileMatrixSet}/{TileMatrix}/{TileCol}/{TileRow}.{format}
    pub resource_url: Option<String>,
    /// Optional tile renderer for real-time tile generation (raster/PNG).
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub renderer: Option<TileRendererFn>,
    /// Optional MVT feature provider for vector tile generation.
    /// When set, the layer can serve `application/vnd.mapbox-vector-tile`.
    #[serde(skip)]
    pub mvt_source: Option<Arc<dyn MvtFeatureProvider>>,
}

fn default_formats() -> Vec<String> {
    vec!["image/png".into()]
}

/// A function that generates tile image data for a given z/x/y.
/// Returns RGBA pixel data (256x256x4 bytes).
pub type TileRendererFn = fn(u32, u32, u32) -> Vec<u8>;

/// Helper: build the global-geographic (EPSG:4326) tile matrix set.
pub fn global_geodetic_tile_matrix_set() -> TileMatrixSet {
    // OGC WMTS 1.0 Annex E.2: Global Geodetic Tile Matrix Set (EPSG:4326)
    let mut matrices = Vec::new();
    for zoom in 0..22 {
        let n = 2u32.pow(zoom);
        matrices.push(TileMatrix {
            identifier: zoom.to_string(),
            scale_denominator: 2.0_f64.powi(18 - zoom as i32) / n as f64,
            top_left_x: -180.0,
            top_left_y: 90.0,
            tile_width: 256,
            tile_height: 256,
            matrix_width: n * 2,
            matrix_height: n,
        });
    }
    TileMatrixSet {
        identifier: "EPSG:4326".into(),
        bounding_box: Wgs84Bbox::new(-180.0, -90.0, 180.0, 90.0),
        supported_crs: "EPSG:4326".into(),
        tile_matrices: matrices,
    }
}

/// Helper: build the Web Mercator (EPSG:3857) tile matrix set.
pub fn global_mercator_tile_matrix_set() -> TileMatrixSet {
    // Standard Google/Bing/OSM scheme
    let mut matrices = Vec::new();
    for zoom in 0..22 {
        let n = 2u32.pow(zoom);
        matrices.push(TileMatrix {
            identifier: zoom.to_string(),
            scale_denominator: 559_082_264.028 / (n as f64 * 256.0),
            top_left_x: -20_037_508.34,
            top_left_y: 20_037_508.34,
            tile_width: 256,
            tile_height: 256,
            matrix_width: n,
            matrix_height: n,
        });
    }
    TileMatrixSet {
        identifier: "EPSG:3857".into(),
        bounding_box: Wgs84Bbox::new(-180.0, -85.06, 180.0, 85.06),
        supported_crs: "EPSG:3857".into(),
        tile_matrices: matrices,
    }
}
