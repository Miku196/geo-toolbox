//! WPS (Web Processing Service) — OGC WPS 2.0 implementation.
//!
//! Supports:
//! - `GetCapabilities` — service metadata + process listing
//! - `DescribeProcess` — input/output schema for a process
//! - `Execute` — run a geospatial processing task (sync or async)

use serde::{Serialize, Deserialize};
use crate::common::{OgcError, ServiceType};

/// WPS request types per OGC WPS 2.0 spec.
#[derive(Debug, Clone)]
pub enum WpsRequest {
    /// Get service metadata and available processes.
    GetCapabilities,
    /// Get input/output details for a process.
    DescribeProcess(DescribeProcessParams),
    /// Execute a process.
    Execute(ExecuteParams),
    /// Check status of an async execution.
    GetStatus(String),
    /// Retrieve results of a completed execution.
    GetResult(String),
}

/// Parameters for DescribeProcess.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescribeProcessParams {
    /// Process identifiers to describe.
    pub identifiers: Vec<String>,
}

/// Parameters for Execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteParams {
    /// Process identifier.
    pub identifier: String,
    /// Process inputs.
    pub inputs: Vec<ProcessInput>,
    /// Requested outputs.
    pub outputs: Vec<ProcessOutput>,
    /// Execution mode: "sync" (default) or "async".
    #[serde(default = "default_mode")]
    pub mode: String,
    /// Response format: "raw" (default) or "document".
    #[serde(default = "default_response")]
    pub response: String,
}

fn default_mode() -> String { "sync".into() }
fn default_response() -> String { "raw".into() }

/// Input parameter to a WPS process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInput {
    /// Input identifier (matches process definition).
    pub id: String,
    /// Input data (inline value or reference).
    pub data: InputData,
}

/// Input data variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum InputData {
    /// Literal value (string, number, boolean).
    #[serde(rename = "literal")]
    Literal {
        /// Data type.
        data_type: String,
        /// String representation of the value.
        value: String,
    },
    /// Reference to external data (URL).
    #[serde(rename = "reference")]
    Reference {
        /// URL to the data.
        href: String,
        /// MIME type.
        mime_type: Option<String>,
        /// CRS if spatial.
        crs: Option<String>,
    },
    /// Inline complex data (JSON, XML, binary).
    #[serde(rename = "complex")]
    Complex {
        /// MIME type.
        mime_type: String,
        /// Data content (base64 for binary, raw text otherwise).
        content: String,
    },
    /// Bounding box input.
    #[serde(rename = "bbox")]
    Bbox {
        /// CRS.
        crs: String,
        /// [min_x, min_y, max_x, max_y].
        bbox: Vec<f64>,
    },
}

/// Requested output from a WPS process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessOutput {
    /// Output identifier (matches process definition).
    pub id: String,
    /// Requested MIME type.
    #[serde(default)]
    pub mime_type: Option<String>,
    /// Whether to transmit the output inline.
    #[serde(default)]
    pub transmission: Option<String>,
}

/// A registered WPS process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WpsProcess {
    /// Unique process identifier.
    pub identifier: String,
    /// Human-readable title.
    pub title: String,
    /// Optional description.
    pub abstract_: Option<String>,
    /// Process version.
    #[serde(default = "default_process_version")]
    pub version: String,
    /// Input definitions.
    #[serde(default)]
    pub inputs: Vec<ProcessParamDef>,
    /// Output definitions.
    #[serde(default)]
    pub outputs: Vec<ProcessParamDef>,
}

fn default_process_version() -> String { "1.0.0".into() }

/// Definition of a process input or output parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessParamDef {
    /// Parameter identifier.
    pub identifier: String,
    /// Human-readable title.
    pub title: String,
    /// Optional description.
    pub abstract_: Option<String>,
    /// Parameter type: "literal", "complex", "bbox".
    pub param_type: String,
    /// For literal types: the data type (e.g., "string", "double", "integer").
    pub data_type: Option<String>,
    /// Allowed MIME types for complex data.
    pub mime_types: Option<Vec<String>>,
    /// Minimum occurrences.
    #[serde(default)]
    pub min_occurs: u32,
    /// Maximum occurrences (None = unbounded).
    #[serde(default)]
    pub max_occurs: Option<u32>,
}

/// WPS service implementation.
pub struct WpsService {
    /// Service title.
    pub title: String,
    /// Service endpoint URL.
    pub online_resource: String,
    /// Registered processes.
    pub processes: Vec<WpsProcess>,
}

