//! CRS (Coordinate Reference System) registry.
//!
//! Manages coordinate system definitions. Coordinate transforms require
//! the `proj` feature (disabled by default to avoid system deps).

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::errors::{GeoError, GeoResult};

/// Category of a CRS — determines which pipeline stage uses it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrsCategory {
    /// Default storage CRS: EPSG:4326 (WGS84 lat/lon).
    Storage,
    /// Web map display: EPSG:3857 (Web Mercator).
    Display,
    /// Area-sensitive computations: EPSG:3405 (World Equal Area) or local UTM.
    Carbon,
    /// Local engineering / CAD coordinate system.
    CadLocal,
}

/// A coordinate reference system definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrsDef {
    /// EPSG code (e.g. 4326).
    pub epsg: u16,
    /// Human-readable name.
    pub name: &'static str,
    /// PROJ string.
    pub proj4: &'static str,
    /// Which pipeline stage primarily uses this CRS.
    pub category: CrsCategory,
}

/// Built-in CRS definitions — covers 90% of common geo-toolbox use cases.
pub const BUILTIN_CRS: &[CrsDef] = &[
    CrsDef {
        epsg: 4326,
        name: "WGS 84",
        proj4: "+proj=longlat +datum=WGS84 +no_defs",
        category: CrsCategory::Storage,
    },
    CrsDef {
        epsg: 3857,
        name: "WGS 84 / Pseudo-Mercator",
        proj4: "+proj=merc +a=6378137 +b=6378137 +lat_ts=0 +lon_0=0 +x_0=0 +y_0=0 +k=1 +units=m +nadgrids=@null +no_defs",
        category: CrsCategory::Display,
    },
    CrsDef {
        epsg: 9000,
        name: "GCJ-02 (Mars Coordinate)",
        proj4: "+proj=longlat +datum=WGS84 +no_defs",
        category: CrsCategory::Display,
    },
    CrsDef {
        epsg: 9001,
        name: "BD-09 (Baidu Coordinate)",
        proj4: "+proj=longlat +datum=WGS84 +no_defs",
        category: CrsCategory::Display,
    },
    CrsDef {
        epsg: 32649,
        name: "WGS 84 / UTM zone 49N",
        proj4: "+proj=utm +zone=49 +datum=WGS84 +units=m +no_defs",
        category: CrsCategory::Carbon,
    },
    CrsDef {
        epsg: 32650,
        name: "WGS 84 / UTM zone 50N",
        proj4: "+proj=utm +zone=50 +datum=WGS84 +units=m +no_defs",
        category: CrsCategory::Carbon,
    },
    CrsDef {
        epsg: 3405,
        name: "World Equal Area",
        proj4: "+proj=cea +lon_0=0 +lat_ts=30 +x_0=0 +y_0=0 +datum=WGS84 +units=m +no_defs",
        category: CrsCategory::Carbon,
    },
];

/// Registry of known CRS. Coordinate transforms require the `proj` feature.
pub struct CrsRegistry {
    by_epsg: FxHashMap<u16, CrsDef>,
}

impl CrsRegistry {
    /// Create a new registry populated with [`BUILTIN_CRS`].
    pub fn new() -> Self {
        let by_epsg: FxHashMap<_, _> = BUILTIN_CRS.iter().map(|c| (c.epsg, c.clone())).collect();
        Self { by_epsg }
    }

    /// Look up a CRS definition by EPSG code.
    pub fn get(&self, epsg: u16) -> Option<&CrsDef> {
        self.by_epsg.get(&epsg)
    }

    /// Iterate over all registered CRS definitions.
    pub fn list(&self) -> impl Iterator<Item = &CrsDef> {
        self.by_epsg.values()
    }

    /// Find all CRS of a given category.
    pub fn by_category(&self, category: CrsCategory) -> Vec<&CrsDef> {
        self.by_epsg
            .values()
            .filter(|c| c.category == category)
            .collect()
    }

