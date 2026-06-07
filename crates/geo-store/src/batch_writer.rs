//! High-throughput batch writer using PostgreSQL COPY protocol.
//!
//! Uses `COPY ... FROM STDIN WITH (FORMAT binary)` to insert thousands of
//! rows in a single round-trip. For spatial data, this is 10–50× faster
//! than individual INSERT statements.

use geo_core::errors::GeoResult;
use sqlx::postgres::{PgPool, PgPoolCopyExt};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time;

/// A single row to be batch-inserted into `spatial_assets`.
#[derive(Debug, Clone)]
pub struct SpatialRow {
    /// WKB bytes for the geometry column.
    pub wkb: Vec<u8>,
    /// JSON properties.
    pub properties: String,
    /// Data source identifier.
    pub source: String,
    /// CRS in "EPSG:nnnn" format.
    pub crs: String,
}

impl SpatialRow {
    /// Helper: create from WKB bytes + optional JSON properties.
    pub fn new(wkb: Vec<u8>, properties: serde_json::Value, source: &str) -> Self {
        Self {
            wkb,
            properties: properties.to_string(),
            source: source.to_string(),
            crs: "EPSG:4326".to_string(),
        }
    }
}

/// Wraps a `PgPool` with a buffered COPY writer.
///
/// ## Usage
/// ```ignore
/// let mut writer = BatchWriter::new(pool, 500);
/// writer.push(SpatialRow::new(wkb, props, "CamoFox"));
/// let rows = writer.flush().await?;
/// ```
pub struct BatchWriter {
    pool: PgPool,
    buffer: Vec<SpatialRow>,
    /// Max rows before auto-flush.
    pub batch_size: usize,
}

impl BatchWriter {
    /// Create a new batch writer.
    pub fn new(pool: PgPool, batch_size: usize) -> Self {
        Self {
            pool,
            buffer: Vec::with_capacity(batch_size),
            batch_size,
        }
    }

    /// Number of rows currently buffered.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Returns `true` if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Buffer a row for later flush. Does **not** write to the database yet.
    pub fn push(&mut self, row: SpatialRow) {
        self.buffer.push(row);
    }

    /// Flush all buffered rows to PostgreSQL via COPY.
    ///
    /// Returns the number of rows written.
    pub async fn flush(&mut self) -> GeoResult<u64> {
        if self.buffer.is_empty() {
            return Ok(0);
        }

        let row_count = self.buffer.len() as u64;
        let start = Instant::now();

        let mut copy = self
            .pool
            .copy_in_raw(
                "COPY spatial_assets (geom, properties, source, crs, ingested_at) \
                 FROM STDIN WITH (FORMAT binary)",
            )
            .await
            .map_err(|e| geo_core::errors::GeoError::Database(e.to_string()))?;

        for row in self.buffer.drain(..) {
            let binary_row = encode_copy_row(&row);
            copy.send(binary_row).await.map_err(|e| {
                geo_core::errors::GeoError::Database(format!("COPY send: {e}"))
            })?;
        }

        copy.finish()
            .await
            .map_err(|e| geo_core::errors::GeoError::Database(e.to_string()))?;

        let elapsed = start.elapsed();
        tracing::info!(
            "BatchWriter flushed {row_count} rows in {:.2?} ({:.0} rows/s)",
            elapsed,
            row_count as f64 / elapsed.as_secs_f64()
        );

        Ok(row_count)
    }

    /// Push + auto-flush when buffer reaches `batch_size`.
    pub async fn push_and_maybe_flush(&mut self, row: SpatialRow) -> GeoResult<Option<u64>> {
        self.push(row);
        if self.len() >= self.batch_size {
            Ok(Some(self.flush().await?))
        } else {
            Ok(None)
        }
    }
}

