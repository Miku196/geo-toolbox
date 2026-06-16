//! WMTS (Web Map Tile Service) — OGC WMTS 1.0.0 implementation.
//!
//! Supports:
//! - `GetCapabilities` — service metadata + tile matrix sets + layer listing
//! - `GetTile` — return a single tile (z/x/y) as image bytes
//! - `GetFeatureInfo` — query feature attributes at a pixel within a tile

use crate::common::{OgcError, ServiceType, Wgs84Bbox};
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

fn default_formats() -> Vec<String> {
    vec!["image/png".into()]
}

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
}

impl WmtsService {
    /// Create a new WMTS service.
    pub fn new(title: impl Into<String>, online_resource: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            online_resource: online_resource.into(),
            layers: Vec::new(),
            tile_matrix_sets: Vec::new(),
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
        if !self.layers.iter().any(|l| l.name == params.layer) {
            return Err(OgcError::new(
                ServiceType::WMTS,
                "1.0.0",
                "LayerNotDefined",
                format!("Layer '{}' not found", params.layer),
            ));
        }

        // Validate tile matrix set
        if !self.tile_matrix_sets.iter().any(|t| t.identifier == params.tile_matrix_set) {
            return Err(OgcError::new(
                ServiceType::WMTS,
                "1.0.0",
                "InvalidParameterValue",
                format!("TileMatrixSet '{}' not found", params.tile_matrix_set),
            ));
        }

        // Placeholder: return empty tile data
        // In production: query spatial data → rasterize → encode tile
        Ok(WmtsResponse::Tile {
            data: vec![0u8; 256 * 256 * 4], // RGBA placeholder
            mime_type: params.format.clone(),
        })
    }

    fn handle_get_feature_info(
        &self,
        params: &WmtsGetFeatureInfoParams,
    ) -> Result<WmtsResponse, OgcError> {
        // Validate layer is queryable
        let layer = self.layers.iter().find(|l| l.name == params.tile_params.layer);
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

    /// Build WMTS 1.0.0 GetCapabilities XML document.
    pub fn build_capabilities_xml(&self) -> String {
        let layers_xml: String = self
            .layers
            .iter()
            .map(|l| self.layer_to_xml(l))
            .collect::<Vec<_>>()
            .join("\n");

        let tms_xml: String = self
            .tile_matrix_sets
            .iter()
            .map(|t| self.tile_matrix_set_to_xml(t))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Capabilities version="1.0.0"
              xmlns="http://www.opengis.net/wmts/1.0"
              xmlns:ows="http://www.opengis.net/ows/1.1"
              xmlns:xlink="http://www.w3.org/1999/xlink"
              xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <ows:ServiceIdentification>
    <ows:Title>{title}</ows:Title>
    <ows:ServiceType>OGC WMTS</ows:ServiceType>
    <ows:ServiceTypeVersion>1.0.0</ows:ServiceTypeVersion>
  </ows:ServiceIdentification>
  <ows:ServiceProvider>
    <ows:ProviderName>geo-toolbox</ows:ProviderName>
  </ows:ServiceProvider>
  <ows:OperationsMetadata>
    <ows:Operation name="GetCapabilities">
      <ows:DCP>
        <ows:HTTP>
          <ows:Get xlink:href="{url}">
            <ows:Constraint name="GetEncoding">
              <ows:AllowedValues>
                <ows:Value>KVP</ows:Value>
              </ows:AllowedValues>
            </ows:Constraint>
          </ows:Get>
        </ows:HTTP>
      </ows:DCP>
    </ows:Operation>
    <ows:Operation name="GetTile">
      <ows:DCP>
        <ows:HTTP>
          <ows:Get xlink:href="{url}">
            <ows:Constraint name="GetEncoding">
              <ows:AllowedValues>
                <ows:Value>KVP</ows:Value>
              </ows:AllowedValues>
            </ows:Constraint>
          </ows:Get>
        </ows:HTTP>
      </ows:DCP>
    </ows:Operation>
  </ows:OperationsMetadata>
  <Contents>
{layers_xml}
{tms_xml}
  </Contents>
  <ServiceMetadataURL xlink:href="{url}"/>
</Capabilities>"#,
            title = self.title,
            url = self.online_resource,
            layers_xml = layers_xml,
            tms_xml = tms_xml,
        )
    }

