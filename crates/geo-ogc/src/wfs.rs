//! WFS (Web Feature Service) — OGC WFS 2.0 implementation.
//!
//! Supports:
//! - `GetCapabilities` — service metadata + feature type listing
//! - `DescribeFeatureType` — schema for a feature type
//! - `GetFeature` — query features (with spatial/temporal/attribute filters)

use serde::{Serialize, Deserialize};
use crate::common::{OgcError, ServiceType, Wgs84Bbox};

/// WFS request types per OGC WFS 2.0 spec.
#[derive(Debug, Clone)]
pub enum WfsRequest {
    /// Get service metadata.
    GetCapabilities,
    /// Get the schema of a feature type.
    DescribeFeatureType(DescribeFeatureTypeParams),
    /// Query features.
    GetFeature(GetFeatureParams),
    /// List stored queries.
    ListStoredQueries,
    /// Execute a stored query.
    DescribeStoredQueries(String),
}

/// Parameters for DescribeFeatureType.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescribeFeatureTypeParams {
    /// Feature type names to describe.
    pub type_names: Vec<String>,
}

/// Parameters for GetFeature query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetFeatureParams {
    /// Feature type names to query.
    pub type_names: Vec<String>,
    /// Maximum features to return.
    #[serde(default = "default_count")]
    pub count: Option<u32>,
    /// OGC Filter (XML string) for spatial/attribute filtering.
    #[serde(default)]
    pub filter: Option<String>,
    /// Spatial bounding box filter (convenience, translated to filter).
    #[serde(default)]
    pub bbox: Option<Wgs84Bbox>,
    /// Output format.
    #[serde(default = "default_output_format")]
    pub output_format: String,
    /// Start index for paging.
    #[serde(default)]
    pub start_index: Option<u32>,
    /// Sort by property name (ascending/descending via +/- prefix).
    #[serde(default)]
    pub sort_by: Option<String>,
    /// Specific property names to return (empty = all).
    #[serde(default)]
    pub property_name: Vec<String>,
    /// CRS for output geometries.
    #[serde(default)]
    pub srs_name: Option<String>,
}

fn default_count() -> Option<u32> { Some(1000) }
fn default_output_format() -> String { "application/gml+xml; version=3.2".into() }

/// A WFS feature type definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureType {
    /// Unique type name (namespace-qualified, e.g., "geo:landcover").
    pub name: String,
    /// Human-readable title.
    pub title: String,
    /// Optional description.
    pub abstract_: Option<String>,
    /// Keywords for search.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// WGS84 bounding box of all features.
    pub wgs84_bbox: Option<Wgs84Bbox>,
    /// Default CRS.
    #[serde(default = "default_crs")]
    pub default_crs: String,
    /// Supported CRS for output.
    #[serde(default)]
    pub other_crs: Vec<String>,
}

fn default_crs() -> String { "EPSG:4326".into() }

/// Property definition for a feature type (DescribeFeatureType response).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureProperty {
    /// Property name.
    pub name: String,
    /// XML Schema type (e.g., "xsd:string", "xsd:double", "gml:PointPropertyType").
    pub type_name: String,
    /// Minimum occurrences.
    #[serde(default = "one")]
    pub min_occurs: u32,
    /// Maximum occurrences (None = unbounded).
    #[serde(default)]
    pub max_occurs: Option<u32>,
    /// Whether this property can be null.
    #[serde(default)]
    pub nillable: bool,
}

fn one() -> u32 { 1 }

/// WFS service implementation.
pub struct WfsService {
    /// Service title.
    pub title: String,
    /// Service endpoint URL.
    pub online_resource: String,
    /// Registered feature types.
    pub feature_types: Vec<FeatureType>,
    /// Max features per request.
    pub max_features: u32,
}

