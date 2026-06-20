//! Pure-Rust H3 (Hexagonal Hierarchical Spatial Index) approximation.
//!
//! Provides lat/lon ↔ H3 index conversion, hexagon boundaries,
//! neighbor traversal, grid disk (k-ring), and bounding box coverage.
//!
//! **Important**: This is an **approximate** H3 implementation using
//! axial hex coordinates on a flat normalized [0,1]² space. It does
//! NOT use the official Uber H3 library or its C bindings, so results
//! will differ slightly from the canonical H3 library, especially near
//! the poles and at pentagon locations. For most spatial indexing use
//! cases (grid aggregation, spatial joins, coarse tiling) the accuracy
//! is sufficient (±10 km at resolution 5).
//!
//! ## Coordinate System
//!
//! - Pointy-top hexagons (vertex pointing up)
//! - Axial coordinate system (q, r) where the third cube coordinate s = -q - r
//! - World normalized to (lon∈[-180,180], lat∈[-90,90]) → pixel (nx∈[0,1], ny∈[0,1])
//! - Hex size at each resolution computed from the known number of hexagons

use geo_core::types::BBox;

// ── H3 Constants (from official H3 specification) ──

/// Maximum H3 resolution.
pub const H3_MAX_RESOLUTION: u8 = 15;

/// Number of base cells at resolution 0.
pub const H3_N_BASE_CELLS: usize = 122;

/// Average hexagon area (km²) at each resolution 0..=15.
const H3_AREA_KM2: [f64; 16] = [
    4_250_546.847, // 0
    607_220.979,   // 1
    86_745.854,    // 2
    12_392.264,    // 3
    1_770.324,     // 4
    252.903,       // 5
    36.129,        // 6
    5.161,         // 7
    0.737,         // 8
    0.105,         // 9
    0.0150,        // 10
    0.00215,       // 11
    0.000307,      // 12
    0.0000439,     // 13
    0.00000627,    // 14
    0.000000896,   // 15
];

/// Average hexagon edge length (km) at each resolution 0..=15.
const H3_EDGE_KM: [f64; 16] = [
    1_107.713, // 0
    418.676,   // 1
    158.244,   // 2
    59.810,    // 3
    22.606,    // 4
    8.544,     // 5
    3.229,     // 6
    1.221,     // 7
    0.461,     // 8
    0.174,     // 9
    0.0658,    // 10
    0.0249,    // 11
    0.0094,    // 12
    0.00355,   // 13
    0.00134,   // 14
    0.000507,  // 15
];

/// Total number of hexagons at each resolution 0..=15.
const H3_NUM_HEXES: [u64; 16] = [
    122,                 // 0
    842,                 // 1
    5_882,               // 2
    41_162,              // 3
    288_122,             // 4
    2_016_842,           // 5
    14_117_882,          // 6
    98_825_162,          // 7
    691_776_122,         // 8
    4_842_433_802,       // 9
    33_897_036_602,      // 10
    237_279_256_202,     // 11
    1_660_954_793_402,   // 12
    11_626_683_553_802,  // 13
    81_386_784_876_602,  // 14
    569_707_494_136_202, // 15
];

/// Square root of 3, used in hex math.
const SQRT3: f64 = 1.732_050_807_568_877_2;

/// 2/3 constant for hex transforms.
const TWO_THIRDS: f64 = 0.666_666_666_666_666_6;

/// 1/3 constant.
const ONE_THIRD: f64 = 0.333_333_333_333_333_3;

// ── Axial neighbour offsets (pointy-top hexagons) ──

/// The 6 neighbour directions in axial (q, r) coordinates.
const AXIAL_DIRECTIONS: [(i64, i64); 6] = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)];

// ── Helper functions ──

/// Validate resolution and panic-safe fallback.
fn check_res(resolution: u8) -> Option<u8> {
    if resolution <= H3_MAX_RESOLUTION {
        Some(resolution)
    } else {
        None
    }
}

/// Compute hexagon grid size in normalized [0,1] space at given resolution.
fn hex_size(resolution: u8) -> f64 {
    let n = H3_NUM_HEXES[resolution as usize] as f64;
    let hexes_per_side = n.sqrt();
    // Choose size so that ~hexes_per_side hexagons span [0,1]
    1.0 / (1.5 * hexes_per_side)
}

