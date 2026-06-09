//! WMS (Web Map Service) — OGC WMS 1.3.0 implementation.
//!
//! Supports:
//! - `GetCapabilities` — service metadata + layer listing
//! - `GetMap` — render a map image (PNG/JPEG/GeoTIFF)
//! - `GetFeatureInfo` — query feature attributes at a pixel location

use serde::{Serialize, Deserialize};
use crate::common::{OgcError, ServiceType, Wgs84Bbox};

/// WMS request types per OGC WMS 1.3.0 spec.
#[derive(Debug, Clone)]
pub enum WmsRequest {
    /// Get service metadata and available layers.
    GetCapabilities,
    /// Render a map image.
    GetMap(GetMapParams),
    /// Query feature info at a pixel.
    GetFeatureInfo(GetFeatureInfoParams),
}

/// Parameters for a GetMap request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetMapParams {
    /// Comma-separated layer names to render.
    pub layers: Vec<String>,
    /// Rendering style per layer (empty = default).
    #[serde(default)]
    pub styles: Vec<String>,
    /// CRS identifier (e.g., "EPSG:4326").
    pub crs: String,
    /// Bounding box in the specified CRS.
    pub bbox: Wgs84Bbox,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Output format (e.g., "image/png", "image/jpeg").
    #[serde(default = "default_format")]
    pub format: String,
    /// Background transparency.
    #[serde(default)]
    pub transparent: bool,
    /// Background color (hex).
    #[serde(default)]
    pub bgcolor: Option<String>,
}

fn default_format() -> String { "image/png".into() }

/// Parameters for a GetFeatureInfo request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetFeatureInfoParams {
    /// Same as GetMap params.
    #[serde(flatten)]
    pub map_params: GetMapParams,
    /// X pixel coordinate.
    pub i: u32,
    /// Y pixel coordinate.
    pub j: u32,
    /// Comma-separated query layer names.
    pub query_layers: Vec<String>,
    /// Output format (e.g., "application/json", "text/html").
    #[serde(default = "default_info_format")]
    pub info_format: String,
    /// Max feature count to return.
    #[serde(default = "default_feature_count")]
    pub feature_count: u32,
}

fn default_info_format() -> String { "application/json".into() }
fn default_feature_count() -> u32 { 1 }

/// A WMS layer definition (exposed in GetCapabilities).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WmsLayer {
    /// Unique layer name.
    pub name: String,
    /// Human-readable title.
    pub title: String,
    /// Optional abstract/description.
    pub abstract_: Option<String>,
    /// Keywords for catalog search.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// WGS84 bounding box.
    pub wgs84_bbox: Option<Wgs84Bbox>,
    /// Supported CRS (at minimum "EPSG:4326").
    #[serde(default = "default_crs_list")]
    pub crs: Vec<String>,
    /// Whether this layer is queryable (GetFeatureInfo).
    #[serde(default)]
    pub queryable: bool,
    /// Minimum scale denominator.
    #[serde(default)]
    pub min_scale: Option<f64>,
    /// Maximum scale denominator.
    #[serde(default)]
    pub max_scale: Option<f64>,
    /// Child layers (for nested layer trees).
    #[serde(default)]
    pub children: Vec<WmsLayer>,
}

fn default_crs_list() -> Vec<String> {
    vec!["EPSG:4326".into(), "EPSG:3857".into()]
}

/// WMS service implementation.
pub struct WmsService {
    /// Service title for GetCapabilities.
    pub title: String,
    /// Service endpoint URL.
    pub online_resource: String,
    /// Maximum image width (pixels).
    pub max_width: u32,
    /// Maximum image height (pixels).
    pub max_height: u32,
    /// Registered layers.
    pub layers: Vec<WmsLayer>,
}

