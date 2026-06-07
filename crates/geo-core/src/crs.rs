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

    /// Without proj feature: identity check only.
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
        Err(GeoError::Unimplemented(
            "coordinate transform requires the `proj` feature.\n\
             Build with: cargo build --features proj\n\
             This requires cmake + a C compiler to build libproj from source."
                .into(),
        ))
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
}