impl WfsService {
    /// Create a new WFS service.
    pub fn new(title: impl Into<String>, online_resource: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            online_resource: online_resource.into(),
            feature_types: Vec::new(),
            max_features: 10_000,
        }
    }

    /// Add a feature type.
    pub fn add_feature_type(&mut self, ft: FeatureType) {
        self.feature_types.push(ft);
    }

    /// Handle a WFS request.
    pub fn handle(&self, request: &WfsRequest) -> Result<WfsResponse, OgcError> {
        match request {
            WfsRequest::GetCapabilities => Ok(WfsResponse::Xml(self.build_capabilities_xml())),
            WfsRequest::DescribeFeatureType(params) => self.handle_describe_feature_type(params),
            WfsRequest::GetFeature(params) => self.handle_get_feature(params),
            WfsRequest::ListStoredQueries => Ok(WfsResponse::Xml(self.empty_stored_queries_xml())),
            WfsRequest::DescribeStoredQueries(_id) => {
                Err(OgcError::new(ServiceType::WFS, "2.0.0", "NotFound", "Stored query not found"))
            }
        }
    }

    fn handle_describe_feature_type(&self, params: &DescribeFeatureTypeParams) -> Result<WfsResponse, OgcError> {
        for name in &params.type_names {
            if !self.feature_types.iter().any(|ft| ft.name == *name) {
                return Err(OgcError::new(
                    ServiceType::WFS, "2.0.0",
                    "NotFound",
                    format!("Feature type '{}' not found", name),
                ));
            }
        }

        Ok(WfsResponse::Xml(self.build_describe_feature_type_xml(&params.type_names)))
    }

    fn handle_get_feature(&self, params: &GetFeatureParams) -> Result<WfsResponse, OgcError> {
        for name in &params.type_names {
            if !self.feature_types.iter().any(|ft| ft.name == *name) {
                return Err(OgcError::new(
                    ServiceType::WFS, "2.0.0",
                    "NotFound",
                    format!("Feature type '{}' not found", name),
                ));
            }
        }

        let count = params.count.unwrap_or(1000).min(self.max_features);
        if count > self.max_features {
            return Err(OgcError::new(
                ServiceType::WFS, "2.0.0",
                "InvalidParameterValue",
                format!("count {count} exceeds max {max}", max = self.max_features),
            ));
        }

        // Placeholder: query features
        // In production: query spatial database / GeoParquet with filter
        let features_json = serde_json::json!({
            "type": "FeatureCollection",
            "features": [],
            "numberMatched": 0,
            "numberReturned": 0,
            "timeStamp": chrono::Utc::now().to_rfc3339(),
        });

        let json_str = serde_json::to_string_pretty(&features_json).unwrap_or_default();
        Ok(WfsResponse::Json(json_str))
    }

    /// Build WFS 2.0 GetCapabilities XML.
    pub fn build_capabilities_xml(&self) -> String {
        let feature_types_xml: String = self.feature_types
            .iter()
            .map(|ft| format!(
                r#"    <FeatureType>
      <Name>{name}</Name>
      <Title>{title}</Title>
      <DefaultCRS>{crs}</DefaultCRS>
    </FeatureType>"#,
                name = ft.name, title = ft.title, crs = ft.default_crs,
            ))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<WFS_Capabilities version="2.0.0"
                  xmlns="http://www.opengis.net/wfs/2.0"
                  xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
                  xsi:schemaLocation="http://www.opengis.net/wfs/2.0
                  http://schemas.opengis.net/wfs/2.0/wfs.xsd">
  <ServiceIdentification>
    <Title>{title}</Title>
  </ServiceIdentification>
  <ServiceProvider>
    <ProviderName>{title}</ProviderName>
    <ServiceContact/>
  </ServiceProvider>
  <FeatureTypeList>
{feature_types_xml}
  </FeatureTypeList>
</WFS_Capabilities>"#,
            title = self.title,
            feature_types_xml = feature_types_xml,
        )
    }

    fn build_describe_feature_type_xml(&self, type_names: &[String]) -> String {
        // Simplified GML Application Schema
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<schema xmlns="http://www.w3.org/2001/XMLSchema"
        xmlns:geo="http://geo-toolbox.dev/geo"
        targetNamespace="http://geo-toolbox.dev/geo"
        elementFormDefault="qualified">
  <import namespace="http://www.opengis.net/gml/3.2"
          schemaLocation="http://schemas.opengis.net/gml/3.2.1/gml.xsd"/>
</schema>"#,
        )
    }

    fn empty_stored_queries_xml(&self) -> String {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<wfs:ListStoredQueriesResponse xmlns:wfs="http://www.opengis.net/wfs/2.0">
  <wfs:StoredQuery id="urn:ogc:def:query:OGC-WFS::GetFeatureById"
                   title="Get feature by ID"/>
</wfs:ListStoredQueriesResponse>"#.to_string()
    }
}

/// WFS response variants.
#[derive(Debug, Clone)]
pub enum WfsResponse {
    /// XML response (GetCapabilities, DescribeFeatureType, GML).
    Xml(String),
    /// GeoJSON response (GetFeature with application/json).
    Json(String),
    /// Binary data (Shapefile zip, GeoPackage, etc.).
    Binary(Vec<u8>),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_service() -> WfsService {
        let mut svc = WfsService::new("Test WFS", "https://example.com/wfs");
        svc.add_feature_type(FeatureType {
            name: "geo:landcover".into(),
            title: "Landcover Polygons".into(),
            abstract_: Some("Landcover classification".into()),
            keywords: vec!["landcover".into()],
            wgs84_bbox: Some(Wgs84Bbox::new(-180.0, -90.0, 180.0, 90.0)),
            default_crs: "EPSG:4326".into(),
            other_crs: vec!["EPSG:3857".into()],
        });
        svc
    }

    #[test]
    fn test_get_capabilities() {
        let svc = make_service();
        let xml = svc.build_capabilities_xml();
        assert!(xml.contains("WFS_Capabilities"));
        assert!(xml.contains("geo:landcover"));
    }

    #[test]
    fn test_describe_feature_type_not_found() {
        let svc = make_service();
        let result = svc.handle(&WfsRequest::DescribeFeatureType(
            DescribeFeatureTypeParams { type_names: vec!["unknown".into()] }
        ));
        assert!(result.is_err());
    }

    #[test]
    fn test_get_feature_not_found() {
        let svc = make_service();
        let result = svc.handle(&WfsRequest::GetFeature(GetFeatureParams {
            type_names: vec!["unknown".into()],
            count: Some(10),
            filter: None,
            bbox: None,
            output_format: "application/json".into(),
            start_index: None,
            sort_by: None,
            property_name: vec![],
            srs_name: None,
        }));
        assert!(result.is_err());
    }
}
