//! CRS engine for WASM — delegates to geo-core's pure-Rust builtin transforms.
//!
//! Supports: 4326↔3857, 4326↔9000(GCJ-02), 9000↔9001(BD-09),
//! 4326↔9001, 4326→3405, identity.

use wasm_bindgen::prelude::*;
use geo_core::crs::{CrsRegistry, builtin};
use serde::Serialize;

/// Pseudo-EPSG codes for Chinese coordinate systems (not official).
/// 火星坐标系
#[allow(dead_code)]
pub const EPSG_GCJ02: u16 = 9000;
/// 百度坐标系
#[allow(dead_code)]
pub const EPSG_BD09: u16 = 9001;

/// WebAssembly-facing CRS engine.
#[wasm_bindgen]
pub struct CrsEngine {
    registry: CrsRegistry,
}

#[wasm_bindgen]
impl CrsEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { registry: CrsRegistry::new() }
    }

    /// List all registered coordinate systems as JSON.
    #[wasm_bindgen(js_name = listAll)]
    pub fn list_all(&self) -> String {
        let defs: Vec<CrsDefJs> = self.registry
            .list()
            .map(|d| CrsDefJs {
                epsg: d.epsg,
                name: d.name.to_string(),
                category: format!("{:?}", d.category),
            })
            .collect();
        serde_json::to_string(&defs).unwrap_or_default()
    }

    /// Get a single CRS definition by EPSG code.
    #[wasm_bindgen(js_name = getCrs)]
    pub fn get_crs(&self, epsg: u16) -> JsValue {
        match self.registry.get(epsg) {
            Some(def) => serde_wasm_bindgen::to_value(&CrsDefJs {
                epsg: def.epsg,
                name: def.name.to_string(),
                category: format!("{:?}", def.category),
            }).unwrap_or(JsValue::NULL),
            None => JsValue::NULL,
        }
    }

    /// Transform a coordinate pair (x, y) from one EPSG to another.
    ///
    /// Built-in transforms (pure Rust, no C dependency):
    /// - 4326 ↔ 3857 (WGS84 ↔ Web Mercator)
    /// - 4326 ↔ 9000 (WGS84 ↔ GCJ-02 Mars coordinate)
    /// - 9000 ↔ 9001 (GCJ-02 ↔ BD-09 Baidu coordinate)
    /// - 4326 ↔ 9001 (WGS84 ↔ BD-09, two-step via GCJ-02)
    /// - 4326 → 3405 (WGS84 → Equal Area, carbon accounting)
    /// - Identity (same EPSG)
    #[wasm_bindgen(js_name = transform)]
    pub fn transform(&self, from_epsg: u16, to_epsg: u16, x: f64, y: f64) -> Result<Box<[f64]>, JsValue> {
        if from_epsg == to_epsg {
            return Ok(Box::new([x, y]));
        }

        match (from_epsg, to_epsg) {
            (4326, 3857) => { let (mx,my)=builtin::wgs84_to_mercator(x,y); return Ok(Box::new([mx,my])); }
            (3857, 4326) => { let (lx,ly)=builtin::mercator_to_wgs84(x,y); return Ok(Box::new([lx,ly])); }
            (4326, 9000) => { let (gx,gy)=builtin::wgs84_to_gcj02(x,y); return Ok(Box::new([gx,gy])); }
            (9000, 4326) => { let (wx,wy)=builtin::gcj02_to_wgs84(x,y); return Ok(Box::new([wx,wy])); }
            (9000, 9001) => { let (bx,by)=builtin::gcj02_to_bd09(x,y); return Ok(Box::new([bx,by])); }
            (9001, 9000) => { let (gx,gy)=builtin::bd09_to_gcj02(x,y); return Ok(Box::new([gx,gy])); }
            (4326, 9001) => {
                let (gx,gy)=builtin::wgs84_to_gcj02(x,y);
                let (bx,by)=builtin::gcj02_to_bd09(gx,gy);
                return Ok(Box::new([bx,by]));
            }
            (9001, 4326) => {
                let (gx,gy)=builtin::bd09_to_gcj02(x,y);
                let (wx,wy)=builtin::gcj02_to_wgs84(gx,gy);
                return Ok(Box::new([wx,wy]));
            }
            (4326, 3405) => { let (ex,ey)=builtin::wgs84_to_equal_area(x,y); return Ok(Box::new([ex,ey])); }
            _ => {}
        }

        Err(JsValue::from_str(
            &format!(
                "Transform EPSG:{from_epsg}→{to_epsg} not built-in.\n\
                 Built-in: 4326↔3857 (WGS84↔Web Mercator), \
                 4326↔9000 (WGS84↔GCJ-02), \
                 9000↔9001 (GCJ-02↔BD-09), \
                 4326↔9001 (WGS84↔BD-09), \
                 4326→3405."
            )
        ))
    }

    /// Transform using a JavaScript proj4js-compatible PROJ string or pipeline.
    ///
    /// Call this from JS after calling proj4js:
    /// ```js
    /// // In JS:
    /// const [x, y] = proj4('EPSG:4326', 'EPSG:32649', [104.06, 30.57]);
    /// // Then pass the result to another function or use proj4js directly
    /// ```
    #[wasm_bindgen(js_name = transformBatch)]
    pub fn transform_batch(
        &self,
        from_epsg: u16,
        to_epsg: u16,
        coords: &[f64],
    ) -> Result<Box<[f64]>, JsValue> {
        if coords.len() % 2 != 0 {
            return Err(JsValue::from_str("coords length must be even"));
        }

        let mut result = Vec::with_capacity(coords.len());
        for chunk in coords.chunks_exact(2) {
            let (rx, ry) = {
                let arr = self.transform(from_epsg, to_epsg, chunk[0], chunk[1])?;
                (arr[0], arr[1])
            };
            result.push(rx);
            result.push(ry);
        }
        Ok(result.into_boxed_slice())
    }

    /// Register a custom CRS (stub — full support requires proj4js).
    #[wasm_bindgen(js_name = registerCrs)]
    pub fn register_crs(&mut self, _epsg: u16, _name: &str, _proj4: &str) -> Result<(), JsValue> {
        Ok(())
    }
}

