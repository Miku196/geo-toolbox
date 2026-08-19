//! Chunked raster processing — iterate over a raster in fixed-size tiles.
//!
//! Useful for processing large rasters without holding the entire output in memory,
//! and for resumable / checkpointable computation.

/// Iterates over a raster in fixed-size chunks (default 256×256).
///
/// Each item is `(chunk_x, chunk_y, chunk_cols, chunk_rows, chunk_data)`:
/// - `chunk_x`, `chunk_y`: tile coordinate (0-indexed)
/// - `chunk_cols`, `chunk_rows`: actual size of this tile (may be smaller at edges)
/// - `chunk_data`: flattened row-major f64 values for this tile
pub struct ChunkIterator {
    data: Vec<f64>,
    cols: usize,
    rows: usize,
    chunk_size: usize,
    chunks_x: usize,
    chunks_y: usize,
    current: usize,
    total: usize,
}

impl ChunkIterator {
    /// Create a new chunk iterator over the row-major data buffer,
    /// splitting it into chunk_size-sized tiles (edge tiles may be smaller).
    pub fn new(data: Vec<f64>, cols: usize, rows: usize, chunk_size: usize) -> Self {
        let chunks_x = cols.div_ceil(chunk_size);
        let chunks_y = rows.div_ceil(chunk_size);
        let total = chunks_x * chunks_y;
        Self {
            data,
            cols,
            rows,
            chunk_size,
            chunks_x,
            chunks_y,
            current: 0,
            total,
        }
    }

    /// Total number of chunks
    pub fn total_chunks(&self) -> usize {
        self.total
    }

    /// Chunk grid dimensions (tiles_x × tiles_y)
    pub fn grid_dims(&self) -> (usize, usize) {
        (self.chunks_x, self.chunks_y)
    }
}

impl Iterator for ChunkIterator {
    type Item = (usize, usize, usize, usize, Vec<f64>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.total {
            return None;
        }

        let chunk_y = self.current / self.chunks_x;
        let chunk_x = self.current % self.chunks_x;

        let pixel_y_start = chunk_y * self.chunk_size;
        let pixel_x_start = chunk_x * self.chunk_size;

        let chunk_cols = (self.cols - pixel_x_start).min(self.chunk_size);
        let chunk_rows = (self.rows - pixel_y_start).min(self.chunk_size);

        let mut chunk = Vec::with_capacity(chunk_cols * chunk_rows);
        for r in 0..chunk_rows {
            let src_row = pixel_y_start + r;
            let src_start = src_row * self.cols + pixel_x_start;
            let src_end = src_start + chunk_cols;
            chunk.extend_from_slice(&self.data[src_start..src_end]);
        }

        self.current += 1;
        Some((chunk_x, chunk_y, chunk_cols, chunk_rows, chunk))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.total - self.current;
        (remaining, Some(remaining))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_iterator_exact() {
        // 4x4 raster with chunk_size=2 → 4 chunks, each 2x2
        let data: Vec<f64> = (0..16).map(|i| i as f64).collect();
        let iter = ChunkIterator::new(data, 4, 4, 2);
        assert_eq!(iter.total_chunks(), 4);

        let chunks: Vec<_> = iter.collect();
        assert_eq!(chunks.len(), 4);
        for (_cx, _cy, ccols, crows, _) in &chunks {
            assert_eq!(*ccols, 2);
            assert_eq!(*crows, 2);
        }
    }

    #[test]
    fn test_chunk_iterator_partial() {
        // 3x3 raster with chunk_size=2 → 4 chunks: 2×2, 1×2, 2×1, 1×1
        let data: Vec<f64> = (0..9).map(|i| i as f64).collect();
        let iter = ChunkIterator::new(data, 3, 3, 2);
        assert_eq!(iter.total_chunks(), 4);

        let chunks: Vec<_> = iter.collect();
        // Top-left: 2x2
        assert_eq!(chunks[0], (0, 0, 2, 2, vec![0.0, 1.0, 3.0, 4.0]));
        // Top-right: 1x2
        assert_eq!(chunks[1], (1, 0, 1, 2, vec![2.0, 5.0]));
        // Bottom-left: 2x1
        assert_eq!(chunks[2], (0, 1, 2, 1, vec![6.0, 7.0]));
        // Bottom-right: 1x1
        assert_eq!(chunks[3], (1, 1, 1, 1, vec![8.0]));
    }

    #[test]
    fn test_chunk_iterator_large() {
        // 256x256 with chunk_size=256 → exactly 1 chunk
        let data = vec![42.0; 256 * 256];
        let iter = ChunkIterator::new(data, 256, 256, 256);
        assert_eq!(iter.total_chunks(), 1);
        let chunks: Vec<_> = iter.collect();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].4.len(), 256 * 256);
    }
}
