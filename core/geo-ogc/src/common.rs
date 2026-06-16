//! Common types shared across OGC service implementations.

use serde::{Deserialize, Serialize};

/// Standard OGC service types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceType {
    /// Web Map Service
    WMS,
    /// Web Feature Service
    WFS,
    /// Web Processing Service
    WPS,
    /// Web Coverage Service
    WCS,
    /// Web Map Tile Service
    WMTS,
}

impl ServiceType {
    /// Get the standard service name string.
    pub fn as_str(&self) -> &'static str {
        match self {
            ServiceType::WMS => "WMS",
            ServiceType::WFS => "WFS",
            ServiceType::WPS => "WPS",
            ServiceType::WCS => "WCS",
            ServiceType::WMTS => "WMTS",
        }
    }
}

/// Standard OGC exception report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OgcException {
    /// Exception code (e.g., "InvalidParameterValue", "OperationNotSupported").
    pub code: String,
    /// Human-readable error message.
    pub locator: String,
    /// Additional text description.
    pub text: String,
}

/// Collection of OGC exceptions (an OWS ExceptionReport).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OgcError {
    /// Service that reported the error.
    pub service: ServiceType,
    /// OGC specification version.
    pub version: String,
    /// List of exception reports.
    pub exceptions: Vec<OgcException>,
}

impl OgcError {
    /// Create a single-exception error report.
    pub fn new(
        service: ServiceType,
        version: impl Into<String>,
        code: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        Self {
            service,
            version: version.into(),
            exceptions: vec![OgcException {
                code: code.into(),
                locator: String::new(),
                text: text.into(),
            }],
        }
    }

    /// Render as OGC XML ExceptionReport.
    pub fn to_xml(&self) -> String {
        let exceptions_xml: String = self
            .exceptions
            .iter()
            .map(|e| {
                format!(
                    r#"    <ows:Exception exceptionCode="{}" locator="{}">
        <ows:ExceptionText>{}</ows:ExceptionText>
    </ows:Exception>"#,
                    e.code, e.locator, e.text
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<ows:ExceptionReport xmlns:ows="http://www.opengis.net/ows/2.0"
                     xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
                     xsi:schemaLocation="http://www.opengis.net/ows/2.0
                     http://schemas.opengis.net/ows/2.0/owsExceptionReport.xsd"
                     version="{version}" language="en">
{exceptions_xml}
</ows:ExceptionReport>"#,
            version = self.version,
            exceptions_xml = exceptions_xml,
        )
    }
}

/// Geographic bounding box (WGS84).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Wgs84Bbox {
    /// West longitude.
    pub west: f64,
    /// South latitude.
    pub south: f64,
    /// East longitude.
    pub east: f64,
    /// North latitude.
    pub north: f64,
}

impl Wgs84Bbox {
    /// Create a new WGS84 bounding box.
    pub fn new(west: f64, south: f64, east: f64, north: f64) -> Self {
        Self {
            west,
            south,
            east,
            north,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ogc_error_xml() {
        let error = OgcError::new(
            ServiceType::WMS,
            "1.3.0",
            "InvalidParameterValue",
            "Layer 'unknown' not found",
        );
        let xml = error.to_xml();
        assert!(xml.contains("InvalidParameterValue"));
        assert!(xml.contains("unknown"));
        assert!(xml.contains("ExceptionReport"));
    }

    #[test]
    fn test_service_type_str() {
        assert_eq!(ServiceType::WMS.as_str(), "WMS");
        assert_eq!(ServiceType::WFS.as_str(), "WFS");
        assert_eq!(ServiceType::WPS.as_str(), "WPS");
    }
}
