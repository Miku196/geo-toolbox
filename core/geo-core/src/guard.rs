//! Input resource guard — prevents OOM / DoS from oversized requests.
//!
//! Every geo-server / geo-wasm endpoint should validate input through a
//! `ResourceGuard` before entering compute. Rejections return structured
//! errors with clear limits, not generic 500s.
//!
//! # Example
//!
//! ```rust,ignore
//! let guard = ResourceGuard::production();
//! guard.check_geojson_size(geojson_bytes)?;
//! let features = guard.check_feature_count(&parsed_fc)?;
//! guard.check_raster_resolution(cols, rows)?;
//! ```

use crate::errors::{GeoError, GeoResult};

/// Production-grade input limits.
///
/// Tuned to prevent OOM on commodity hardware (4GB RAM).
/// For edge/IoT deployments, create a custom `ResourceGuard`.
#[derive(Debug, Clone)]
pub struct ResourceGuard {
    /// Max raw bytes accepted per request.
    pub max_payload_bytes: u64,
    /// Max GeoJSON features processed per request.
    pub max_feature_count: usize,
    /// Max raster dimension (cols or rows).
    pub max_raster_dim: usize,
    /// Max total raster pixels (cols × rows).
    pub max_raster_pixels: u64,
}

impl Default for ResourceGuard {
    fn default() -> Self {
        Self::production()
    }
}

impl ResourceGuard {
    /// Conservative production defaults (4GB RAM target).
    pub fn production() -> Self {
        Self {
            max_payload_bytes: 50 * 1024 * 1024, // 50 MB
            max_feature_count: 1_000_000,        // 1 million
            max_raster_dim: 10_000,              // 10k × 10k
            max_raster_pixels: 100_000_000,      // 100 million pixels
        }
    }

    /// Tight limits for edge / WASM / IoT deployments.
    pub fn edge() -> Self {
        Self {
            max_payload_bytes: 5 * 1024 * 1024, // 5 MB
            max_feature_count: 10_000,
            max_raster_dim: 2_048,
            max_raster_pixels: 4_194_304, // 2048²
        }
    }

    /// Check raw payload size. Returns `Ok(())` or `Err(GeoError::PayloadTooLarge { ... })`.
    pub fn check_payload_size(&self, bytes: u64) -> GeoResult<()> {
        if bytes > self.max_payload_bytes {
            return Err(GeoError::PayloadTooLarge {
                actual: bytes,
                limit: self.max_payload_bytes,
            });
        }
        Ok(())
    }

    /// Check feature count. Returns `Ok(feature_count)` or `Err(GeoError::TooManyFeatures { ... })`.
    pub fn check_feature_count(&self, count: usize) -> GeoResult<usize> {
        if count > self.max_feature_count {
            return Err(GeoError::TooManyFeatures {
                actual: count,
                limit: self.max_feature_count,
            });
        }
        Ok(count)
    }

    /// Check raster dimensions. Returns `Ok(())` or `Err(GeoError::RasterTooLarge { ... })`.
    pub fn check_raster_dimensions(&self, cols: usize, rows: usize) -> GeoResult<()> {
        if cols > self.max_raster_dim || rows > self.max_raster_dim {
            return Err(GeoError::RasterTooLarge {
                cols,
                rows,
                max_dim: self.max_raster_dim,
            });
        }
        let pixels = cols as u64 * rows as u64;
        if pixels > self.max_raster_pixels {
            return Err(GeoError::RasterTooManyPixels {
                pixels,
                limit: self.max_raster_pixels,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_under_limit() {
        let guard = ResourceGuard::production();
        assert!(guard.check_payload_size(1024).is_ok());
    }

    #[test]
    fn test_payload_over_limit() {
        let guard = ResourceGuard::production();
        assert!(guard.check_payload_size(51 * 1024 * 1024).is_err());
    }

    #[test]
    fn test_feature_count_under() {
        let guard = ResourceGuard::production();
        assert_eq!(guard.check_feature_count(100).unwrap(), 100);
    }

    #[test]
    fn test_feature_count_over() {
        let guard = ResourceGuard::production();
        assert!(guard.check_feature_count(2_000_000).is_err());
    }

    #[test]
    fn test_raster_ok() {
        let guard = ResourceGuard::production();
        assert!(guard.check_raster_dimensions(2048, 2048).is_ok());
    }

    #[test]
    fn test_raster_too_large() {
        let guard = ResourceGuard::production();
        assert!(guard.check_raster_dimensions(20000, 20000).is_err());
    }
}
