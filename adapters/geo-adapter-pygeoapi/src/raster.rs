//! numpy ndarray ↔ geo-raster Band conversion.
//!
//! Uses flat byte slices with shape/dtype metadata for zero-copy transfer.
//! On the Python side, `numpy.frombuffer(bytes, dtype=dtype).reshape(shape)`
//! reconstructs the array without copying data.

use geo_core::errors::GeoResult;

/// Supported numpy dtypes for raster data interchange.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumpyDtype {
    Float32,
    Float64,
    Int16,
    Int32,
    UInt8,
    UInt16,
}

impl NumpyDtype {
    /// Size in bytes per element.
    pub fn element_size(&self) -> usize {
        match self {
            NumpyDtype::Float32 => 4,
            NumpyDtype::Float64 => 8,
            NumpyDtype::Int16 => 2,
            NumpyDtype::Int32 => 4,
            NumpyDtype::UInt8 => 1,
            NumpyDtype::UInt16 => 2,
        }
    }

    pub fn from_str(s: &str) -> GeoResult<Self> {
        match s {
            "float32" | "f4" => Ok(NumpyDtype::Float32),
            "float64" | "f8" => Ok(NumpyDtype::Float64),
            "int16" | "i2" => Ok(NumpyDtype::Int16),
            "int32" | "i4" => Ok(NumpyDtype::Int32),
            "uint8" | "u1" => Ok(NumpyDtype::UInt8),
            "uint16" | "u2" => Ok(NumpyDtype::UInt16),
            other => Err(geo_core::GeoError::Validation(format!(
                "Unsupported numpy dtype: {other}"
            ))),
        }
    }
}

/// Convert a numpy flat byte array into an f64 raster band.
///
/// `flat_data` — raw bytes from `ndarray.tobytes()`
/// `rows`, `cols` — spatial dimensions
/// `bands` — number of bands (1 for single-band, >1 for multi-band)
/// `dtype` — numpy dtype string (e.g. "float32", "float64", "int16")
///
/// Returns a flat `Vec<f64>` suitable for use with geo-raster Band types.
/// Band-interleaved pixel order: [b0_r0c0, ..., b0_r0cN, b0_r1c0, ...,
/// b1_r0c0, ...] for multi-band.
pub fn numpy_to_geo_raster(
    flat_data: &[u8],
    rows: usize,
    cols: usize,
    bands: usize,
    dtype: &str,
) -> GeoResult<Vec<f64>> {
    let np_dtype = NumpyDtype::from_str(dtype)?;
    let elem_size = np_dtype.element_size();
    let expected_len = rows * cols * bands * elem_size;

    if flat_data.len() < expected_len {
        return Err(geo_core::GeoError::Validation(format!(
            "Data too short: expected {expected_len} bytes for {rows}×{cols}×{bands}×{elem_size}, got {}",
            flat_data.len()
        )));
    }

    let total_pixels = rows * cols * bands;
    let mut result = Vec::with_capacity(total_pixels);

    match np_dtype {
        NumpyDtype::Float64 => {
            for i in 0..total_pixels {
                let start = i * 8;
                let val = f64::from_le_bytes([
                    flat_data[start],
                    flat_data[start + 1],
                    flat_data[start + 2],
                    flat_data[start + 3],
                    flat_data[start + 4],
                    flat_data[start + 5],
                    flat_data[start + 6],
                    flat_data[start + 7],
                ]);
                result.push(val);
            }
        }
        NumpyDtype::Float32 => {
            for i in 0..total_pixels {
                let start = i * 4;
                let val = f32::from_le_bytes([
                    flat_data[start],
                    flat_data[start + 1],
                    flat_data[start + 2],
                    flat_data[start + 3],
                ]) as f64;
                result.push(val);
            }
        }
        NumpyDtype::Int16 => {
            for i in 0..total_pixels {
                let start = i * 2;
                let val = i16::from_le_bytes([flat_data[start], flat_data[start + 1]]) as f64;
                result.push(val);
            }
        }
        NumpyDtype::Int32 => {
            for i in 0..total_pixels {
                let start = i * 4;
                let val = i32::from_le_bytes([
                    flat_data[start],
                    flat_data[start + 1],
                    flat_data[start + 2],
                    flat_data[start + 3],
                ]) as f64;
                result.push(val);
            }
        }
        NumpyDtype::UInt8 => {
            result = flat_data[..total_pixels]
                .iter()
                .map(|&b| b as f64)
                .collect();
        }
        NumpyDtype::UInt16 => {
            for i in 0..total_pixels {
                let start = i * 2;
                let val = u16::from_le_bytes([flat_data[start], flat_data[start + 1]]) as f64;
                result.push(val);
            }
        }
    }

    Ok(result)
}

/// Convert an f64 raster band into a numpy-compatible f64 flat buffer.
///
/// Returns f64 values in row-major order that can be loaded as
/// `np.array(data, dtype=np.float64).reshape((rows, cols))` on the Python side.
pub fn geo_raster_to_numpy(band_data: &[f64], rows: usize, cols: usize) -> Vec<f64> {
    let expected_len = rows * cols;
    if band_data.len() >= expected_len {
        band_data[..expected_len].to_vec()
    } else {
        let mut padded = band_data.to_vec();
        padded.resize(expected_len, 0.0);
        padded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dtype_from_str() {
        assert_eq!(
            NumpyDtype::from_str("float32").unwrap(),
            NumpyDtype::Float32
        );
        assert_eq!(NumpyDtype::from_str("f8").unwrap(), NumpyDtype::Float64);
        assert_eq!(NumpyDtype::from_str("uint8").unwrap(), NumpyDtype::UInt8);
        assert!(NumpyDtype::from_str("complex64").is_err());
    }

    #[test]
    fn test_numpy_to_geo_raster_f64_roundtrip() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // 2×3
        let flat: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        let result = numpy_to_geo_raster(&flat, 2, 3, 1, "float64").unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_numpy_to_geo_raster_f32() {
        let original: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];
        let flat: Vec<u8> = original.iter().flat_map(|v| v.to_le_bytes()).collect();
        let result = numpy_to_geo_raster(&flat, 2, 2, 1, "float32").unwrap();
        assert_eq!(result, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_numpy_to_geo_raster_uint8() {
        let data: Vec<u8> = vec![0, 128, 255, 64];
        let result = numpy_to_geo_raster(&data, 2, 2, 1, "uint8").unwrap();
        assert_eq!(result, vec![0.0, 128.0, 255.0, 64.0]);
    }

    #[test]
    fn test_numpy_to_geo_raster_multi_band() {
        // 2 bands, 2×2 → 8 pixels
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let flat: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        let result = numpy_to_geo_raster(&flat, 2, 2, 2, "float64").unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_numpy_to_geo_raster_too_short() {
        let short = vec![1u8, 2, 3]; // 3 bytes for 100 expected
        let err = numpy_to_geo_raster(&short, 5, 5, 1, "float64");
        assert!(err.is_err());
    }

    #[test]
    fn test_geo_raster_to_numpy() {
        let band = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let result = geo_raster_to_numpy(&band, 2, 3);
        assert_eq!(result, band);
    }

    #[test]
    fn test_geo_raster_to_numpy_padding() {
        let band = vec![1.0, 2.0]; // 2 elements, but 2×3=6 expected
        let result = geo_raster_to_numpy(&band, 2, 3);
        assert_eq!(result.len(), 6);
        assert_eq!(result[..2], [1.0, 2.0]);
        assert_eq!(result[2..], [0.0, 0.0, 0.0, 0.0]);
    }
}
