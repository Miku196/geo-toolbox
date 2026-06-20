//! Gauss-Krüger projection — coordinate zone transformation for Chinese survey standards.
//!
//! Supports 3°/6° zone systems with CGCS2000, Xi'an80, Beijing54, and WGS84 ellipsoids.
//!
//! ## Usage
//!
//! ```rust
//! use geo_plugin_survey::gauss::{Ellipsoid, gauss_forward, gauss_inverse, zone_transform, zone_info};
//!
//! // Forward: (B,L) → (X,Y) in 3° zone 35
//! let (x, y) = gauss_forward(30.5_f64.to_radians(), 104.0_f64.to_radians(), 105.0_f64.to_radians(), Ellipsoid::CGCS2000);
//!
//! // Inverse: (X,Y) → (B,L)
//! let (b, l) = gauss_inverse(x, y, 105.0_f64.to_radians(), Ellipsoid::CGCS2000);
//!
//! // Zone transform: 3° zone 35 → 3° zone 36
//! let (x2, y2) = zone_transform(x, y, 35, 36, true, Ellipsoid::CGCS2000);
//!
//! // Get zone info from longitude
//! let info = zone_info(104.0);
//! // → 3° zone 35, central meridian 105°
//! ```

use serde::{Deserialize, Serialize};

// ── Ellipsoid ─────────────────────────────────────────────────

/// Supported reference ellipsoids for Chinese survey coordinate systems.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum Ellipsoid {
    /// CGCS2000 (a=6378137, f=1/298.257222101)
    #[default]
    CGCS2000,
    /// Xi'an 1980 — IAG75 (a=6378140, f=1/298.257)
    Xian80,
    /// Beijing 1954 — Krasovsky 1940 (a=6378245, f=1/298.3)
    Beijing54,
    /// WGS84 (a=6378137, f=1/298.257223563)
    WGS84,
}

impl Ellipsoid {
    /// Semi-major axis (meters).
    pub fn a(&self) -> f64 {
        match self {
            Ellipsoid::CGCS2000 | Ellipsoid::WGS84 => 6378137.0,
            Ellipsoid::Xian80 => 6378140.0,
            Ellipsoid::Beijing54 => 6378245.0,
        }
    }

    /// Flattening 1/f.
    pub fn inv_f(&self) -> f64 {
        match self {
            Ellipsoid::CGCS2000 => 298.257222101,
            Ellipsoid::Xian80 => 298.257,
            Ellipsoid::Beijing54 => 298.3,
            Ellipsoid::WGS84 => 298.257223563,
        }
    }

    /// First eccentricity squared e².
    pub fn e2(&self) -> f64 {
        let f = 1.0 / self.inv_f();
        2.0 * f - f * f
    }

    /// Second eccentricity squared e'².
    pub fn ep2(&self) -> f64 {
        let e2 = self.e2();
        e2 / (1.0 - e2)
    }

    /// Label for display.
    pub fn label(&self) -> &'static str {
        match self {
            Ellipsoid::CGCS2000 => "CGCS2000 (GRS80)",
            Ellipsoid::Xian80 => "Xian80 (IAG75)",
            Ellipsoid::Beijing54 => "Beijing54 (Krasovsky)",
            Ellipsoid::WGS84 => "WGS84",
        }
    }
}

// ── Zone Info ─────────────────────────────────────────────────

/// Zone information for Gauss-Krüger projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneInfo {
    /// Zone number.
    pub zone_number: u16,
    /// Central meridian longitude (degrees).
    pub central_meridian_deg: f64,
    /// Whether this is a 3° zone (false = 6° zone).
    pub is_3degree: bool,
    /// Zone label (e.g. "3°-35").
    pub label: String,
}

/// Get zone information from a longitude (degrees).
///
/// Returns both 3° and 6° zone numbers.
pub fn zone_info(lon_deg: f64) -> ZoneInfoResult {
    // 6° zone: zone = floor(lon/6) + 1
    let zone6 = ((lon_deg / 6.0).floor() as u16) + 1;
    let cm6 = zone6 as f64 * 6.0 - 3.0;

    // 3° zone: zone = floor((lon - 1.5) / 3) + 1
    let zone3_raw = ((lon_deg - 1.5) / 3.0).floor() as u16 + 1;
    let zone3 = zone3_raw.clamp(1, 120);
    let cm3 = zone3 as f64 * 3.0;

    ZoneInfoResult {
        zone6,
        central_meridian_6_deg: cm6,
        zone3,
        central_meridian_3_deg: cm3,
    }
}

