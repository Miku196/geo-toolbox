//! GeoParquet schema utilities.
//!
//! Maps between Rust/geo types and Arrow/Parquet schemas
//! with GeoParquet metadata encoding.

use crate::metadata::ColumnMetadata;

/// Schema definition for a GeoParquet file.
///
/// Describes the non-geometry columns and the geometry encoding.
#[derive(Debug, Clone)]
pub struct GeoSchema {
    /// GeoParquet metadata for config.
    pub geo_meta: crate::metadata::GeoParquetMetadata,

    /// Non-geometry column names and their Arrow types (as strings for portability).
    pub attribute_columns: Vec<AttrColumn>,

    /// Geometry column name (default: "geometry").
    pub geometry_column: String,

    /// Geometry encoding: "WKB" (default) or "WKT" or "point" (raw coords).
    pub geometry_encoding: String,
}

/// A non-geometry attribute column.
#[derive(Debug, Clone)]
pub struct AttrColumn {
    /// Column name.
    pub name: String,
    /// Arrow type as string (e.g., "Int64", "Utf8", "Float64").
    pub arrow_type: String,
}

impl GeoSchema {
    /// Create a schema with WKB-encoded geometry and a set of attribute columns.
    pub fn new(geometry_column: impl Into<String>, attributes: Vec<AttrColumn>) -> Self {
        let geo_col = geometry_column.into();

        let mut columns = std::collections::HashMap::new();
        columns.insert(
            geo_col.clone(),
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
            geo_meta: crate::metadata::GeoParquetMetadata {
                version: "1.1.0".into(),
                primary_column: geo_col.clone(),
                columns,
            },
            attribute_columns: attributes,
            geometry_column: geo_col,
            geometry_encoding: "WKB".into(),
        }
    }

    /// Validate that a column name exists in the schema.
    pub fn has_column(&self, name: &str) -> bool {
        self.attribute_columns.iter().any(|c| c.name == name) || name == self.geometry_column
    }
}

impl Default for GeoSchema {
    fn default() -> Self {
        Self::new(
            "geometry",
            vec![
                AttrColumn {
                    name: "id".into(),
                    arrow_type: "Utf8".into(),
                },
                AttrColumn {
                    name: "name".into(),
                    arrow_type: "Utf8".into(),
                },
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_schema() {
        let schema = GeoSchema::default();
        assert!(schema.has_column("geometry"));
        assert!(schema.has_column("id"));
        assert!(!schema.has_column("nonexistent"));
    }
}