/// Convert pixel (nx, ny) in [0,1]² to axial (q, r) with given hex size.
fn pixel_to_axial(nx: f64, ny: f64, size: f64) -> (f64, f64) {
    // Pointy-top hex conversion
    let q = (SQRT3 / 3.0 * nx - ONE_THIRD * ny) / size;
    let r = (TWO_THIRDS * ny) / size;
    (q, r)
}

/// Convert axial (q, r) to pixel (nx, ny) with given hex size.
fn axial_to_pixel(q: f64, r: f64, size: f64) -> (f64, f64) {
    let nx = size * (SQRT3 * q + SQRT3 / 2.0 * r);
    let ny = size * (1.5 * r);
    (nx, ny)
}

/// Cube-round: round floating axial coords to the nearest valid hex center.
fn cube_round(q: f64, r: f64) -> (i64, i64) {
    let s = -q - r;
    let mut qi = q.round();
    let mut ri = r.round();
    let si = s.round();

    let q_diff = (qi - q).abs();
    let r_diff = (ri - r).abs();
    let s_diff = (si - s).abs();

    if q_diff > r_diff && q_diff > s_diff {
        qi = -ri - si;
    } else if r_diff > s_diff {
        ri = -qi - si;
    }
    // Otherwise si is adjusted (qi, ri stay as-is)

    (qi as i64, ri as i64)
}

/// Normalize lat/lon to [0, 1] pixel coords.
fn latlon_to_pixel(lat: f64, lon: f64) -> (f64, f64) {
    let nx = (lon + 180.0) / 360.0;
    let ny = (lat + 90.0) / 180.0;
    (nx, ny)
}

/// Convert [0, 1] pixel coords back to lat/lon.
fn pixel_to_latlon(nx: f64, ny: f64) -> (f64, f64) {
    let lon = nx * 360.0 - 180.0;
    let lat = ny * 180.0 - 90.0;
    (lat, lon)
}

// ── Public API ──

/// Average hexagon area in km² at given resolution (0–15).
pub fn h3_hex_area_km2(resolution: u8) -> Option<f64> {
    let r = check_res(resolution)?;
    Some(H3_AREA_KM2[r as usize])
}

/// Average hexagon edge length in km at given resolution (0–15).
pub fn h3_edge_length_km(resolution: u8) -> Option<f64> {
    let r = check_res(resolution)?;
    Some(H3_EDGE_KM[r as usize])
}

/// Total number of hexagons covering the globe at given resolution (0–15).
pub fn h3_num_hexagons(resolution: u8) -> Option<u64> {
    let r = check_res(resolution)?;
    Some(H3_NUM_HEXES[r as usize])
}

/// A single H3 hexagon index using axial coordinates.
///
/// ## Note
///
/// This stores (resolution, i, j) where i = axial q, j = axial r.
/// The third cube coordinate is implicitly s = -i - j.
/// These are approximate H3 indices — they do NOT correspond
/// to canonical H3 index integers from the official library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct H3Index {
    /// Resolution 0–15.
    pub resolution: u8,
    /// Axial coordinate q.
    pub i: i64,
    /// Axial coordinate r.
    pub j: i64,
}

impl H3Index {
    /// Convert this H3 index to approximate (lat, lon) of its center.
    ///
    /// Accuracy degrades at higher latitudes due to the flat projection.
    pub fn to_latlon(&self) -> (f64, f64) {
        let size = hex_size(self.resolution);
        let (nx, ny) = axial_to_pixel(self.i as f64, self.j as f64, size);
        pixel_to_latlon(nx, ny)
    }

    /// Compute the 6 boundary vertices of this hexagon as (lon, lat).
    ///
    /// Returns vertices in clockwise order. The 6th vertex connects back
    /// to the first to form a closed polygon.
    /// Vertices are clamped to valid lat [-90, 90] / lon [-180, 180] ranges.
    pub fn to_boundary(&self) -> Vec<(f64, f64)> {
        let size = hex_size(self.resolution);
        let (cx, cy) = axial_to_pixel(self.i as f64, self.j as f64, size);

        (0..6)
            .map(|k| {
                let angle_deg = 60.0 * k as f64 - 30.0;
                let angle_rad = angle_deg.to_radians();
                let vx = cx + size * angle_rad.cos();
                let vy = cy + size * angle_rad.sin();
                let (mut lat, mut lon) = pixel_to_latlon(vx, vy);
                lat = lat.clamp(-90.0, 90.0);
                lon = lon.clamp(-180.0, 180.0);
                (lon, lat)
            })
            .collect()
    }