impl WpsService {
    /// Create a new WPS service.
    pub fn new(title: impl Into<String>, online_resource: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            online_resource: online_resource.into(),
            processes: Vec::new(),
        }
    }

    /// Register a process.
    pub fn add_process(&mut self, process: WpsProcess) {
        self.processes.push(process);
    }

    /// Handle a WPS request.
    pub fn handle(&self, request: &WpsRequest) -> Result<WpsResponse, OgcError> {
        match request {
            WpsRequest::GetCapabilities => Ok(WpsResponse::Xml(self.build_capabilities_xml())),
            WpsRequest::DescribeProcess(params) => self.handle_describe_process(params),
            WpsRequest::Execute(params) => self.handle_execute(params),
            WpsRequest::GetStatus(job_id) => self.handle_get_status(job_id),
            WpsRequest::GetResult(job_id) => self.handle_get_result(job_id),
        }
    }

    fn handle_describe_process(&self, params: &DescribeProcessParams) -> Result<WpsResponse, OgcError> {
        for id in &params.identifiers {
            if !self.processes.iter().any(|p| p.identifier == *id) {
                return Err(OgcError::new(
                    ServiceType::WPS, "2.0.0",
                    "InvalidParameterValue",
                    format!("Process '{}' not found", id),
                ));
            }
        }
        Ok(WpsResponse::Xml(self.build_describe_process_xml(&params.identifiers)))
    }

    fn handle_execute(&self, params: &ExecuteParams) -> Result<WpsResponse, OgcError> {
        if !self.processes.iter().any(|p| p.identifier == params.identifier) {
            return Err(OgcError::new(
                ServiceType::WPS, "2.0.0",
                "InvalidParameterValue",
                format!("Process '{}' not found", params.identifier),
            ));
        }

        if params.mode == "async" {
            let job_id = uuid::Uuid::new_v4().to_string();
            return Ok(WpsResponse::Xml(format!(
                r#"<wps:StatusInfo>
  <wps:JobID>{job_id}</wps:JobID>
  <wps:Status>Accepted</wps:Status>
</wps:StatusInfo>"#,
                job_id = job_id,
            )));
        }

        // Placeholder: execute process synchronously
        Ok(WpsResponse::Xml(r#"<wps:ProcessOutputs>
  <wps:Output id="result">
    <wps:Data>Processing complete.</wps:Data>
  </wps:Output>
</wps:ProcessOutputs>"#.to_string()))
    }

    fn handle_get_status(&self, _job_id: &str) -> Result<WpsResponse, OgcError> {
        Ok(WpsResponse::Xml(r#"<wps:StatusInfo>
  <wps:Status>Succeeded</wps:Status>
</wps:StatusInfo>"#.to_string()))
    }

    fn handle_get_result(&self, _job_id: &str) -> Result<WpsResponse, OgcError> {
        Ok(WpsResponse::Xml(r#"<wps:ProcessOutputs>
  <wps:Output id="result">
    <wps:Data>Result data here.</wps:Data>
  </wps:Output>
</wps:ProcessOutputs>"#.to_string()))
    }

    /// Build WPS 2.0 GetCapabilities XML.
    pub fn build_capabilities_xml(&self) -> String {
        let processes_xml: String = self.processes
            .iter()
            .map(|p| format!(
                r#"      <Process>
        <ows:Identifier>{id}</ows:Identifier>
        <ows:Title>{title}</ows:Title>
        <ows:Abstract>{abstract_}</ows:Abstract>
        <wps:ProcessVersion>{version}</wps:ProcessVersion>
      </Process>"#,
                id = p.identifier,
                title = p.title,
                abstract_ = p.abstract_.as_deref().unwrap_or(""),
                version = p.version,
            ))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<wps:Capabilities version="2.0.0"
                  xmlns:wps="http://www.opengis.net/wps/2.0"
                  xmlns:ows="http://www.opengis.net/ows/2.0">
  <ows:ServiceIdentification>
    <ows:Title>{title}</ows:Title>
  </ows:ServiceIdentification>
  <wps:ProcessOfferings>
{processes_xml}
  </wps:ProcessOfferings>
</wps:Capabilities>"#,
            title = self.title,
            processes_xml = processes_xml,
        )
    }

    fn build_describe_process_xml(&self, identifiers: &[String]) -> String {
        // In production: build complete ProcessOffering with inputs/outputs
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<wps:ProcessOffering xmlns:wps="http://www.opengis.net/wps/2.0">
  <wps:Process>
    <ows:Identifier>{id}</ows:Identifier>
  </wps:Process>
</wps:ProcessOffering>"#,
            id = identifiers.first().map(|s| s.as_str()).unwrap_or("unknown"),
        )
    }

    /// Register built-in geo-toolbox processes.
    pub fn register_builtin_processes(&mut self) {
        self.add_process(WpsProcess {
            identifier: "carbon:emission-factor".into(),
            title: "Carbon Emission Factor Calculation".into(),
            abstract_: Some("IPCC Tier 1 emission factor method".into()),
            version: "1.0.0".into(),
            inputs: vec![
                ProcessParamDef {
                    identifier: "features".into(),
                    title: "Landcover Features".into(),
                    abstract_: Some("GeoJSON FeatureCollection with landcover polygons".into()),
                    param_type: "complex".into(),
                    data_type: None,
                    mime_types: Some(vec!["application/geo+json".into()]),
                    min_occurs: 1,
                    max_occurs: Some(1),
                },
                ProcessParamDef {
                    identifier: "factors".into(),
                    title: "Emission Factors".into(),
                    abstract_: Some("CSV: category,factor_value,source".into()),
                    param_type: "complex".into(),
                    data_type: None,
                    mime_types: Some(vec!["text/csv".into()]),
                    min_occurs: 1,
                    max_occurs: Some(1),
                },
                ProcessParamDef {
                    identifier: "year".into(),
                    title: "Target Year".into(),
                    abstract_: Some("Year for factor validity check".into()),
                    param_type: "literal".into(),
                    data_type: Some("integer".into()),
                    mime_types: None,
                    min_occurs: 1,
                    max_occurs: Some(1),
                },
            ],
            outputs: vec![
                ProcessParamDef {
                    identifier: "report".into(),
                    title: "Carbon Report".into(),
                    abstract_: Some("JSON: {total_area_ha, total_emission_tco2e, classes, ...}".into()),
                    param_type: "complex".into(),
                    data_type: None,
                    mime_types: Some(vec!["application/json".into()]),
                    min_occurs: 1,
                    max_occurs: Some(1),
                },
            ],
        });

        self.add_process(WpsProcess {
            identifier: "crs:transform".into(),
            title: "Coordinate Transform".into(),
            abstract_: Some("Transform coordinates between CRS".into()),
            version: "1.0.0".into(),
            inputs: vec![
                ProcessParamDef {
                    identifier: "coordinates".into(),
                    title: "Coordinates".into(),
                    abstract_: Some("JSON: [{x,y},...] or pair [x,y]".into()),
                    param_type: "complex".into(),
                    data_type: None,
                    mime_types: Some(vec!["application/json".into()]),
                    min_occurs: 1,
                    max_occurs: Some(1),
                },
                ProcessParamDef {
                    identifier: "from_epsg".into(),
                    title: "Source EPSG".into(),
                    abstract_: None,
                    param_type: "literal".into(),
                    data_type: Some("integer".into()),
                    mime_types: None,
                    min_occurs: 1,
                    max_occurs: Some(1),
                },
                ProcessParamDef {
                    identifier: "to_epsg".into(),
                    title: "Target EPSG".into(),
                    abstract_: None,
                    param_type: "literal".into(),
                    data_type: Some("integer".into()),
                    mime_types: None,
                    min_occurs: 1,
                    max_occurs: Some(1),
                },
            ],
            outputs: vec![
                ProcessParamDef {
                    identifier: "transformed".into(),
                    title: "Transformed Coordinates".into(),
                    abstract_: Some("JSON: [{x,y},...]".into()),
                    param_type: "complex".into(),
                    data_type: None,
                    mime_types: Some(vec!["application/json".into()]),
                    min_occurs: 1,
                    max_occurs: Some(1),
                },
            ],
        });
    }
}

/// WPS response variants.
#[derive(Debug, Clone)]
pub enum WpsResponse {
    /// XML response (GetCapabilities, DescribeProcess, status).
    Xml(String),
    /// JSON response (execute result).
    Json(String),
    /// Binary result (GeoTIFF, GeoPackage, etc.).
    Binary(Vec<u8>),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_service() -> WpsService {
        let mut svc = WpsService::new("Test WPS", "https://example.com/wps");
        svc.register_builtin_processes();
        svc
    }

    #[test]
    fn test_get_capabilities() {
        let svc = make_service();
        let xml = svc.build_capabilities_xml();
        assert!(xml.contains("wps:Capabilities"));
        assert!(xml.contains("carbon:emission-factor"));
        assert!(xml.contains("crs:transform"));
    }

    #[test]
    fn test_describe_process_exists() {
        let svc = make_service();
        let result = svc.handle(&WpsRequest::DescribeProcess(
            DescribeProcessParams { identifiers: vec!["carbon:emission-factor".into()] }
        ));
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_unknown_process() {
        let svc = make_service();
        let result = svc.handle(&WpsRequest::Execute(ExecuteParams {
            identifier: "unknown".into(),
            inputs: vec![],
            outputs: vec![],
            mode: "sync".into(),
            response: "raw".into(),
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_async_execute() {
        let svc = make_service();
        let result = svc.handle(&WpsRequest::Execute(ExecuteParams {
            identifier: "crs:transform".into(),
            inputs: vec![],
            outputs: vec![],
            mode: "async".into(),
            response: "raw".into(),
        }));
        assert!(result.is_ok());
        if let Ok(WpsResponse::Xml(xml)) = result {
            assert!(xml.contains("JobID"));
        }
    }
}