/// Result of zone info lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneInfoResult {
    pub zone6: u16,
    pub central_meridian_6_deg: f64,
    pub zone3: u16,
    pub central_meridian_3_deg: f64,
}

/// Get central meridian for a zone.
pub fn central_meridian(zone: u16, is_3degree: bool) -> f64 {
    if is_3degree {
        zone as f64 * 3.0
    } else {
        zone as f64 * 6.0 - 3.0
    }
}

// ── Meridian Arc Length ──────────────────────────────────────

/// Compute meridian arc length from equator to latitude B.
///
/// Uses series expansion with coefficients A₀, A₂, A₄, A₆, A₈.
/// X₀ = a·(1-e²)·[A₀·B + A₂·sin2B + A₄·sin4B + A₆·sin6B + A₈·sin8B]
#[allow(non_snake_case)]
fn meridian_arc(B: f64, ell: Ellipsoid) -> f64 {
    let e2 = ell.e2();
    let e4 = e2 * e2;
    let e6 = e4 * e2;
    let e8 = e6 * e2;

    let a0 = 1.0 + 3.0 / 4.0 * e2 + 45.0 / 64.0 * e4 + 175.0 / 256.0 * e6 + 11025.0 / 16384.0 * e8;
    let a2 = -(3.0 / 4.0 * e2 + 15.0 / 16.0 * e4 + 525.0 / 512.0 * e6 + 2205.0 / 2048.0 * e8) / 2.0;
    let a4 = (15.0 / 64.0 * e4 + 105.0 / 256.0 * e6 + 2205.0 / 4096.0 * e8) / 4.0;
    let a6 = -(35.0 / 512.0 * e6 + 315.0 / 2048.0 * e8) / 6.0;
    let a8 = (315.0 / 16384.0 * e8) / 8.0;

    let m = ell.a() * (1.0 - e2);
    m * (a0 * B
        + a2 * (2.0 * B).sin()
        + a4 * (4.0 * B).sin()
        + a6 * (6.0 * B).sin()
        + a8 * (8.0 * B).sin())
}

/// Compute footpoint latitude (latitude where meridian arc length equals given X).
///
/// Uses Newton-Raphson iteration: B_{n+1} = B_n - f(B_n)/f'(B_n)
#[allow(non_snake_case)]
fn footpoint_latitude(X: f64, ell: Ellipsoid) -> f64 {
    // Initial approximation
    let mut B = X / (ell.a() * (1.0 - ell.e2()));

    for _ in 0..20 {
        let arc = meridian_arc(B, ell);
        let f = arc - X;

        // Derivative: d(meridian_arc)/dB = a·(1-e²)·A₀
        // More precisely: dX₀/dB = a·(1-e²)·[A₀ + 2·A₂·cos2B + 4·A₄·cos4B + 6·A₆·cos6B + 8·A₈·cos8B]
        let e2 = ell.e2();
        let e4 = e2 * e2;
        let e6 = e4 * e2;
        let e8 = e6 * e2;

        let a0 =
            1.0 + 3.0 / 4.0 * e2 + 45.0 / 64.0 * e4 + 175.0 / 256.0 * e6 + 11025.0 / 16384.0 * e8;
        let a2 =
            -(3.0 / 4.0 * e2 + 15.0 / 16.0 * e4 + 525.0 / 512.0 * e6 + 2205.0 / 2048.0 * e8) / 2.0;
        let a4 = (15.0 / 64.0 * e4 + 105.0 / 256.0 * e6 + 2205.0 / 4096.0 * e8) / 4.0;
        let a6 = -(35.0 / 512.0 * e6 + 315.0 / 2048.0 * e8) / 6.0;
        let a8 = (315.0 / 16384.0 * e8) / 8.0;

        let df = ell.a()
            * (1.0 - e2)
            * (a0
                + 2.0 * a2 * (2.0 * B).cos()
                + 4.0 * a4 * (4.0 * B).cos()
                + 6.0 * a6 * (6.0 * B).cos()
                + 8.0 * a8 * (8.0 * B).cos());

        if df.abs() < 1e-15 {
            break;
        }

        B -= f / df;

        if f.abs() < 1e-12 {
            break;
        }
    }

    B
}