    /// Get the 6 neighbouring hexagons at the same resolution.
    pub fn neighbors(&self) -> Vec<H3Index> {
        AXIAL_DIRECTIONS
            .iter()
            .map(|(dq, dr)| H3Index {
                resolution: self.resolution,
                i: self.i + dq,
                j: self.j + dr,
            })
            .collect()
    }
}

/// Convert lat/lon to the nearest H3 hexagon at given resolution.
///
/// This is an **approximation** using a flat normalized space.
/// Expected accuracy: < 10 km at resolution 5, better at finer resolutions.
pub fn latlon_to_h3(lat: f64, lon: f64, resolution: u8) -> Option<H3Index> {
    check_res(resolution)?;
    let size = hex_size(resolution);
    let (nx, ny) = latlon_to_pixel(lat, lon);
    let (q, r) = pixel_to_axial(nx, ny, size);
    let (i, j) = cube_round(q, r);
    Some(H3Index { resolution, i, j })
}

/// Convert an H3 index to a GeoJSON Polygon geometry.
///
/// The polygon has 7 coordinate pairs (6 vertices + closing). Coordinates
/// are in (lon, lat) order per GeoJSON spec.
pub fn h3_to_geojson(index: &H3Index) -> serde_json::Value {
    let boundary = index.to_boundary();
    // Build the ring: vertices in clockwise order + closing vertex
    let mut ring: Vec<Vec<f64>> = boundary.iter().map(|(lon, lat)| vec![*lon, *lat]).collect();
    // Close the ring
    if let Some(first) = boundary.first() {
        ring.push(vec![first.0, first.1]);
    }

    serde_json::json!({
        "type": "Polygon",
        "coordinates": [ring]
    })
}

/// Format an H3 index as a string (e.g. `"h3_8_12a4b"`).
///
/// Format: `h3_{resolution}_{hex_i}_{hex_j}` where hex values are
/// base-36 encoded for compactness.
pub fn h3_to_string(index: &H3Index) -> String {
    format!(
        "h3_{}_{}_{}",
        index.resolution,
        encode_i64(index.i),
        encode_i64(index.j)
    )
}

/// Parse an H3 string back to an `H3Index`.
///
/// Accepts the format produced by `h3_to_string()`.
pub fn h3_from_string(s: &str) -> Option<H3Index> {
    let parts: Vec<&str> = s.split('_').collect();
    if parts.len() != 4 || parts[0] != "h3" {
        return None;
    }
    let resolution: u8 = parts[1].parse().ok()?;
    check_res(resolution)?;
    let i = decode_i64(parts[2])?;
    let j = decode_i64(parts[3])?;
    Some(H3Index { resolution, i, j })
}

/// Encode an i64 as a base-36 string (with sign).
fn encode_i64(v: i64) -> String {
    if v == 0 {
        return "0".to_string();
    }
    let negative = v < 0;
    let mut n = v.unsigned_abs();
    let mut digits = Vec::new();
    while n > 0 {
        let d = (n % 36) as u8;
        let c = if d < 10 { b'0' + d } else { b'a' + (d - 10) };
        digits.push(c);
        n /= 36;
    }
    digits.reverse();
    let s = String::from_utf8(digits).unwrap();
    if negative {
        format!("-{s}")
    } else {
        s
    }
}

/// Decode a base-36 string to i64.
fn decode_i64(s: &str) -> Option<i64> {
    if s.is_empty() {
        return None;
    }
    let (sign, digits) = if s.starts_with('-') {
        (-1_i64, &s[1..])
    } else if s.starts_with('+') {
        (1, &s[1..])
    } else {
        (1, s)
    };
    if digits.is_empty() {
        return None;
    }
    let mut result: i64 = 0;
    for c in digits.chars() {
        let d = match c {
            '0'..='9' => (c as u8 - b'0') as i64,
            'a'..='z' => (c as u8 - b'a' + 10) as i64,
            'A'..='Z' => (c as u8 - b'A' + 10) as i64,
            _ => return None,
        };
        result = result.checked_mul(36)?.checked_add(d)?;
    }
    Some(sign * result)
}

