//! GeoParquet reader — spatial predicate pushdown.
//!
//! Reads GeoParquet files and filters by spatial extent
//! at the row-group level before decoding.

use crate::metadata::GeoParquetMetadata;
use crate::predicate::SpatialFilter;
use crate::schema::GeoSchema;
use geo_core::{GeoError, GeoResult};

/// Reads GeoParquet files with spatial predicate pushdown.
#[derive(Debug)]
pub struct GeoParquetReader {
    /// File path or object store URL.
    path: String,
    /// Parsed GeoParquet metadata.
    metadata: Option<GeoParquetMetadata>,
    /// Schema definition.
    schema: GeoSchema,
}

/// A geometry record read from a GeoParquet file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GeoRecord {
    /// WKB-encoded geometry bytes.
    pub geometry: Vec<u8>,
    /// Attribute columns as key-value pairs.
    pub properties: std::collections::HashMap<String, serde_json::Value>,
}

impl GeoParquetReader {
    /// Create a new reader for a GeoParquet file.
    pub fn new(path: impl Into<String>, schema: GeoSchema) -> Self {
        Self {
            path: path.into(),
            metadata: None,
            schema,
        }
    }

    /// Open the file and parse GeoParquet metadata.
    ///
    /// # Honest degradation
    ///
    /// Real GeoParquet file I/O is not implemented yet. This crate currently
    /// provides only the schema/metadata structures; opening a real file would
    /// silently fake success by returning defaulted metadata, which is unsafe.
    /// Until a real [arrow]/parquet backend lands, this returns
    /// [`GeoError::Unimplemented`] instead of pretending to read the file.
    pub fn open(self) -> GeoResult<Self> {
        Err(GeoError::Unimplemented(
            "GeoParquet real I/O not implemented yet; this crate currently \
             only provides schema/metadata structures"
                .into(),
        ))
    }

    /// Read all features, optionally filtered by spatial predicate.
    ///
    /// # Honest degradation
    ///
    /// Real GeoParquet read + predicate pushdown is not implemented yet.
    /// Returning `Ok(vec![])` would silently pretend the file had no data.
    /// Instead this returns [`GeoError::Unimplemented`] until a real backend
    /// lands.
    pub fn read_with_filter(&self, _filter: Option<&SpatialFilter>) -> GeoResult<Vec<GeoRecord>> {
        Err(GeoError::Unimplemented(
            "GeoParquet real I/O not implemented yet; this crate currently \
             only provides schema/metadata structures"
                .into(),
        ))
    }

    /// Read all features without filtering.
    ///
    /// Delegates to [`Self::read_with_filter`], which currently returns
    /// [`GeoError::Unimplemented`].
    pub fn read_all(&self) -> GeoResult<Vec<GeoRecord>> {
        self.read_with_filter(None)
    }

    /// Get the parsed GeoParquet metadata.
    pub fn metadata(&self) -> Option<&GeoParquetMetadata> {
        self.metadata.as_ref()
    }

    /// Get the file path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get the schema definition.
    pub fn schema(&self) -> &GeoSchema {
        &self.schema
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo_core::GeoError;

    #[test]
    fn test_reader_construct() {
        let schema = GeoSchema::default();
        let reader = GeoParquetReader::new("test.parquet", schema);
        assert_eq!(reader.path, "test.parquet");
        assert!(reader.metadata.is_none());
    }

    #[test]
    fn test_open_returns_implemented_error_not_fake_success() {
        let reader = GeoParquetReader::new("nonexistent.parquet", GeoSchema::default());
        // Real GeoParquet I/O is not implemented: open must fail loudly,
        // not silently return a defaulted metadata as a fake success.
        let err = reader.open().unwrap_err();
        assert!(matches!(err, GeoError::Unimplemented(_)), "got {err:?}");
    }

    #[test]
    fn test_read_all_returns_implemented_error_not_empty() {
        let reader = GeoParquetReader::new("test.parquet", GeoSchema::default());
        // Must not silently return an empty Vec pretending nothing matched.
        let err = reader.read_all().unwrap_err();
        assert!(matches!(err, GeoError::Unimplemented(_)), "got {err:?}");
    }

    #[test]
    fn test_read_with_filter_returns_implemented_error_not_empty() {
        let reader = GeoParquetReader::new("test.parquet", GeoSchema::default());
        let err = reader
            .read_with_filter(Some(&SpatialFilter::Bbox {
                min_x: 103.0,
                min_y: 30.0,
                max_x: 105.0,
                max_y: 31.0,
            }))
            .unwrap_err();
        assert!(matches!(err, GeoError::Unimplemented(_)), "got {err:?}");
    }
}