    /// Transform a single (x, y) coordinate pair.
    ///
    /// With the `proj` feature: thread-safe via [`std::cell::RefCell`]
    /// per Proj instance cached on the current thread. Suitable for
    /// both CLI (single-thread) and MCP server (multi-thread, each
    /// tokio task gets its own Proj via thread-local storage).
    ///
    /// Without `proj`: identity transforms only.
    #[cfg(feature = "proj")]
    pub fn transform_point(
        &self,
        from_epsg: u16,
        to_epsg: u16,
        x: f64,
        y: f64,
    ) -> GeoResult<(f64, f64)> {
        use proj::Proj;
        use std::cell::RefCell;

        let from_def = self
            .get(from_epsg)
            .ok_or(GeoError::CrsNotFound(from_epsg, to_epsg))?;
        let to_def = self
            .get(to_epsg)
            .ok_or(GeoError::CrsNotFound(from_epsg, to_epsg))?;

        // Thread-local cache: each OS thread gets its own Proj instance.
        // Safe because tokio tasks that share a pool thread will reuse
        // the same cached Proj (RefCell gives interior mutability).
        thread_local! {
            static PROJ_CACHE: RefCell<rustc_hash::FxHashMap<(u16, u16), Proj>> =
                RefCell::new(rustc_hash::FxHashMap::default());
        }

        PROJ_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            let proj = if let Some(p) = cache.get(&(from_epsg, to_epsg)) {
                // Reuse cached instance
                p
            } else {
                let p = Proj::new_known_crs(from_def.proj4, to_def.proj4, None)
                    .map_err(|e| GeoError::CrsTransform(format!("{from_epsg}→{to_epsg}: {e}")))?;
                cache.insert((from_epsg, to_epsg), p);
                cache.get(&(from_epsg, to_epsg)).unwrap()
            };
            // Note: proj.convert takes (&self) so a shared reference is fine
            proj.convert((x, y))
                .map_err(|e| GeoError::CrsTransform(format!("({x},{y}): {e}")))
        })
    }

    /// Without proj feature: pure Rust built-in transforms.
    /// Supports: 4326↔3857, 4326↔9000(GCJ-02), 9000↔9001(BD-09),
    /// 4326↔9001, 4326→3405, identity. UTM projections still need proj.
    #[cfg(not(feature = "proj"))]
    pub fn transform_point(
        &self,
        from_epsg: u16,
        to_epsg: u16,
        x: f64,
        y: f64,
    ) -> GeoResult<(f64, f64)> {
        if from_epsg == to_epsg {
            return Ok((x, y));
        }

        use crate::crs::builtin;
        match (from_epsg, to_epsg) {
            // WGS84 ↔ Web Mercator
            (4326, 3857) => Ok(builtin::wgs84_to_mercator(x, y)),
            (3857, 4326) => Ok(builtin::mercator_to_wgs84(x, y)),

            // WGS84 ↔ GCJ-02 (Mars coordinate)
            (4326, 9000) => Ok(builtin::wgs84_to_gcj02(x, y)),
            (9000, 4326) => Ok(builtin::gcj02_to_wgs84(x, y)),

            // GCJ-02 ↔ BD-09
            (9000, 9001) => Ok(builtin::gcj02_to_bd09(x, y)),
            (9001, 9000) => Ok(builtin::bd09_to_gcj02(x, y)),

            // WGS84 ↔ BD-09 (two-step via GCJ-02)
            (4326, 9001) => {
                let (gx, gy) = builtin::wgs84_to_gcj02(x, y);
                Ok(builtin::gcj02_to_bd09(gx, gy))
            }
            (9001, 4326) => {
                let (gx, gy) = builtin::bd09_to_gcj02(x, y);
                Ok(builtin::gcj02_to_wgs84(gx, gy))
            }

            // WGS84 → Equal Area (carbon accounting)
            (4326, 3405) => Ok(builtin::wgs84_to_equal_area(x, y)),

            _ => Err(GeoError::Unimplemented(
                format!(
                    "Transform EPSG:{from_epsg}→{to_epsg} not built-in.\n\
                     Built-in: 4326↔3857, 4326↔9000(GCJ-02), 9000↔9001(BD-09),\n\
                     4326↔9001, 4326→3405.\n\
                     For UTM/proj-based transforms, build with: cargo build --features proj"
                )
            )),
        }
    }

    /// Total count of registered CRS.
    pub fn len(&self) -> usize {
        self.by_epsg.len()
    }

    /// Returns true if no CRS are registered.
    pub fn is_empty(&self) -> bool {
        self.by_epsg.is_empty()
    }
}

impl Default for CrsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Pure Rust built-in transforms (zero C dependency) ────────────

/// 纯 Rust 实现的坐标变换（零 C 依赖）。
///
/// 支持：WGS84 ↔ Web Mercator, WGS84 → 等积投影, GCJ-02 / BD-09 中国坐标系。
pub mod builtin {
    use std::f64::consts::PI;

