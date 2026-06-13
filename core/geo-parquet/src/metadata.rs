//! GeoParquet metadata — encoding of spatial reference info.
//!
//! Implements the [GeoParquet 1.1 specification](https://geoparquet.org/).

use serde::{Deserialize, Serialize};

/// GeoParquet file-level metadata, stored in the Parquet file's key-value metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeoParquetMetadata {
    /// GeoParquet specification version (e.g., "1.1.0").
    pub version: String,

    /// The primary geometry column name.
    pub primary_column: String,

    /// Per-column geometry metadata.
    pub columns: std::collections::HashMap<String, ColumnMetadata>,
}

impl Default for GeoParquetMetadata {
    fn default() -> Self {
        let mut columns = std::collections::HashMap::new();
        columns.insert(
            "geometry".into(),
            ColumnMetadata {
                encoding: "WKB".into(),
                geometry_types: vec![],
                crs: None,
                bbox: None,
                edges: None,
                orientation: None,
            },
        );
        Self {
            version: "1.1.0".into(),
            primary_column: "geometry".into(),
            columns,
        }
    }
}

/// Per-column geometry metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColumnMetadata {
    /// Geometry encoding format.
    /// "WKB" (Well-Known Binary) is the only mandatory encoding.
    pub encoding: String,

    /// Geometry types present in this column.
    /// Empty = all types possible.
    #[serde(default)]
    pub geometry_types: Vec<String>,

    /// Coordinate Reference System (PROJJSON format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crs: Option<serde_json::Value>,

    /// Bounding box covering all geometries in the file.
    /// [xmin, ymin, xmax, ymax] for 2D.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<Vec<f64>>,

    /// "planar" or "spherical" — how edges are interpreted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edges: Option<String>,

    /// Polygon winding order: "counterclockwise" or "clockwise".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation: Option<String>,
}

/// CRS definition in PROJJSON format (subset).
///
/// See: <https://proj.org/specifications/projjson.html>
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjJsonCrs {
    /// Must be "PROJJSON".
    #[serde(rename = "type")]
    pub type_: String,

    /// PROJJSON object with id, name, coordinate_system, etc.
    pub properties: ProjJsonProperties,
}

/// Core PROJJSON properties.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjJsonProperties {
    /// Human-readable name.
    pub name: String,

    /// Authority + code identifier.
    pub id: Option<AuthorityCode>,

    /// Coordinate system type.
    #[serde(rename = "coordinate_system")]
    pub coordinate_system: Option<serde_json::Value>,
}

/// Authority and code (e.g., EPSG:4326).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthorityCode {
    /// Authority name (e.g., "EPSG").
    pub authority: String,
    /// Code within that authority.
    pub code: u32,
}

/// Build PROJJSON CRS for common EPSG codes.
pub fn projjson_for_epsg(epsg: u32) -> Option<serde_json::Value> {
    let (name, cs_type) = match epsg {
        4326 => ("WGS 84", "geographic"),
        3857 => ("WGS 84 / Pseudo-Mercator", "projected"),
        32649 => ("WGS 84 / UTM zone 49N", "projected"),
        3405 => ("World Equal Area", "projected"),
        4547 => ("CGCS2000", "geographic"),
        _ => return None,
    };

    Some(serde_json::json!({
        "type": "PROJJSON",
        "properties": {
            "name": name,
            "id": {
                "authority": "EPSG",
                "code": epsg
            },
            "coordinate_system": {
                "type": cs_type
            }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_metadata() {
        let meta = GeoParquetMetadata::default();
        assert_eq!(meta.version, "1.1.0");
        assert!(meta.columns.contains_key("geometry"));
    }

    #[test]
    fn test_projjson_wgs84() {
        let crs = projjson_for_epsg(4326).unwrap();
        assert_eq!(crs["type"], "PROJJSON");
        assert!(crs["properties"]["name"].as_str().unwrap().contains("WGS"));
    }
}