/// Spawn a background task that flushes at a regular interval or when the
/// buffer reaches `batch_size`.
pub async fn batch_worker(
    pool: PgPool,
    mut rx: mpsc::Receiver<SpatialRow>,
    batch_size: usize,
    flush_interval: Duration,
) {
    let mut writer = BatchWriter::new(pool, batch_size);
    let mut interval = time::interval(flush_interval);

    loop {
        tokio::select! {
            maybe_row = rx.recv() => {
                match maybe_row {
                    Some(row) => {
                        writer.push(row);
                        if writer.len() >= writer.batch_size {
                            if let Err(e) = writer.flush().await {
                                tracing::error!("BatchWorker flush error: {e}");
                            }
                        }
                    }
                    None => {
                        // Channel closed — final flush
                        if let Err(e) = writer.flush().await {
                            tracing::error!("BatchWorker final flush error: {e}");
                        }
                        break;
                    }
                }
            }
            _ = interval.tick() => {
                if !writer.is_empty() {
                    if let Err(e) = writer.flush().await {
                        tracing::error!("BatchWorker interval flush error: {e}");
                    }
                }
            }
        }
    }
}

// ── Binary COPY row encoding ──────────────────────────────────────

fn encode_copy_row(row: &SpatialRow) -> Vec<u8> {
    // PostgreSQL binary COPY format:
    //   [PGCOPY signature (11 bytes)] [flags (4)] [header_ext (4)]
    //   For each row:
    //     [field_count (2)]
    //     For each field:
    //       [size (4)] [data]
    //
    // This is a simplified encoding. A production implementation should
    // use a dedicated crate (e.g., `postgres-binary-copy` or manual
    // encoding with proper field type OIDs).

    let mut buf = Vec::new();

    // PGCOPY magic
    buf.extend_from_slice(b"PGCOPY\n\xff\r\n\0");
    // Flags (bit 15 = 1 if OID included — not in our case)
    buf.extend_from_slice(&0u32.to_be_bytes());
    // Header extension length
    buf.extend_from_slice(&0u32.to_be_bytes());

    // ── Row data ──
    let field_count: u16 = 5; // geom, properties, source, crs, ingested_at
    buf.extend_from_slice(&field_count.to_be_bytes());

    // Field 1: geom (WKB → bytea)
    let wkb_len = row.wkb.len() as i32;
    buf.extend_from_slice(&wkb_len.to_be_bytes());
    buf.extend_from_slice(&row.wkb);

    // Field 2: properties (JSON → text)
    let props = row.properties.as_bytes();
    let props_len = props.len() as i32;
    buf.extend_from_slice(&props_len.to_be_bytes());
    buf.extend_from_slice(props);

    // Field 3: source (text)
    let src = row.source.as_bytes();
    let src_len = src.len() as i32;
    buf.extend_from_slice(&src_len.to_be_bytes());
    buf.extend_from_slice(src);

    // Field 4: crs (text)
    let crs = row.crs.as_bytes();
    let crs_len = crs.len() as i32;
    buf.extend_from_slice(&crs_len.to_be_bytes());
    buf.extend_from_slice(crs);

    // Field 5: ingested_at = NOW() (NULL → -1)
    buf.extend_from_slice(&(-1i32).to_be_bytes());

    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_encode_copy_row() {
        let row = SpatialRow::new(
            vec![1, 2, 3],
            json!({"class": "forest", "confidence": 0.95}),
            "CamoFox",
        );
        let encoded = encode_copy_row(&row);

        // Should start with PGCOPY signature
        assert_eq!(&encoded[0..11], b"PGCOPY\n\xff\r\n\0");
        // Should contain the property JSON
        let encoded_str = String::from_utf8_lossy(&encoded);
        assert!(encoded_str.contains("forest"));
    }

    #[test]
    fn test_batch_writer_buffer() {
        // Test buffer count/empty without needing a tokio runtime or real DB.
        // PgPool::connect_lazy with an invalid URL creates a pool that
        // panics on first use, but doesn't allocate anything.

        // We can't actually create a PgPool without tokio, so test the
        // SpatialRow struct directly.
        let row = SpatialRow::new(
            vec![1, 2, 3],
            json!({"class": "forest", "confidence": 0.95}),
            "CamoFox",
        );
        assert_eq!(row.source, "CamoFox");
        assert!(row.properties.contains("forest"));
        assert_eq!(row.crs, "EPSG:4326");
    }
}
