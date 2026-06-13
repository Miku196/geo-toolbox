//! GeoHash 编解码（Base32，精度 1-12）。

use geo_core::types::BBox;

const BASE32: &[u8; 32] = b"0123456789bcdefghjkmnpqrstuvwxyz";

/// 编码经纬度为 GeoHash 字符串。
pub fn encode(lon: f64, lat: f64, precision: usize) -> String {
    let precision = precision.clamp(1, 12);
    let mut min_lon = -180.0;
    let mut max_lon = 180.0;
    let mut min_lat = -90.0;
    let mut max_lat = 90.0;
    let mut hash = String::with_capacity(precision);
    let mut bits = 0u8;
    let mut bit_count = 0u8;
    let mut is_lon = true;

    while hash.len() < precision {
        if is_lon {
            let mid = (min_lon + max_lon) / 2.0;
            if lon >= mid {
                bits = (bits << 1) | 1;
                min_lon = mid;
            } else {
                bits <<= 1;
                max_lon = mid;
            }
        } else {
            let mid = (min_lat + max_lat) / 2.0;
            if lat >= mid {
                bits = (bits << 1) | 1;
                min_lat = mid;
            } else {
                bits <<= 1;
                max_lat = mid;
            }
        }
        is_lon = !is_lon;
        bit_count += 1;
        if bit_count == 5 {
            hash.push(BASE32[bits as usize] as char);
            bits = 0;
            bit_count = 0;
        }
    }
    hash
}

/// 解码 GeoHash 为中心点和边界框。
pub fn decode(hash: &str) -> Option<(f64, f64, BBox)> {
    if hash.is_empty() || hash.len() > 12 {
        return None;
    }
    let mut min_lon = -180.0;
    let mut max_lon = 180.0;
    let mut min_lat = -90.0;
    let mut max_lat = 90.0;
    let mut is_lon = true;

    for ch in hash.chars() {
        let val = BASE32.iter().position(|&c| c == ch as u8)? as u8;
        for i in (0..5).rev() {
            let bit = (val >> i) & 1;
            if is_lon {
                let mid = (min_lon + max_lon) / 2.0;
                if bit == 1 {
                    min_lon = mid;
                } else {
                    max_lon = mid;
                }
            } else {
                let mid = (min_lat + max_lat) / 2.0;
                if bit == 1 {
                    min_lat = mid;
                } else {
                    max_lat = mid;
                }
            }
            is_lon = !is_lon;
        }
    }
    let center_lon = (min_lon + max_lon) / 2.0;
    let center_lat = (min_lat + max_lat) / 2.0;
    Some((
        center_lon,
        center_lat,
        BBox::new(min_lon, min_lat, max_lon, max_lat),
    ))
}

/// 计算邻域 8 个 GeoHash。
pub fn neighbors(hash: &str) -> Vec<String> {
    let dirs: [(f64, f64); 8] = [
        (0.0, 1.0),
        (1.0, 0.0),
        (0.0, -1.0),
        (-1.0, 0.0),
        (1.0, 1.0),
        (-1.0, 1.0),
        (1.0, -1.0),
        (-1.0, -1.0),
    ];
    let (lon, lat, _) = match decode(hash) {
        Some(v) => v,
        None => return vec![],
    };
    let precision = hash.len();
    dirs.iter()
        .map(|(dlon, dlat)| {
            let offset_lon = if hash.len().is_multiple_of(2) {
                0.0001
            } else {
                0.001
            };
            let offset_lat = if hash.len().is_multiple_of(2) {
                0.001
            } else {
                0.0001
            };
            encode(lon + dlon * offset_lon, lat + dlat * offset_lat, precision)
        })
        .collect()
}

/// 边界框覆盖为 GeoHash 集合。
pub fn bbox_to_geohashes(bbox: &BBox, precision: usize) -> Vec<String> {
    let mut hashes = Vec::new();
    let step_lon = if precision <= 3 {
        5.0
    } else if precision <= 6 {
        0.5
    } else {
        0.05
    };
    let step_lat = step_lon * 0.5;
    let mut lat = bbox.min_y;
    while lat < bbox.max_y {
        let mut lon = bbox.min_x;
        while lon < bbox.max_x {
            hashes.push(encode(
                lon + step_lon / 2.0,
                lat + step_lat / 2.0,
                precision,
            ));
            lon += step_lon;
        }
        lat += step_lat;
    }
    hashes.sort();
    hashes.dedup();
    hashes
}

/// 边界框包含的 GeoHash 范围。
#[derive(Debug, Clone)]
pub struct GeohashBounds {
    pub center_lon: f64,
    pub center_lat: f64,
    pub bbox: BBox,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let hash = encode(104.0657, 30.5723, 8);
        assert_eq!(hash.len(), 8);

        let (lon, lat, bbox) = decode(&hash).unwrap();
        assert!((lon - 104.0657).abs() < 1.0);
        assert!((lat - 30.5723).abs() < 1.0);
        assert!(bbox.contains(lon, lat));
    }

    #[test]
    fn test_neighbors() {
        let hash = encode(104.0, 30.5, 6);
        let nb = neighbors(&hash);
        assert!(!nb.is_empty());
    }

    #[test]
    fn test_bbox_to_geohashes() {
        let bbox = BBox::new(104.0, 30.5, 104.1, 30.6);
        let hashes = bbox_to_geohashes(&bbox, 5);
        assert!(!hashes.is_empty());
    }
}