    fn layer_to_xml(&self, layer: &WmtsLayer) -> String {
        let bbox_xml = if let Some(bbox) = &layer.wgs84_bbox {
            format!(
                r#"      <ows:WGS84BoundingBox>
        <ows:LowerCorner>{west} {south}</ows:LowerCorner>
        <ows:UpperCorner>{east} {north}</ows:UpperCorner>
      </ows:WGS84BoundingBox>"#,
                west = bbox.west,
                south = bbox.south,
                east = bbox.east,
                north = bbox.north,
            )
        } else {
            String::new()
        };

        let formats_xml: String = layer
            .formats
            .iter()
            .map(|f| format!("      <Format>{f}</Format>"))
            .collect::<Vec<_>>()
            .join("\n");

        let styles_xml: String = if layer.styles.is_empty() {
            r#"      <Style isDefault="true">
        <ows:Title>Default</ows:Title>
        <ows:Identifier>default</ows:Identifier>
      </Style>"#.into()
        } else {
            layer
                .styles
                .iter()
                .map(|s| format!(
                    r#"      <Style isDefault="false">
        <ows:Title>{s}</ows:Title>
        <ows:Identifier>{s}</ows:Identifier>
      </Style>"#
                ))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let tms_refs: String = layer
            .tile_matrix_sets
            .iter()
            .map(|t| format!("      <TileMatrixSetLink>\n        <TileMatrixSet>{t}</TileMatrixSet>\n      </TileMatrixSetLink>"))
            .collect::<Vec<_>>()
            .join("\n");

        let resource_url = if let Some(url) = &layer.resource_url {
            format!(
                r#"    <ResourceURL format="{fmt}" resourceType="tile" template="{url}"/>"#,
                fmt = layer.formats.first().map(|s| s.as_str()).unwrap_or("image/png"),
                url = url,
            )
        } else {
            String::new()
        };

        format!(
            r#"    <Layer>
      <ows:Title>{title}</ows:Title>
      <ows:Identifier>{name}</ows:Identifier>
{abstract_xml}
{bbox_xml}
      <ows:CRS>{crs}</ows:CRS>
{tms_refs}
{formats_xml}
{styles_xml}
{resource_url}
    </Layer>"#,
            title = layer.title,
            name = layer.name,
            abstract_xml = layer
                .abstract_
                .as_ref()
                .map(|a| format!("      <ows:Abstract>{a}</ows:Abstract>"))
                .unwrap_or_default(),
            bbox_xml = bbox_xml,
            crs = layer.crs.first().map(|s| s.as_str()).unwrap_or("EPSG:4326"),
            tms_refs = tms_refs,
            formats_xml = formats_xml,
            styles_xml = styles_xml,
            resource_url = resource_url,
        )
    }

    fn tile_matrix_set_to_xml(&self, tms: &TileMatrixSet) -> String {
        let matrices_xml: String = tms
            .tile_matrices
            .iter()
            .map(|tm| {
                format!(
                    r#"      <TileMatrix>
        <ows:Identifier>{id}</ows:Identifier>
        <ScaleDenominator>{scale}</ScaleDenominator>
        <TopLeftCorner>{tlx} {tly}</TopLeftCorner>
        <TileWidth>{tw}</TileWidth>
        <TileHeight>{th}</TileHeight>
        <MatrixWidth>{mw}</MatrixWidth>
        <MatrixHeight>{mh}</MatrixHeight>
      </TileMatrix>"#,
                    id = tm.identifier,
                    scale = tm.scale_denominator,
                    tlx = tm.top_left_x,
                    tly = tm.top_left_y,
                    tw = tm.tile_width,
                    th = tm.tile_height,
                    mw = tm.matrix_width,
                    mh = tm.matrix_height,
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"    <TileMatrixSet>
      <ows:Identifier>{id}</ows:Identifier>
      <ows:CRS>{crs}</ows:CRS>
      <ows:BoundingBox CRS="{crs}">
        <ows:LowerCorner>{west} {south}</ows:LowerCorner>
        <ows:UpperCorner>{east} {north}</ows:UpperCorner>
      </ows:BoundingBox>
{matrices_xml}
    </TileMatrixSet>"#,
            id = tms.identifier,
            crs = tms.supported_crs,
            west = tms.bounding_box.west,
            south = tms.bounding_box.south,
            east = tms.bounding_box.east,
            north = tms.bounding_box.north,
            matrices_xml = matrices_xml,
        )
    }
}

/// WMTS response variants.
#[derive(Debug, Clone)]
pub enum WmtsResponse {
    /// XML response (GetCapabilities).
    Xml(String),
    /// Tile binary data (GetTile).
    Tile {
        /// Tile bytes.
        data: Vec<u8>,
        /// MIME type.
        mime_type: String,
    },
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_service() -> WmtsService {
        let mut svc = WmtsService::new("Test WMTS", "https://example.com/wmts");
        svc.add_layer(WmtsLayer {
            name: "sentinel-2".into(),
            title: "Sentinel-2 NDVI".into(),
            abstract_: Some("Sentinel-2 satellite NDVI imagery".into()),
            keywords: vec!["sentinel".into(), "ndvi".into()],
            wgs84_bbox: Some(Wgs84Bbox::new(-180.0, -90.0, 180.0, 90.0)),
            crs: vec!["EPSG:4326".into(), "EPSG:3857".into()],
            tile_matrix_sets: vec!["EPSG:4326".into(), "EPSG:3857".into()],
            formats: vec!["image/png".into()],
            styles: vec!["default".into()],
            resource_url: Some("https://example.com/tiles/{TileMatrixSet}/{TileMatrix}/{TileCol}/{TileRow}.png".into()),
        });
        svc.add_tile_matrix_set(global_geodetic_tile_matrix_set());
        svc.add_tile_matrix_set(global_mercator_tile_matrix_set());
        svc
    }

    #[test]
    fn test_get_capabilities_xml() {
        let svc = make_service();
        let xml = svc.build_capabilities_xml();
        assert!(xml.contains("WMTS"));
        assert!(xml.contains("sentinel-2"));
        assert!(xml.contains("EPSG:4326"));
        assert!(xml.contains("EPSG:3857"));
        assert!(xml.contains("TileMatrixSet"));
    }

    #[test]
    fn test_get_tile_valid() {
        let svc = make_service();
        let params = WmtsGetTileParams {
            layer: "sentinel-2".into(),
            tile_matrix_set: "EPSG:4326".into(),
            tile_matrix: "5".into(),
            tile_col: 16,
            tile_row: 8,
            format: "image/png".into(),
        };
        let result = svc.handle(&WmtsRequest::GetTile(params));
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_tile_unknown_layer() {
        let svc = make_service();
        let params = WmtsGetTileParams {
            layer: "nonexistent".into(),
            tile_matrix_set: "EPSG:4326".into(),
            tile_matrix: "5".into(),
            tile_col: 0,
            tile_row: 0,
            format: "image/png".into(),
        };
        let result = svc.handle(&WmtsRequest::GetTile(params));
        assert!(result.is_err());
    }

    #[test]
    fn test_global_geodetic_tms() {
        let tms = global_geodetic_tile_matrix_set();
        assert_eq!(tms.identifier, "EPSG:4326");
        assert_eq!(tms.tile_matrices.len(), 22);
        assert_eq!(tms.tile_matrices[0].matrix_width, 2);
        assert_eq!(tms.tile_matrices[0].matrix_height, 1);
        assert_eq!(tms.tile_matrices[21].matrix_width, 2u32.pow(21) * 2);
    }

    #[test]
    fn test_global_mercator_tms() {
        let tms = global_mercator_tile_matrix_set();
        assert_eq!(tms.identifier, "EPSG:3857");
        assert_eq!(tms.tile_matrices.len(), 22);
        assert_eq!(tms.tile_matrices[0].matrix_width, 1);
        assert_eq!(tms.tile_matrices[0].matrix_height, 1);
    }
}