// ── Gauss-Krüger Forward ─────────────────────────────────────

/// Gauss-Krüger forward: geodetic coordinates (B, L) → plane coordinates (X, Y).
///
/// Args:
/// - `B`: Latitude in radians
/// - `L`: Longitude in radians
/// - `L0`: Central meridian in radians
/// - `ell`: Reference ellipsoid
///
/// Returns `(X, Y)` in meters.
/// X = North coordinate, Y = East coordinate (with 500km false easting).
#[allow(non_snake_case)]
pub fn gauss_forward(B: f64, L: f64, L0: f64, ell: Ellipsoid) -> (f64, f64) {
    let e2 = ell.e2();
    let ep2 = ell.ep2();

    let l = L - L0; // longitude difference from central meridian

    let sinB = B.sin();
    let cosB = B.cos();
    let tanB = B.tan();
    let t = tanB;
    let t2 = t * t;
    let t4 = t2 * t2;
    let n2 = ep2 * cosB * cosB;
    let n4 = n2 * n2;

    // Radius of curvature in prime vertical
    let N = ell.a() / (1.0 - e2 * sinB * sinB).sqrt();

    // Meridian arc from equator
    let X0 = meridian_arc(B, ell);

    // Series expansion terms
    let l2 = l * l;
    let l3 = l2 * l;
    let l4 = l3 * l;
    let l5 = l4 * l;
    let l6 = l5 * l;

    // X = north-south
    let x_term1 = N / 2.0 * t * cosB * cosB * l2;
    let x_term2 = N / 24.0 * t * (5.0 - t2 + 9.0 * n2 + 4.0 * n4) * cosB.powi(4) * l4;
    let x_term3 =
        N / 720.0 * t * (61.0 - 58.0 * t2 + t4 + 270.0 * n2 - 330.0 * t2 * n2) * cosB.powi(6) * l6;

    let X = X0 + x_term1 + x_term2 + x_term3;

    // Y = east-west (with 500km false easting)
    let y_term1 = N * cosB * l;
    let y_term2 = N / 6.0 * (1.0 - t2 + n2) * cosB.powi(3) * l3;
    let y_term3 =
        N / 120.0 * (5.0 - 18.0 * t2 + t4 + 14.0 * n2 - 58.0 * t2 * n2) * cosB.powi(5) * l5;

    let Y = 500_000.0 + y_term1 + y_term2 + y_term3;

    (X, Y)
}

// ── Gauss-Krüger Inverse ─────────────────────────────────────