impl WmsService {
    /// Create a new WMS service.
    pub fn new(title: impl Into<String>, online_resource: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            online_resource: online_resource.into(),
            max_width: 4096,
            max_height: 4096,
            layers: Vec::new(),
        }
    }

    /// Add a layer to the service.
    pub fn add_layer(&mut self, layer: WmsLayer) {
        self.layers.push(layer);
    }

    /// Handle a WMS request.
    pub fn handle(&self, request: &WmsRequest) -> Result<WmsResponse, OgcError> {
        match request {
            WmsRequest::GetCapabilities => Ok(WmsResponse::Xml(self.build_capabilities_xml())),
            WmsRequest::GetMap(params) => self.handle_get_map(params),
            WmsRequest::GetFeatureInfo(params) => self.handle_get_feature_info(params),
        }
    }

    fn handle_get_map(&self, params: &GetMapParams) -> Result<WmsResponse, OgcError> {
        // Validate dimensions
        if params.width > self.max_width || params.height > self.max_height {
            return Err(OgcError::new(
                ServiceType::WMS, "1.3.0",
                "InvalidParameterValue",
                format!("Image dimensions {}x{} exceed max {}x{}",
                    params.width, params.height, self.max_width, self.max_height),
            ));
        }

        // Validate layers exist
        for layer_name in &params.layers {
            if !self.layers.iter().any(|l| l.name == *layer_name) {
                return Err(OgcError::new(
                    ServiceType::WMS, "1.3.0",
                    "LayerNotDefined",
                    format!("Layer '{}' not found", layer_name),
                ));
            }
        }

        // Placeholder: render map image
        // In production, this would:
        // 1. Query spatial data for the requested layers + bbox
        // 2. Rasterize features to an image (using GDAL or software renderer)
        // 3. Encode as PNG/JPEG/GeoTIFF
        let image_bytes = vec![0u8; (params.width * params.height) as usize];

        Ok(WmsResponse::Image {
            data: image_bytes,
            mime_type: params.format.clone(),
            width: params.width,
            height: params.height,
        })
    }

    fn handle_get_feature_info(&self, params: &GetFeatureInfoParams) -> Result<WmsResponse, OgcError> {
        // Validate query layers
        for layer_name in &params.query_layers {
            let layer = self.layers.iter().find(|l| l.name == *layer_name);
            match layer {
                Some(l) if !l.queryable => {
                    return Err(OgcError::new(
                        ServiceType::WMS, "1.3.0",
                        "LayerNotQueryable",
                        format!("Layer '{}' is not queryable", layer_name),
                    ));
                }
                None => {
                    return Err(OgcError::new(
                        ServiceType::WMS, "1.3.0",
                        "LayerNotDefined",
                        format!("Layer '{}' not found", layer_name),
                    ));
                }
                _ => {}
            }
        }

        // Placeholder: query features at pixel
        // In production: convert pixel coords → map coords, query spatial index
        let features = serde_json::json!({
            "type": "FeatureCollection",
            "features": [],
            "totalFeatures": 0
        });

        let json_str = serde_json::to_string_pretty(&features).unwrap_or_default();
        Ok(WmsResponse::Xml(json_str))
    }

    /// Build WMS 1.3.0 GetCapabilities XML document.
    pub fn build_capabilities_xml(&self) -> String {
        let layers_xml: String = self.layers
            .iter()
            .map(|l| self.layer_to_xml(l))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<WMS_Capabilities version="1.3.0"
                  xmlns="http://www.opengis.net/wms"
                  xmlns:xlink="http://www.w3.org/1999/xlink">
  <Service>
    <Name>WMS</Name>
    <Title>{title}</Title>
    <OnlineResource xlink:href="{url}"/>
  </Service>
  <Capability>
    <Request>
      <GetCapabilities>
        <Format>text/xml</Format>
      </GetCapabilities>
      <GetMap>
        <Format>image/png</Format>
        <Format>image/jpeg</Format>
        <Format>image/tiff</Format>
      </GetMap>
      <GetFeatureInfo>
        <Format>application/json</Format>
        <Format>text/xml</Format>
      </GetFeatureInfo>
    </Request>
    <Exception>
      <Format>XML</Format>
    </Exception>
    <Layer>
      <Title>{title}</Title>
      <CRS>EPSG:4326</CRS>
      <CRS>EPSG:3857</CRS>
{layers_xml}
    </Layer>
  </Capability>
</WMS_Capabilities>"#,
            title = self.title,
            url = self.online_resource,
            layers_xml = layers_xml,
        )
    }

    fn layer_to_xml(&self, layer: &WmsLayer) -> String {
        let bbox_xml = if let Some(bbox) = &layer.wgs84_bbox {
            format!(
                r#"      <EX_GeographicBoundingBox>
        <westBoundLongitude>{west}</westBoundLongitude>
        <eastBoundLongitude>{east}</eastBoundLongitude>
        <southBoundLatitude>{south}</southBoundLatitude>
        <northBoundLatitude>{north}</northBoundLatitude>
      </EX_GeographicBoundingBox>
      <BoundingBox CRS="EPSG:4326" minx="{west}" miny="{south}" maxx="{east}" maxy="{north}"/>"#,
                west = bbox.west, east = bbox.east, south = bbox.south, north = bbox.north
            )
        } else {
            String::new()
        };

        let crs_xml: String = layer.crs.iter().map(|c| format!("      <CRS>{c}</CRS>")).collect::<Vec<_>>().join("\n");

        let children_xml: String = layer.children.iter().map(|c| self.layer_to_xml(c)).collect::<Vec<_>>().join("\n");

        format!(
            r#"      <Layer queryable="{queryable}">
        <Name>{name}</Name>
        <Title>{title}</Title>
{bbox_xml}
{crs_xml}
{children_xml}
      </Layer>"#,
            name = layer.name,
            title = layer.title,
            queryable = if layer.queryable { "1" } else { "0" },
            bbox_xml = bbox_xml,
            crs_xml = crs_xml,
            children_xml = children_xml,
        )
    }
}

