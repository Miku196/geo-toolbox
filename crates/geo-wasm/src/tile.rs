//! 浏览器端瓦片工具 — lat/lon → z/x/y + MVT 编码。

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// 瓦片坐标。
#[wasm_bindgen]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileCoord {
    pub x: u32,
    pub y: u32,
    pub z: u8,
}

/// 瓦片引擎 — 经纬度与瓦片坐标互转 + MVT 编码。
#[wasm_bindgen]
#[wasm_bindgen]
#[derive(Default)]
pub struct TileEngine;

#[wasm_bindgen]
impl TileEngine {
    /// 创建瓦片引擎。
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self
    }

    /// 经纬度 → 瓦片坐标。
    pub fn latlon_to_tile(&self, lon: f64, lat: f64, zoom: u8) -> TileCoord {
        let (x, y, z) = geo_tile::latlon_to_tile(lon, lat, zoom);
        TileCoord { x, y, z }
    }

    /// 瓦片坐标 → 中心经纬度。
    pub fn tile_to_latlon(&self, x: u32, y: u32, zoom: u8) -> js_sys::Array {
        let (lon, lat) = geo_tile::tile_to_latlon(x, y, zoom);
        let arr = js_sys::Array::new();
        arr.push(&JsValue::from_f64(lon));
        arr.push(&JsValue::from_f64(lat));
        arr
    }

    /// 瓦片边界 (min_lon, min_lat, max_lon, max_lat)。
    pub fn tile_bounds(&self, x: u32, y: u32, zoom: u8) -> js_sys::Array {
        let (w, s, e, n) = geo_tile::tile_bounds(x, y, zoom);
        let arr = js_sys::Array::new();
        arr.push(&JsValue::from_f64(w));
        arr.push(&JsValue::from_f64(s));
        arr.push(&JsValue::from_f64(e));
        arr.push(&JsValue::from_f64(n));
        arr
    }

    /// 瓦片 URL（支持 OSM / 高德 / 天地图）。
    pub fn tile_url(&self, source: &str, x: u32, y: u32, z: u8) -> String {
        let src = match source {
            "gaode" | "amap" => geo_tile::TileSource::Gaode,
            "tianditu" => geo_tile::TileSource::TianDiTu,
            _ => geo_tile::TileSource::OpenStreetMap,
        };
        geo_tile::tile_url(src, x, y, z)
    }

    /// GeoJSON FeatureCollection → MVT 字节（Uint8Array）。
    /// extent 通常为 4096。
    pub fn encode_mvt(
        &self,
        layer_name: &str,
        geojson_fc: &str,
        tile_x: u32,
        tile_y: u32,
        zoom: u8,
        extent: u32,
    ) -> Result<Vec<u8>, JsValue> {
        let fc: serde_json::Value =
            serde_json::from_str(geojson_fc).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let features = fc["features"]
            .as_array()
            .ok_or_else(|| JsValue::from_str("no features"))?;

        let encoder = geo_tile::MvtEncoder::new(extent);
        encoder
            .encode_tile(layer_name, features, tile_x, tile_y, zoom)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

#[wasm_bindgen]
pub fn latlon_to_tile(lon: f64, lat: f64, zoom: u8) -> TileCoord {
    TileEngine::new().latlon_to_tile(lon, lat, zoom)
}

#[wasm_bindgen]
pub fn tile_url_osm(x: u32, y: u32, z: u8) -> String {
    TileEngine::new().tile_url("osm", x, y, z)
}

#[wasm_bindgen]
pub fn tile_url_gaode(x: u32, y: u32, z: u8) -> String {
    TileEngine::new().tile_url("gaode", x, y, z)
}
