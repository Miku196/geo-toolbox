//! Geohash encoding/decoding.

use geo_index::geohash::{decode, encode, neighbors};
use pyo3::prelude::*;

/// Encode WGS84 lat/lon to a geohash string.
pub fn geohash_encode_impl(lat: f64, lon: f64, precision: usize) -> PyResult<String> {
    if precision == 0 || precision > 12 {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "precision must be 1-12",
        ));
    }
    Ok(encode(lon, lat, precision))
}

/// Decode a geohash string to its bounding box.
/// Returns (lat_min, lat_max, lon_min, lon_max).
pub fn geohash_decode_impl(hash: &str) -> PyResult<(f64, f64, f64, f64)> {
    let (_center_lon, _center_lat, bbox) = decode(hash).ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid geohash: {hash}"))
    })?;
    Ok((bbox.min_y, bbox.max_y, bbox.min_x, bbox.max_x))
}

/// Get the 8 neighboring geohash cells for a given hash.
pub fn geohash_neighbors_impl(hash: &str) -> PyResult<Vec<String>> {
    let nb = neighbors(hash);
    if nb.is_empty() {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Invalid geohash: {hash}"
        )));
    }
    Ok(nb)
}