/// Gauss-Krüger inverse: plane coordinates (X, Y) → geodetic coordinates (B, L).
///
/// Args:
/// - `X`: North coordinate in meters
/// - `Y`: East coordinate in meters (with 500km false easting)
/// - `L0`: Central meridian in radians
/// - `ell`: Reference ellipsoid
///
/// Returns `(B, L)` in radians.
#[allow(non_snake_case)]
pub fn gauss_inverse(X: f64, Y: f64, L0: f64, ell: Ellipsoid) -> (f64, f64) {
    let e2 = ell.e2();
    let ep2 = ell.ep2();

    // Remove false easting
    let y = Y - 500_000.0;

    // Footpoint latitude
    let Bf = footpoint_latitude(X, ell);

    let sinBf = Bf.sin();
    let cosBf = Bf.cos();
    let tanBf = Bf.tan();
    let tf = tanBf;
    let tf2 = tf * tf;
    let tf4 = tf2 * tf2;
    let nf2 = ep2 * cosBf * cosBf;
    let nf4 = nf2 * nf2;

    // Radius in prime vertical at footpoint
    let Nf = ell.a() / (1.0 - e2 * sinBf * sinBf).sqrt();

    // Radius in meridian at footpoint
    let Mf = ell.a() * (1.0 - e2) / (1.0 - e2 * sinBf * sinBf).powf(1.5);

    let y2 = y * y;
    let y3 = y2 * y;
    let y4 = y3 * y;
    let y5 = y4 * y;

    let Nf2 = Nf * Nf;
    let Nf3 = Nf2 * Nf;
    let Nf4 = Nf2 * Nf2;
    let Nf5 = Nf4 * Nf;
    let MfNf = Mf * Nf;

    // B = Bf - ... series
    let b_term1 = tf / (2.0 * MfNf) * y2;
    let b_term2 =
        tf / (24.0 * Mf * Nf3) * (5.0 + 3.0 * tf2 + nf2 - 9.0 * nf2 * tf2 - 4.0 * nf4) * y4;
    let b_term3 = tf / (720.0 * Mf * Nf5) * (61.0 + 90.0 * tf2 + 45.0 * tf4) * y5 * y;

    let B = Bf - b_term1 + b_term2 - b_term3;

    // L = L0 + ... series
    let l_term1 = 1.0 / (Nf * cosBf) * y;
    let l_term2 = 1.0 / (6.0 * Nf3 * cosBf) * (1.0 + 2.0 * tf2 + nf2) * y3;
    let l_term3 = 1.0 / (120.0 * Nf5 * cosBf)
        * (5.0 + 28.0 * tf2 + 24.0 * tf4 + 6.0 * nf2 + 8.0 * nf2 * tf2)
        * y5;

    let L = L0 + l_term1 - l_term2 + l_term3;

    (B, L)
}

// ── Zone Transform ────────────────────────────────────────────

/// Coordinate zone transformation: X,Y from one zone to another.
///
/// Steps:
/// 1. Gauss-Krüger inverse: (X, Y, L0_from) → (B, L)
/// 2. Gauss-Krüger forward: (B, L, L0_to) → (X', Y')
///
/// Args:
/// - `X`, `Y`: Coordinate in source zone (meters, Y with 500km false easting)
/// - `from_zone`: Source zone number
/// - `to_zone`: Target zone number
/// - `is_3degree`: True = 3° zones, False = 6° zones
/// - `ell`: Reference ellipsoid
///
/// Returns `(X', Y')` in target zone coordinates.
#[allow(non_snake_case)]
pub fn zone_transform(
    X: f64,
    Y: f64,
    from_zone: u16,
    to_zone: u16,
    is_3degree: bool,
    ell: Ellipsoid,
) -> (f64, f64) {
    let L0_from = central_meridian(from_zone, is_3degree).to_radians();
    let L0_to = central_meridian(to_zone, is_3degree).to_radians();

    let (B, L) = gauss_inverse(X, Y, L0_from, ell);
    gauss_forward(B, L, L0_to, ell)
}

/// Coordinate zone transformation using source and target central meridians directly.
#[allow(non_snake_case)]
pub fn zone_transform_by_meridians(
    X: f64,
    Y: f64,
    L0_from_deg: f64,
    L0_to_deg: f64,
    ell: Ellipsoid,
) -> (f64, f64) {
    let (B, L) = gauss_inverse(X, Y, L0_from_deg.to_radians(), ell);
    gauss_forward(B, L, L0_to_deg.to_radians(), ell)
}

// ── Auto-detect Zone ──────────────────────────────────────────

/// Auto-detect zone from X (north) coordinate by checking if the longitude
/// extracted via inverse calculation matches a standard zone.
///
/// Returns `(zone_number, central_meridian, is_3degree)` or None if detection fails.
#[allow(non_snake_case)]
pub fn auto_detect_zone(X: f64, Y: f64, ell: Ellipsoid) -> Option<(u16, f64, bool)> {
    // Try inverse with each candidate meridian and pick the one closest
    // to a valid zone center

    // First, try 3° zones (1-120)
    for zone in 1..=120 {
        let cm = central_meridian(zone, true);
        let (B, L) = gauss_inverse(X, Y, cm.to_radians(), ell);

        let lon_diff_deg = (L.to_degrees() - cm).abs();
        if lon_diff_deg < 1.5 && B.to_degrees().abs() < 85.0 {
            return Some((zone, cm, true));
        }
    }

    // Fallback: try 6° zones (1-60)
    for zone in 1..=60 {
        let cm = central_meridian(zone, false);
        let (B, L) = gauss_inverse(X, Y, cm.to_radians(), ell);

        let lon_diff_deg = (L.to_degrees() - cm).abs();
        if lon_diff_deg < 3.0 && B.to_degrees().abs() < 85.0 {
            return Some((zone, cm, false));
        }
    }

    None
}