    const EARTH_RADIUS: f64 = 6_378_137.0;
    const DEG_TO_RAD: f64 = PI / 180.0;

    /// WGS84 (EPSG:4326) → Web Mercator (EPSG:3857)。
    pub fn wgs84_to_mercator(lon: f64, lat: f64) -> (f64, f64) {
        let x = lon * DEG_TO_RAD * EARTH_RADIUS;
        let y = ((90.0 + lat) * DEG_TO_RAD / 2.0).tan().ln() * EARTH_RADIUS;
        (x, y)
    }

    /// Web Mercator (EPSG:3857) → WGS84 (EPSG:4326)。
    pub fn mercator_to_wgs84(x: f64, y: f64) -> (f64, f64) {
        let lon = (x / EARTH_RADIUS) / DEG_TO_RAD;
        let lat = (2.0 * (y / EARTH_RADIUS).exp().atan() - PI / 2.0) / DEG_TO_RAD;
        (lon, lat)
    }

    /// WGS84 (EPSG:4326) → 等积投影（近似 EPSG:3405）。
    pub fn wgs84_to_equal_area(lon: f64, lat: f64) -> (f64, f64) {
        let sp = 30.0_f64.to_radians();
        (lon * DEG_TO_RAD * EARTH_RADIUS * sp.cos(), lat.sin() * EARTH_RADIUS)
    }

    // ── Chinese coordinate systems (GCJ-02 / BD-09) ───────────
    const X_PI: f64 = PI * 3000.0 / 180.0;
    const A: f64 = 6378245.0;
    const EE: f64 = 0.006_693_421_622_965_943;

    /// WGS84 → GCJ-02（火星坐标系）。
    pub fn wgs84_to_gcj02(lon: f64, lat: f64) -> (f64, f64) {
        if out_of_china(lon, lat) { return (lon, lat); }
        let (dlon, dlat) = delta(lon, lat);
        (lon + dlon, lat + dlat)
    }

    /// GCJ-02（火星坐标系） → WGS84。
    pub fn gcj02_to_wgs84(lon: f64, lat: f64) -> (f64, f64) {
        if out_of_china(lon, lat) { return (lon, lat); }
        let (mut wx, mut wy) = (lon, lat);
        for _ in 0..5 {
            let (gx, gy) = wgs84_to_gcj02(wx, wy);
            wx += lon - gx;
            wy += lat - gy;
        }
        (wx, wy)
    }

    /// GCJ-02 → BD-09（百度坐标系）。
    pub fn gcj02_to_bd09(lon: f64, lat: f64) -> (f64, f64) {
        let z = (lon*lon + lat*lat).sqrt() + 0.00002*(lat*X_PI).sin();
        let t = lat.atan2(lon) + 0.000003*(lon*X_PI).cos();
        (z*t.cos() + 0.0065, z*t.sin() + 0.006)
    }

    /// BD-09（百度坐标系） → GCJ-02。
    pub fn bd09_to_gcj02(lon: f64, lat: f64) -> (f64, f64) {
        let x = lon - 0.0065;
        let y = lat - 0.006;
        let z = (x*x + y*y).sqrt() - 0.00002*(y*X_PI).sin();
        let t = y.atan2(x) - 0.000003*(x*X_PI).cos();
        (z*t.cos(), z*t.sin())
    }

    fn out_of_china(lon: f64, lat: f64) -> bool {
        !(72.004..=137.8347).contains(&lon) || !(0.8293..=55.8271).contains(&lat)
    }

    fn delta(lon: f64, lat: f64) -> (f64, f64) {
        let dl = tl(lon - 105.0, lat - 35.0);
        let db = tb(lon - 105.0, lat - 35.0);
        let r = lat * PI / 180.0;
        let m = 1.0 - EE * r.sin() * r.sin();
        let s = m.sqrt();
        ((dl*180.0)/(A/s*r.cos()*PI), (db*180.0)/((A*(1.0-EE))/(m*s)*PI))
    }

    fn tb(x: f64, y: f64) -> f64 {
        let mut r = -100.0 + 2.0*x + 3.0*y + 0.2*y*y + 0.1*x*y + 0.2*x.abs().sqrt();
        r += (20.0*(6.0*x*PI).sin()+20.0*(2.0*x*PI).sin())*2.0/3.0;
        r += (20.0*(y*PI).sin()+40.0*(y/3.0*PI).sin())*2.0/3.0;
        r += (160.0*(y/12.0*PI).sin()+320.0*(y*PI/30.0).sin())*2.0/3.0;
        r
    }

