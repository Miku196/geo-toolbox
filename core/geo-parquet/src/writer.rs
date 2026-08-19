//! GeoParquet writer — produces spec-compliant files.
//!
//! Writes GeoJSON/vector data to GeoParquet format with
//! spatial metadata (bbox, CRS, geometry types).

use crate::metadata::{projjson_for_epsg, ColumnMetadata, GeoParquetMetadata};
use crate::reader::GeoRecord;
use crate::schema::GeoSchema;
use geo_core::{GeoError, GeoResult};
use geo_types::Geometry;

const WRITE_MSG: &str = "GeoParquet real I/O not implemented yet; this crate currently only provides schema/metadata structures. Writes must be handled by a real backend";

/// Writes GeoParquet files from geometry records.
#[derive(Debug)]
pub struct GeoParquetWriter {
    schema: GeoSchema,
    /// Accumulated bounding box of all features.
    bbox: Option<(f64, f64, f64, f64)>,
    /// Set of geometry types seen.
    geometry_types: Vec<String>,
    /// Features buffer.
    features: Vec<GeoRecord>,
}

impl GeoParquetWriter {
    /// Create a new writer with a schema.
    pub fn new(schema: GeoSchema) -> Self {
        Self {
            schema,
            bbox: None,
            geometry_types: Vec::new(),
            features: Vec::new(),
        }
    }

    /// Add a feature from WKB geometry bytes.
    pub fn add_feature(
        &mut self,
        wkb: Vec<u8>,
        properties: std::collections::HashMap<String, serde_json::Value>,
    ) -> GeoResult<()> {
        // Update bounding box
        if let Some(geom) = parse_wkb_bbox(&wkb) {
            self.update_bbox(&geom);
        }

        self.features.push(GeoRecord {
            geometry: wkb,
            properties,
        });

        Ok(())
    }

    /// Add a feature from a geo_types Geometry.
    ///
    /// Converts to WKB internally. Only [`geo_types::Geometry::Point`]
    /// encoding is currently implemented; any other geometry kind returns
    /// [`GeoError::Unimplemented`] rather than emitting invalid WKB bytes.
    pub fn add_geometry(
        &mut self,
        geometry: &Geometry<f64>,
        properties: std::collections::HashMap<String, serde_json::Value>,
    ) -> GeoResult<()> {
        use geo::algorithm::bounding_rect::BoundingRect;

        // Update bbox
        if let Some(bbox) = geometry.bounding_rect() {
            self.update_bbox_raw(bbox.min().x, bbox.min().y, bbox.max().x, bbox.max().y);
        }

        // Track geometry type
        let geom_type = match geometry {
            Geometry::Point(_) => "Point",
            Geometry::MultiPoint(_) => "MultiPoint",
            Geometry::LineString(_) => "LineString",
            Geometry::MultiLineString(_) => "MultiLineString",
            Geometry::Polygon(_) => "Polygon",
            Geometry::MultiPolygon(_) => "MultiPolygon",
            Geometry::GeometryCollection(_) => "GeometryCollection",
            _ => "Unknown",
        };

        if !self.geometry_types.iter().any(|t| t == geom_type) {
            self.geometry_types.push(geom_type.to_string());
        }

        // WKB encoding — only Point is implemented; non-Point kinds
        // return Err instead of pushing an invalid placeholder byte.
        let wkb = encode_to_wkb(geometry)?;

        self.features.push(GeoRecord {
            geometry: wkb,
            properties,
        });

        Ok(())
    }

    /// Write all buffered features to a GeoParquet file.
    ///
    /// # Honest degradation
    ///
    /// Real GeoParquet disk writing is not implemented yet; writes must be
    /// handled by a real backend. Returning the buffered feature count as if
    /// the file had been written would silently fake success, so this returns
    /// [`GeoError::Unimplemented`] until a real backend lands.
    pub fn write_to_file(&self, _path: &str, _epsg: Option<u32>) -> GeoResult<usize> {
        Err(GeoError::Unimplemented(WRITE_MSG.to_string()))
    }

    /// Build the GeoParquet metadata for the accumulated features.
    pub fn build_metadata(&self, epsg: Option<u32>) -> GeoParquetMetadata {
        let crs = epsg.and_then(projjson_for_epsg);

        let bbox = self
            .bbox
            .map(|(min_x, min_y, max_x, max_y)| vec![min_x, min_y, max_x, max_y]);

        let mut columns = std::collections::HashMap::new();
        columns.insert(
            self.schema.geometry_column.clone(),
            ColumnMetadata {
                encoding: self.schema.geometry_encoding.clone(),
                geometry_types: self.geometry_types.clone(),
                crs,
                bbox,
                edges: Some("planar".into()),
                orientation: Some("counterclockwise".into()),
            },
        );

        GeoParquetMetadata {
            version: "1.1.0".into(),
            primary_column: self.schema.geometry_column.clone(),
            columns,
        }
    }

    /// Get the number of buffered features.
    pub fn len(&self) -> usize {
        self.features.len()
    }

    /// Returns true if no features are buffered.
    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }

    // ── Internal helpers ───────────────────────────────────────

    fn update_bbox(&mut self, geometry: &Geometry<f64>) {
        use geo::algorithm::bounding_rect::BoundingRect;
        if let Some(rect) = geometry.bounding_rect() {
            self.update_bbox_raw(rect.min().x, rect.min().y, rect.max().x, rect.max().y);
        }
    }

    fn update_bbox_raw(&mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) {
        match self.bbox {
            None => self.bbox = Some((min_x, min_y, max_x, max_y)),
            Some((ref mut bx, ref mut by, ref mut bx2, ref mut by2)) => {
                *bx = bx.min(min_x);
                *by = by.min(min_y);
                *bx2 = bx2.max(max_x);
                *by2 = by2.max(max_y);
            }
        }
    }
}

