use std::collections::HashMap;
use std::sync::Mutex;

/// Key for identifying a cached tile. The output `format` is part of the key so
/// that different encodings (e.g. PNG vs MVT) of the same tile do not collide.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TileKey {
    layer: String,
    tile_matrix_set: String,
    tile_matrix: String,
    tile_col: u32,
    tile_row: u32,
    format: String,
}

/// In-memory tile cache with memory limit.
///
/// The backing map is wrapped in a `Mutex` so the cache can be read *and*
/// written through an `&self` handle (the serving code previously only read and
/// never inserted, which made the cache a permanent miss). `TileCache` remains
/// `Send + Sync`, so it can be stored behind an `Arc` in an HTTP handler.
pub struct TileCache {
    tiles: Mutex<HashMap<TileKey, Vec<u8>>>,
    pub(crate) max_entries: usize,
}

impl TileCache {
    /// Create a new tile cache.
    pub fn new(max_entries: usize) -> Self {
        Self {
            tiles: Mutex::new(HashMap::new()),
            max_entries,
        }
    }

    /// Get a cached tile.
    pub fn get(
        &self,
        layer: &str,
        tile_matrix_set: &str,
        tile_matrix: &str,
        tile_col: u32,
        tile_row: u32,
        format: &str,
    ) -> Option<Vec<u8>> {
        let key = TileKey {
            layer: layer.to_string(),
            tile_matrix_set: tile_matrix_set.to_string(),
            tile_matrix: tile_matrix.to_string(),
            tile_col,
            tile_row,
            format: format.to_string(),
        };
        self.tiles
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(&key)
            .cloned()
    }

    /// Insert a tile into the cache.
    ///
    /// When full, the entire cache is cleared before inserting (simple capacity
    /// policy, unchanged from the original behaviour).
    pub fn insert(
        &self,
        layer: &str,
        tile_matrix_set: &str,
        tile_matrix: &str,
        tile_col: u32,
        tile_row: u32,
        format: &str,
        data: Vec<u8>,
    ) {
        let mut tiles = self.tiles.lock().unwrap_or_else(|p| p.into_inner());
        if tiles.len() >= self.max_entries {
            tiles.clear();
        }
        let key = TileKey {
            layer: layer.to_string(),
            tile_matrix_set: tile_matrix_set.to_string(),
            tile_matrix: tile_matrix.to_string(),
            tile_col,
            tile_row,
            format: format.to_string(),
        };
        tiles.insert(key, data);
    }

    /// Returns the number of cached tiles.
    pub fn len(&self) -> usize {
        self.tiles
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .len()
    }

    /// Returns true if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.tiles
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .is_empty()
    }

    /// Clear all cached tiles.
    pub fn clear(&self) {
        self.tiles
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clear();
    }

    /// Pre-cache tiles for a layer over zoom levels 0-4 for a given format.
    pub fn pre_cache(
        &self,
        layer: &str,
        tile_matrix_set: &str,
        format: &str,
        matrix_width: u32,
        matrix_height: u32,
    ) -> usize {
        let mut count = 0;
        let tile_size = 256 * 256 * 4;
        let mut tiles = self.tiles.lock().unwrap_or_else(|p| p.into_inner());
        for zm in 0..5u32 {
            let scale = 2u32.pow(zm);
            let w = (matrix_width * scale).min(32);
            let h = (matrix_height * scale).min(32);
            for col in 0..w {
                for row in 0..h {
                    let key = TileKey {
                        layer: layer.to_string(),
                        tile_matrix_set: tile_matrix_set.to_string(),
                        tile_matrix: zm.to_string(),
                        tile_col: col,
                        tile_row: row,
                        format: format.to_string(),
                    };
                    if let std::collections::hash_map::Entry::Vacant(e) = tiles.entry(key) {
                        let mut data = vec![0u8; tile_size];
                        for yy in 0..256 {
                            for xx in 0..256 {
                                let idx = (yy * 256 + xx) * 4;
                                data[idx] = ((col * 16 + xx as u32) % 256) as u8;
                                data[idx + 1] = ((row * 16 + yy as u32) % 256) as u8;
                                data[idx + 2] = (zm * 40) as u8;
                                data[idx + 3] = 255;
                            }
                        }
                        e.insert(data);
                        count += 1;
                    }
                }
            }
        }
        count
    }
}

impl Default for TileCache {
    fn default() -> Self {
        Self::new(10_000)
    }
}