    fn tl(x: f64, y: f64) -> f64 {
        let mut r = 300.0 + x + 2.0*y + 0.1*x*x + 0.1*x*y + 0.1*x.abs().sqrt();
        r += (20.0*(6.0*x*PI).sin()+20.0*(2.0*x*PI).sin())*2.0/3.0;
        r += (20.0*(x*PI).sin()+40.0*(x/3.0*PI).sin())*2.0/3.0;
        r += (150.0*(x/12.0*PI).sin()+300.0*(x/30.0*PI).sin())*2.0/3.0;
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_builtin() {
        let reg = CrsRegistry::new();
        assert!(reg.len() >= 5);
    }

    #[test]
    fn test_get_wgs84() {
        let reg = CrsRegistry::new();
        let wgs84 = reg.get(4326).expect("WGS84 should exist");
        assert_eq!(wgs84.name, "WGS 84");
    }

    #[test]
    fn test_get_nonexistent() {
        let reg = CrsRegistry::new();
        assert!(reg.get(9999).is_none());
    }

    #[test]
    fn test_identity_transform() {
        let reg = CrsRegistry::new();
        let (x, y) = reg.transform_point(4326, 4326, 113.9, 22.5).unwrap();
        assert!((x - 113.9).abs() < 0.001);
        assert!((y - 22.5).abs() < 0.001);
    }

    #[test]
    fn test_by_category() {
        let reg = CrsRegistry::new();
        let carbon = reg.by_category(CrsCategory::Carbon);
        assert!(!carbon.is_empty());
    }

    #[test]
    fn test_wgs84_to_mercator_chengdu() {
        let (x, y) = builtin::wgs84_to_mercator(104.06, 30.57);
        assert!((x - 11_583_906.0).abs() < 10.0);
        assert!((y - 3_577_030.0).abs() < 10.0);
    }

    #[test]
    fn test_mercator_roundtrip() {
        let (mx, my) = builtin::wgs84_to_mercator(104.06, 30.57);
        let (lx, ly) = builtin::mercator_to_wgs84(mx, my);
        assert!((lx - 104.06).abs() < 0.0001);
        assert!((ly - 30.57).abs() < 0.0001);
    }

    #[test]
    fn test_wgs84_to_gcj02_offset() {
        let (gx, gy) = builtin::wgs84_to_gcj02(104.06, 30.57);
        assert!((gx - 104.06).abs() > 0.001);
        assert!((gy - 30.57).abs() > 0.001);
    }

    #[test]
    fn test_gcj02_wgs84_roundtrip() {
        let (gx, gy) = builtin::wgs84_to_gcj02(104.06, 30.57);
        let (wx, wy) = builtin::gcj02_to_wgs84(gx, gy);
        assert!((wx - 104.06).abs() < 0.000001);
        assert!((wy - 30.57).abs() < 0.000001);
    }

    #[test]
    fn test_bd09_gcj02_roundtrip() {
        let (gx, gy) = builtin::wgs84_to_gcj02(104.06, 30.57);
        let (bx, by) = builtin::gcj02_to_bd09(gx, gy);
        let (gx2, gy2) = builtin::bd09_to_gcj02(bx, by);
        assert!((gx2 - gx).abs() < 0.000001);
        assert!((gy2 - gy).abs() < 0.000001);
    }

    #[test]
    fn test_cli_transform_4326_to_3857() {
        let reg = CrsRegistry::new();
        let (x, y) = reg.transform_point(4326, 3857, 104.06, 30.57).unwrap();
        assert!((x - 11_583_906.0).abs() < 10.0);
        assert!((y - 3_577_030.0).abs() < 10.0);
    }

    #[test]
    fn test_cli_transform_4326_to_9000() {
        let reg = CrsRegistry::new();
        let (gx, _gy) = reg.transform_point(4326, 9000, 104.06, 30.57).unwrap();
        assert!((gx - 104.06).abs() > 0.001, "GCJ-02 should differ from WGS84");
    }

    #[test]
    fn test_cli_transform_4326_to_9001() {
        let reg = CrsRegistry::new();
        let (bx, _by) = reg.transform_point(4326, 9001, 104.06, 30.57).unwrap();
        assert!((bx - 104.06).abs() > 0.001, "BD-09 should differ from WGS84");
    }

    #[test]
    fn test_cli_transform_unsupported() {
        let reg = CrsRegistry::new();
        assert!(reg.transform_point(4326, 32649, 104.0, 30.0).is_err());
    }
}