/// WMS response variants.
#[derive(Debug, Clone)]
pub enum WmsResponse {
    /// XML response (GetCapabilities, GetFeatureInfo with text/xml).
    Xml(String),
    /// Binary image response (GetMap).
    Image {
        /// Image bytes.
        data: Vec<u8>,
        /// MIME type (e.g., "image/png").
        mime_type: String,
        /// Image width in pixels.
        width: u32,
        /// Image height in pixels.
        height: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_service() -> WmsService {
        let mut svc = WmsService::new("Test WMS", "https://example.com/wms");
        svc.add_layer(WmsLayer {
            name: "landcover".into(),
            title: "Landcover Classification".into(),
            abstract_: Some("Global land cover map".into()),
            keywords: vec!["landcover".into(), "environment".into()],
            wgs84_bbox: Some(Wgs84Bbox::new(-180.0, -90.0, 180.0, 90.0)),
            crs: vec!["EPSG:4326".into(), "EPSG:3857".into(), "EPSG:3405".into()],
            queryable: true,
            min_scale: None,
            max_scale: None,
            children: vec![],
        });
        svc
    }

    #[test]
    fn test_get_capabilities_xml() {
        let svc = make_service();
        let xml = svc.build_capabilities_xml();
        assert!(xml.contains("WMS_Capabilities"));
        assert!(xml.contains("Landcover"));
        assert!(xml.contains("EPSG:3405"));
    }

    #[test]
    fn test_get_map_valid() {
        let svc = make_service();
        let params = GetMapParams {
            layers: vec!["landcover".into()],
            styles: vec![],
            crs: "EPSG:4326".into(),
            bbox: Wgs84Bbox::new(-180.0, -90.0, 180.0, 90.0),
            width: 800,
            height: 400,
            format: "image/png".into(),
            transparent: true,
            bgcolor: None,
        };
        let result = svc.handle(&WmsRequest::GetMap(params));
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_map_unknown_layer() {
        let svc = make_service();
        let params = GetMapParams {
            layers: vec!["nonexistent".into()],
            styles: vec![],
            crs: "EPSG:4326".into(),
            bbox: Wgs84Bbox::new(0.0, 0.0, 1.0, 1.0),
            width: 100,
            height: 100,
            format: "image/png".into(),
            transparent: false,
            bgcolor: None,
        };
        let result = svc.handle(&WmsRequest::GetMap(params));
        assert!(result.is_err());
    }
}