// ── WKB helpers ──────────────────────────────────────────────────

/// Encode a [`geo_types::Geometry`] to WKB bytes.
///
/// Only [`geo_types::Geometry::Point`] is currently implemented (the WKB
/// point encoding: byte-order marker + wkbPoint type tag + x + y). Any other
/// geometry kind returns [`GeoError::Unimplemented`] instead of emitting the
/// invalid placeholder bytes the old skeleton produced. Full WKB encoding
/// should use the `wkb` crate once a real backend lands.
fn encode_to_wkb(geom: &Geometry<f64>) -> GeoResult<Vec<u8>> {
    let mut buf = Vec::new();
    buf.push(1u8); // little-endian byte order marker
    match geom {
        Geometry::Point(p) => {
            buf.extend_from_slice(&1u32.to_le_bytes()); // wkbPoint
            buf.extend_from_slice(&p.x().to_le_bytes());
            buf.extend_from_slice(&p.y().to_le_bytes());
            Ok(buf)
        }
        other => Err(GeoError::Unimplemented(format!(
            "WKB encoding not implemented for non-Point geometry: {other:?}",
        ))),
    }
}

/// Parse WKB bytes to extract bounding box.
fn parse_wkb_bbox(wkb: &[u8]) -> Option<Geometry<f64>> {
    if wkb.len() < 6 {
        return None;
    }
    // Very simplified WKB point extraction for bbox
    let _is_le = wkb[0] == 1u8;
    let geom_type = u32::from_le_bytes([wkb[1], wkb[2], wkb[3], wkb[4]]);

    if geom_type == 1 && wkb.len() >= 21 {
        // Point: 1 + 4 (type) + 8 (x) + 8 (y) = 21 bytes
        let x = f64::from_le_bytes([
            wkb[5], wkb[6], wkb[7], wkb[8], wkb[9], wkb[10], wkb[11], wkb[12],
        ]);
        let y = f64::from_le_bytes([
            wkb[13], wkb[14], wkb[15], wkb[16], wkb[17], wkb[18], wkb[19], wkb[20],
        ]);
        return Some(Geometry::Point(geo_types::Point::new(x, y)));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo_types::Point;

    #[test]
    fn test_writer_add_geometry() {
        let schema = GeoSchema::default();
        let mut writer = GeoParquetWriter::new(schema);

        let point = Geometry::Point(Point::new(104.0, 30.5));
        writer
            .add_geometry(&point, {
                let mut m = std::collections::HashMap::new();
                m.insert("name".into(), serde_json::json!("Chengdu"));
                m
            })
            .unwrap();

        assert_eq!(writer.len(), 1);
    }

    #[test]
    fn test_bbox_tracking() {
        let schema = GeoSchema::default();
        let mut writer = GeoParquetWriter::new(schema);

        writer
            .add_geometry(
                &Geometry::Point(Point::new(103.0, 30.0)),
                std::collections::HashMap::new(),
            )
            .unwrap();
        writer
            .add_geometry(
                &Geometry::Point(Point::new(105.0, 31.0)),
                std::collections::HashMap::new(),
            )
            .unwrap();

        let meta = writer.build_metadata(Some(4326));
        let bbox = meta.columns["geometry"].bbox.as_ref().unwrap();
        assert!((bbox[0] - 103.0).abs() < 0.01);
        assert!((bbox[2] - 105.0).abs() < 0.01);
    }

    #[test]
    fn test_geometry_type_tracking() {
        let schema = GeoSchema::default();
        let mut writer = GeoParquetWriter::new(schema);

        let point = Geometry::Point(Point::new(104.0, 30.5));
        writer
            .add_geometry(&point, std::collections::HashMap::new())
            .unwrap();

        let meta = writer.build_metadata(Some(4326));
        assert!(meta.columns["geometry"]
            .geometry_types
            .contains(&"Point".to_string()));
    }

    #[test]
    fn test_write_to_file_returns_implemented_error_not_fake_count() {
        use geo_core::GeoError;
        let schema = GeoSchema::default();
        let mut writer = GeoParquetWriter::new(schema);
        writer
            .add_geometry(
                &Geometry::Point(Point::new(104.0, 30.5)),
                std::collections::HashMap::new(),
            )
            .unwrap();
        assert_eq!(writer.len(), 1);

        // Real disk write is not implemented: must fail loudly,
        // not fake-return the feature count as if the file was written.
        let err = writer.write_to_file("out.parquet", Some(4326)).unwrap_err();
        assert!(matches!(err, GeoError::Unimplemented(_)), "got {err:?}");
    }

    #[test]
    fn test_add_geometry_non_point_returns_error_not_invalid_wkb() {
        use geo_core::GeoError;
        let schema = GeoSchema::default();
        let mut writer = GeoParquetWriter::new(schema);

        let line = Geometry::LineString(geo_types::LineString::new(vec![
            geo_types::Coord { x: 104.0, y: 30.0 },
            geo_types::Coord { x: 105.0, y: 31.0 },
        ]));

        // Non-Point geometry must not be silently accepted with an
        // invalid placeholder WKB byte.
        let err = writer
            .add_geometry(&line, std::collections::HashMap::new())
            .unwrap_err();
        assert!(matches!(err, GeoError::Unimplemented(_)), "got {err:?}");
    }
}