/// Perform a ring traversal: all hexagons within `radius_hex` steps
/// of the center hex (k-ring / grid disk).
///
/// `radius_km` is converted to hex-distance units using the edge length
/// at the given resolution. The center is a lat/lon point.
///
/// Returns all hexagon indices in the disk, including the center.
pub fn h3_grid_disk(
    center_lat: f64,
    center_lon: f64,
    radius_km: f64,
    resolution: u8,
) -> Vec<H3Index> {
    let center = match latlon_to_h3(center_lat, center_lon, resolution) {
        Some(c) => c,
        None => return Vec::new(),
    };

    let edge_km = match h3_edge_length_km(resolution) {
        Some(e) => e,
        None => return Vec::new(),
    };

    let max_steps = if radius_km < edge_km * 0.001 {
        0
    } else {
        (radius_km / edge_km).ceil() as u32
    };

    // BFS outwards from center. Only expand neighbors for steps < max_steps.
    let mut visited = std::collections::HashSet::new();
    let mut result = Vec::new();
    let mut frontier = Vec::new();
    frontier.push(center);
    visited.insert((center.i, center.j));

    for step in 0..=max_steps {
        let mut next_frontier = Vec::new();
        for hex in &frontier {
            result.push(*hex);
            if step < max_steps {
                for neighbor in hex.neighbors() {
                    if visited.insert((neighbor.i, neighbor.j)) {
                        next_frontier.push(neighbor);
                    }
                }
            }
        }
        frontier = next_frontier;
    }

    result
}