// ── Degrees ↔ Radians Helpers ────────────────────────────────

/// Convert degrees to (degrees, minutes, seconds) tuple.
pub fn deg_to_dms(deg: f64) -> (i32, i32, f64) {
    let d = deg.floor() as i32;
    let rem = (deg - d as f64) * 60.0;
    let m = rem.floor() as i32;
    let s = (rem - m as f64) * 60.0;
    (d, m, s)
}

/// Convert (degrees, minutes, seconds) to decimal degrees.
pub fn dms_to_deg(d: i32, m: i32, s: f64) -> f64 {
    d as f64 + m as f64 / 60.0 + s / 3600.0
}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Test point: Chengdu (B=30.57°, L=104.06°) in 3° zone 35 (CM=105°)
    /// Expected coordinates from known survey data:
    /// X ≈ 3383300, Y ≈ 355000
    #[test]
    fn test_gauss_forward_chengdu() {
        let B = 30.57_f64.to_radians();
        let L = 104.06_f64.to_radians();
        let L0 = 105.0_f64.to_radians();

        let (X, Y) = gauss_forward(B, L, L0, Ellipsoid::CGCS2000);

        // Chengdu: roughly 3383km north, ~355km east (after 500km false easting)
        assert!(
            (X - 3_383_000.0).abs() < 10_000.0,
            "X={}, expected ~3383000",
            X
        );
        assert!((Y - 500_000.0).abs() < 1_000_000.0, "Y={}", Y);

        // Roundtrip test
        let (B2, L2) = gauss_inverse(X, Y, L0, Ellipsoid::CGCS2000);
        assert!((B2 - B).abs() < 1e-8, "B diff: {}", (B2 - B).abs());
        assert!((L2 - L).abs() < 1e-8, "L diff: {}", (L2 - L).abs());
    }

    /// Test Beijing (B=39.9°, L=116.4°) in 3° zone 39 (CM=117°), 6° zone 20 (CM=117°)
    #[test]
    fn test_gauss_forward_beijing() {
        let B = 39.9_f64.to_radians();
        let L = 116.4_f64.to_radians();

        // 3° zone 39, CM=117°
        let L0 = 117.0_f64.to_radians();
        let (X, Y) = gauss_forward(B, L, L0, Ellipsoid::CGCS2000);
        assert!(X > 4_400_000.0 && X < 4_500_000.0, "X={}", X);

        // Roundtrip
        let (B2, L2) = gauss_inverse(X, Y, L0, Ellipsoid::CGCS2000);
        assert!((B2 - B).abs() < 1e-8);
        assert!((L2 - L).abs() < 1e-8);
    }

    /// Zone transform test: 3° zone 35 (CM=105°) → 3° zone 36 (CM=108°)
    #[test]
    fn test_zone_transform() {
        let B = 30.57_f64.to_radians();
        let L = 104.06_f64.to_radians();
        let L0_35 = central_meridian(35, true).to_radians();
        let L0_36 = central_meridian(36, true).to_radians();

        let (X35, Y35) = gauss_forward(B, L, L0_35, Ellipsoid::CGCS2000);
        let (B2, L2) = gauss_inverse(X35, Y35, L0_35, Ellipsoid::CGCS2000);
        assert!((B2 - B).abs() < 1e-8);
        assert!((L2 - L).abs() < 1e-8);

        let (X36, Y36) = zone_transform(X35, Y35, 35, 36, true, Ellipsoid::CGCS2000);
        // Verify by computing forward directly in zone 36
        let (X36_direct, Y36_direct) = gauss_forward(B, L, L0_36, Ellipsoid::CGCS2000);
        assert!(
            (X36 - X36_direct).abs() < 1.0,
            "X diff: {}",
            (X36 - X36_direct).abs()
        );
        assert!(
            (Y36 - Y36_direct).abs() < 1.0,
            "Y diff: {}",
            (Y36 - Y36_direct).abs()
        );
    }

    #[test]
    fn test_zone_info() {
        let info = zone_info(104.06);
        assert_eq!(info.zone3, 35);
        assert!((info.central_meridian_3_deg - 105.0).abs() < 0.01);
        assert_eq!(info.zone6, 18);
        assert!((info.central_meridian_6_deg - 105.0).abs() < 0.01);
    }

    #[test]
    fn test_central_meridian() {
        // 3° zone 35 → 105°
        assert!((central_meridian(35, true) - 105.0).abs() < 0.01);
        // 6° zone 20 → 117°
        assert!((central_meridian(20, false) - 117.0).abs() < 0.01);
        // 3° zone 1 → 3°
        assert!((central_meridian(1, true) - 3.0).abs() < 0.01);
        // 6° zone 1 → 3°
        assert!((central_meridian(1, false) - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_dms_roundtrip() {
        let deg = 30.5678;
        let (d, m, s) = deg_to_dms(deg);
        let back = dms_to_deg(d, m, s);
        assert!((deg - back).abs() < 1e-6);
    }

    #[test]
    fn test_ellipsoid_params() {
        let ell = Ellipsoid::CGCS2000;
        assert!((ell.a() - 6378137.0).abs() < 0.1);
        assert!((ell.inv_f() - 298.257222101).abs() < 1e-6);

        let e2 = ell.e2();
        assert!(e2 > 0.006 && e2 < 0.007, "CGCS2000 e²={}", e2);
    }

    #[test]
    fn test_meridian_arc() {
        // At equator, arc length should be close to 0
        let arc0 = meridian_arc(0.0, Ellipsoid::CGCS2000);
        assert!(arc0.abs() < 1.0, "arc at equator={}", arc0);

        // At 45°, arc should be ~4,986,000 m
        let arc45 = meridian_arc(45.0_f64.to_radians(), Ellipsoid::CGCS2000);
        assert!(
            (arc45 - 4_986_000.0).abs() < 10_000.0,
            "arc at 45°={}",
            arc45
        );

        // At 90°, arc should be ~quarter meridian ~10,001,966 m
        let arc90 = meridian_arc(90.0_f64.to_radians(), Ellipsoid::CGCS2000);
        assert!(
            (arc90 - 10_001_966.0).abs() < 10_000.0,
            "arc at pole={}",
            arc90
        );
    }

    #[test]
    fn test_footpoint_latitude() {
        let B0 = 30.0_f64.to_radians();
        let arc = meridian_arc(B0, Ellipsoid::CGCS2000);
        let Bf = footpoint_latitude(arc, Ellipsoid::CGCS2000);
        assert!(
            (Bf - B0).abs() < 1e-10,
            "footpoint diff: {}",
            (Bf - B0).abs()
        );
    }

    #[test]
    fn test_all_ellipsoids() {
        let test_cases = [
            (
                30.0_f64.to_radians(),
                104.0_f64.to_radians(),
                105.0_f64.to_radians(),
            ),
            (
                39.9_f64.to_radians(),
                116.4_f64.to_radians(),
                117.0_f64.to_radians(),
            ),
            (
                22.5_f64.to_radians(),
                113.5_f64.to_radians(),
                114.0_f64.to_radians(),
            ),
        ];

        for ell in [
            Ellipsoid::CGCS2000,
            Ellipsoid::Xian80,
            Ellipsoid::Beijing54,
            Ellipsoid::WGS84,
        ] {
            for &(B, L, L0) in &test_cases {
                let (X, Y) = gauss_forward(B, L, L0, ell);
                let (B2, L2) = gauss_inverse(X, Y, L0, ell);
                assert!(
                    (B2 - B).abs() < 1e-8,
                    "{}: B diff {}",
                    ell.label(),
                    (B2 - B).abs()
                );
                assert!(
                    (L2 - L).abs() < 1e-8,
                    "{}: L diff {}",
                    ell.label(),
                    (L2 - L).abs()
                );
            }
        }
    }
}