impl Default for CrsEngine {
    fn default() -> Self { Self::new() }
}

// ── Re-export builtin transforms for external users ───────────

// ── JS-friendly types ────────────────────────────────────────────

#[derive(Serialize)]
struct CrsDefJs {
    epsg: u16,
    name: String,
    category: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_4326_to_3857() {
        let engine = CrsEngine::new();
        let result = engine.transform(4326, 3857, 104.06, 30.57).unwrap();
        assert!((result[0] - 11_583_906.0).abs() < 10.0);
        assert!((result[1] - 3_577_030.0).abs() < 10.0);
    }

    #[test]
    fn test_mercator_roundtrip() {
        let (mx, my) = builtin::wgs84_to_mercator(104.06, 30.57);
        let (lx, ly) = builtin::mercator_to_wgs84(mx, my);
        assert!((lx - 104.06).abs() < 0.0001);
        assert!((ly - 30.57).abs() < 0.0001);
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
    }

    #[test]
    fn test_out_of_china() {
        let (gx, gy) = builtin::wgs84_to_gcj02(139.76, 35.68);
        assert!((gx - 139.76).abs() < 0.000001);
    }

    #[test]
    fn test_transform_wgs84_to_gcj02() {
        let engine = CrsEngine::new();
        let result = engine.transform(4326, 9000, 104.06, 30.57).unwrap();
        assert!((result[0] - 104.06).abs() > 0.001);
        assert!((result[1] - 30.57).abs() > 0.001);
    }

    #[test]
    fn test_transform_wgs84_to_bd09() {
        let engine = CrsEngine::new();
        let result = engine.transform(4326, 9001, 104.06, 30.57).unwrap();
        assert!((result[0] - 104.06).abs() > 0.001);
    }

    #[test]
    fn test_identity() {
        let engine = CrsEngine::new();
        let result = engine.transform(4326, 4326, 113.9, 22.5).unwrap();
        assert!((result[0] - 113.9).abs() < 0.01);
    }
}