/// Cover a bounding box with H3 hexagons at the given resolution.
///
/// Iterates the axial bounding box that minimally covers the lat/lon bounding
/// box and returns hexagons whose center falls within the BBox.
pub fn h3_cover_bbox(bbox: &BBox, resolution: u8) -> Vec<H3Index> {
    // Sample enough points along the bbox edges to capture all intersecting hexagons
    let mut candidates = std::collections::HashSet::new();

    // Add hexes at the 4 corners
    let corners = [
        (bbox.min_y, bbox.min_x),
        (bbox.min_y, bbox.max_x),
        (bbox.max_y, bbox.min_x),
        (bbox.max_y, bbox.max_x),
    ];

    for (lat, lon) in &corners {
        if let Some(h3) = latlon_to_h3(*lat, *lon, resolution) {
            candidates.insert((h3.i, h3.j));
        }
    }

    // If corners are close and few unique hexes, also sample edges
    let edge_samples = 10;
    for i in 0..=edge_samples {
        let t = i as f64 / edge_samples as f64;
        // Bottom edge
        let lon = bbox.min_x + t * bbox.width();
        if let Some(h3) = latlon_to_h3(bbox.min_y, lon, resolution) {
            candidates.insert((h3.i, h3.j));
        }
        // Top edge
        if let Some(h3) = latlon_to_h3(bbox.max_y, lon, resolution) {
            candidates.insert((h3.i, h3.j));
        }
        // Left edge
        let lat = bbox.min_y + t * bbox.height();
        if let Some(h3) = latlon_to_h3(lat, bbox.min_x, resolution) {
            candidates.insert((h3.i, h3.j));
        }
        // Right edge
        if let Some(h3) = latlon_to_h3(lat, bbox.max_x, resolution) {
            candidates.insert((h3.i, h3.j));
        }
    }

    // Compute axial bounding box
    let mut i_min = i64::MAX;
    let mut i_max = i64::MIN;
    let mut j_min = i64::MAX;
    let mut j_max = i64::MIN;

    for (i, j) in &candidates {
        i_min = i_min.min(*i);
        i_max = i_max.max(*i);
        j_min = j_min.min(*j);
        j_max = j_max.max(*j);
    }

    if i_min == i64::MAX {
        return Vec::new();
    }

    // Fill the axial bounding box, filtering by actual bbox containment
    let mut result = Vec::new();
    for i in i_min..=i_max {
        for j in j_min..=j_max {
            let idx = H3Index { resolution, i, j };
            let (lat, lon) = idx.to_latlon();
            if bbox.contains(lon, lat) {
                result.push(idx);
            }
        }
    }

    result
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_area_monotonic() {
        for r in 0..15 {
            let a0 = h3_hex_area_km2(r).unwrap();
            let a1 = h3_hex_area_km2(r + 1).unwrap();
            assert!(
                a1 < a0,
                "Area at res {} ({}) should be > res {} ({})",
                r,
                a0,
                r + 1,
                a1
            );
        }
    }

    #[test]
    fn test_edge_length_monotonic() {
        for r in 0..15 {
            let e0 = h3_edge_length_km(r).unwrap();
            let e1 = h3_edge_length_km(r + 1).unwrap();
            assert!(e1 < e0, "Edge at res {} > res {}", r, r + 1);
        }
    }

    #[test]
    fn test_num_hexagons_monotonic() {
        for r in 0..15 {
            let n0 = h3_num_hexagons(r).unwrap();
            let n1 = h3_num_hexagons(r + 1).unwrap();
            assert!(n1 > n0, "Num hexes at res {} <= res {}", r, r + 1);
        }
    }

    #[test]
    fn test_invalid_resolution() {
        assert!(h3_hex_area_km2(16).is_none());
        assert!(h3_edge_length_km(16).is_none());
        assert!(h3_num_hexagons(16).is_none());
        assert!(latlon_to_h3(0.0, 0.0, 16).is_none());
    }

    #[test]
    fn test_latlon_roundtrip_res5() {
        let (lat, lon) = (39.9, 116.4); // Beijing
        let h3 = latlon_to_h3(lat, lon, 5).unwrap();
        let (rlat, rlon) = h3.to_latlon();
        let dx = (rlon - lon).abs() * 111_320.0 * (lat.to_radians()).cos();
        let dy = (rlat - lat).abs() * 111_320.0;
        let dist_km = ((dx * dx + dy * dy).sqrt()) / 1000.0;
        // At res 5, should be within ~15 km
        assert!(
            dist_km < 15.0,
            "Roundtrip error {:.1} km at res 5 (expected < 15 km)",
            dist_km
        );
    }

    #[test]
    fn test_latlon_roundtrip_res8() {
        let (lat, lon) = (48.8566, 2.3522); // Paris
        let h3 = latlon_to_h3(lat, lon, 8).unwrap();
        let (rlat, rlon) = h3.to_latlon();
        let dx = (rlon - lon).abs() * 111_320.0 * (lat.to_radians()).cos();
        let dy = (rlat - lat).abs() * 111_320.0;
        let dist_km = ((dx * dx + dy * dy).sqrt()) / 1000.0;
        // At res 8, should be within ~2 km
        assert!(
            dist_km < 2.0,
            "Roundtrip error {:.1} km at res 8 (expected < 2 km)",
            dist_km
        );
    }

    #[test]
    fn test_neighbors_unique() {
        let h3 = latlon_to_h3(0.0, 0.0, 4).unwrap();
        let neighbors = h3.neighbors();
        assert_eq!(neighbors.len(), 6, "Expected 6 neighbors");

        // All should be unique
        let mut unique = std::collections::HashSet::new();
        for n in &neighbors {
            assert!(
                unique.insert((n.i, n.j)),
                "Duplicate neighbor: ({}, {})",
                n.i,
                n.j
            );
        }

        // All should be at same resolution
        for n in &neighbors {
            assert_eq!(n.resolution, h3.resolution);
        }
    }

    #[test]
    fn test_neighbors_reversible() {
        let center = latlon_to_h3(0.0, 0.0, 4).unwrap();
        let neighbors = center.neighbors();
        // Each neighbor should list back to center
        for n in &neighbors {
            let nn = n.neighbors();
            let has_center = nn.iter().any(|n2| n2.i == center.i && n2.j == center.j);
            assert!(
                has_center,
                "Neighbor ({}, {}) doesn't point back to center",
                n.i, n.j
            );
        }
    }

    #[test]
    fn test_grid_disk_count() {
        let hexes = h3_grid_disk(39.9, 116.4, 100.0, 4);
        // At res 4, edge ~22 km. Radius 100 km → ~5 steps.
        // Expected: roughly 1 + 6 + 12 + 18 + 24 + 30 = ~91 hexes
        assert!(
            hexes.len() >= 10,
            "Grid disk too small: {} (expected >= 10)",
            hexes.len()
        );
        assert!(
            hexes.len() <= 500,
            "Grid disk too large: {} (expected <= 500)",
            hexes.len()
        );

        // Center hex should be in the result
        let center = latlon_to_h3(39.9, 116.4, 4).unwrap();
        assert!(
            hexes.iter().any(|h| h.i == center.i && h.j == center.j),
            "Center hex not in grid disk"
        );
    }

    #[test]
    fn test_grid_disk_zero_radius() {
        let hexes = h3_grid_disk(0.0, 0.0, 0.001, 4);
        assert_eq!(hexes.len(), 1, "Zero radius should give exactly 1 hex");
    }

    #[test]
    fn test_cover_bbox_china() {
        let bbox = BBox::new(73.5, 3.8, 135.0, 53.6); // China approximate
        let hexes = h3_cover_bbox(&bbox, 3);
        // Res 3 has ~41k hexes globally; China is a big country
        assert!(
            hexes.len() >= 10,
            "China cover at res 3 too small: {}",
            hexes.len()
        );
        assert!(
            hexes.len() <= 5000,
            "China cover at res 3 too large: {}",
            hexes.len()
        );
    }

    #[test]
    fn test_cover_bbox_small() {
        // A small bbox (1 km²) at high resolution
        let bbox = BBox::new(116.38, 39.90, 116.39, 39.91);
        let hexes = h3_cover_bbox(&bbox, 10);
        assert!(
            hexes.len() >= 1,
            "Small bbox should cover at least 1 hex at res 10"
        );
    }

    #[test]
    fn test_to_geojson() {
        let h3 = latlon_to_h3(0.0, 0.0, 4).unwrap();
        let gj = h3_to_geojson(&h3);
        assert_eq!(gj["type"], "Polygon");

        let coords = gj["coordinates"][0].as_array().unwrap();
        assert_eq!(coords.len(), 7, "Polygon should have 7 coords (6+closing)");

        // First and last should be same
        let first = coords[0].as_array().unwrap();
        let last = coords[6].as_array().unwrap();
        assert!(
            (first[0].as_f64().unwrap() - last[0].as_f64().unwrap()).abs() < 1e-10,
            "Polygon not closed"
        );
        assert!(
            (first[1].as_f64().unwrap() - last[1].as_f64().unwrap()).abs() < 1e-10,
            "Polygon not closed"
        );
    }

    #[test]
    fn test_to_string_roundtrip() {
        let h3 = latlon_to_h3(0.0, 0.0, 7).unwrap();
        let s = h3_to_string(&h3);
        let parsed = h3_from_string(&s).unwrap();
        assert_eq!(
            h3, parsed,
            "String roundtrip failed: '{}' → ({}, {}) vs ({}, {})",
            s, parsed.i, parsed.j, h3.i, h3.j
        );
    }

    #[test]
    fn test_to_string_negative_coords() {
        let h3 = H3Index {
            resolution: 5,
            i: -1234,
            j: 56789,
        };
        let s = h3_to_string(&h3);
        let parsed = h3_from_string(&s).unwrap();
        assert_eq!(h3, parsed, "Negative coord roundtrip failed: '{}'", s);
    }

    #[test]
    fn test_encode_decode_zero() {
        assert_eq!(encode_i64(0), "0");
        let s = encode_i64(-0);
        // -0 should also be "0"
        assert!(s == "0" || s.chars().next() == Some('0'));
    }

    #[test]
    fn test_equator_at_res0() {
        // At res 0, there are 122 base hexagons covering the world.
        // Equator at lon=0 should map to some valid hex.
        let h3 = latlon_to_h3(0.0, 0.0, 0).unwrap();
        let (lat, lon) = h3.to_latlon();
        // Should be roughly near (0, 0)
        assert!(lat.abs() < 30.0, "Equator hex center too far: lat={}", lat);
        assert!(lon.abs() < 30.0, "Equator hex center too far: lon={}", lon);
    }

    #[test]
    fn test_boundary_within_world() {
        let h3 = latlon_to_h3(40.0, -100.0, 6).unwrap();
        let boundary = h3.to_boundary();
        for (lon, lat) in &boundary {
            assert!(
                (-180.0..=180.0).contains(lon),
                "Boundary lon {} out of range",
                lon
            );
            assert!(
                (-90.0..=90.0).contains(lat),
                "Boundary lat {} out of range",
                lat
            );
        }
    }

    #[test]
    fn test_cover_bbox_covers_center() {
        // Verify that the cover function includes the bbox's own center
        let bbox = BBox::new(100.0, 10.0, 110.0, 20.0);
        let hexes = h3_cover_bbox(&bbox, 5);
        assert!(!hexes.is_empty(), "BBox cover should not be empty");

        // The center of the bbox should definitely be covered
        let center_h3 = latlon_to_h3(15.0, 105.0, 5).unwrap();
        let found = hexes
            .iter()
            .any(|h| h.i == center_h3.i && h.j == center_h3.j);
        assert!(
            found,
            "Center hex ({}, {}) not in cover",
            center_h3.i, center_h3.j
        );

        // All returned hexes should have centers within the bbox
        for h in &hexes {
            let (lat, lon) = h.to_latlon();
            assert!(
                bbox.contains(lon, lat),
                "Hex ({}, {}) center ({}, {}) outside bbox",
                h.i,
                h.j,
                lat,
                lon
            );
        }
    }
}
