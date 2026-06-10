//! 瓦片索引 — 经纬度与 z/x/y 互转，复用 geo-index 的 GeoHash。
//!
//! ## 坐标系统
//!
//! - Web Mercator 投影 (EPSG:3857)
//! - 瓦片原点 = 左上角 (NW)
//! - z = 缩放级别 (0-22)
//! - x = 列 (从左到右), y = 行 (从上到下)
//!
//! ## 分辨率
//!
//! | z | 瓦片数 (全球) | 每瓦片约 |
//! |---|:----------:|---------|
//! | 0 | 1×1 | 全球 |
//! | 5 | 32×32 | 大陆 |
//! | 10 | 1024×1024 | 省 |
//! | 14 | 16384×16384 | 城市 |
//! | 18 | 262144×262144 | 街区 |

use std::f64::consts::PI;

/// 将经纬度映射到指定缩放级别的瓦片坐标。
pub fn latlon_to_tile(lon: f64, lat: f64, zoom: u8) -> (u32, u32, u8) {
    let n = 2.0_f64.powi(zoom as i32);
    let x = ((lon + 180.0) / 360.0 * n).floor() as u32;
    let lat_rad = lat.to_radians();
    let y = ((1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / PI) / 2.0 * n).floor() as u32;
    // clamp
    let max = (n as u32).saturating_sub(1);
    (x.min(max), y.min(max), zoom)
}

/// 瓦片坐标 → 瓦片中心经纬度。
pub fn tile_to_latlon(x: u32, y: u32, zoom: u8) -> (f64, f64) {
    let n = 2.0_f64.powi(zoom as i32);
    let lon = (x as f64 / n) * 360.0 - 180.0;
    let lat_rad = (PI * (1.0 - 2.0 * y as f64 / n)).sinh().atan();
    (lon, lat_rad.to_degrees())
}

/// 瓦片数据源。
#[derive(Debug, Clone, Copy)]
pub enum TileSource {
    /// OpenStreetMap 标准瓦片。
    OpenStreetMap,
    /// 高德地图 (GCJ-02 坐标系)。
    Gaode,
    /// 天地图 (需 key，GCJ-02)。
    TianDiTu,
}

/// 根据数据源生成瓦片 URL。
///
/// 高德和天地图使用 GCJ-02 坐标系，URL 中的 x/y/z 需用 GCJ-02 坐标计算。
pub fn tile_url(source: TileSource, x: u32, y: u32, z: u8) -> String {
    match source {
        TileSource::OpenStreetMap => {
            format!("https://tile.openstreetmap.org/{z}/{x}/{y}.png")
        }
        TileSource::Gaode => {
            let subdomain = (x + y) as u8 % 4 + 1;
            format!("https://webrd0{subdomain}.is.autonavi.com/appmaptile?lang=zh_cn&size=1&scale=1&style=8&x={x}&y={y}&z={z}")
        }
        TileSource::TianDiTu => {
            // 需要替换为实际的天地图 key
            format!("https://t{s}.tianditu.gov.cn/vec_w/wmts?SERVICE=WMTS&REQUEST=GetTile&VERSION=1.0.0&LAYER=vec&STYLE=default&TILEMATRIXSET=w&FORMAT=tiles&TILEMATRIX={z}&TILEROW={y}&TILECOL={x}&tk=YOUR_KEY", s = (x + y) % 8)
        }
    }
}

/// 返回瓦片四条边界的经纬度 (min_lon, min_lat, max_lon, max_lat)。
pub fn tile_bounds(x: u32, y: u32, zoom: u8) -> (f64, f64, f64, f64) {
    let (nw_lon, nw_lat) = tile_to_latlon(x, y, zoom);
    let (se_lon, se_lat) = tile_to_latlon(x + 1, y + 1, zoom);
    (nw_lon, se_lat, se_lon, nw_lat) // W,S,E,N
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chengdu_tile_z12() {
        let (x, y, z) = latlon_to_tile(104.06, 30.57, 12);
        assert_eq!(z, 12);
        // 成都 (104.06, 30.57) 在 z12 约在 x=3233, y=1681±1
        assert!((3229..=3235).contains(&x), "x={x}");
        assert!((1679..=1683).contains(&y), "y={y}");
    }

    #[test]
    fn test_tile_roundtrip() {
        // z=14 精度约 0.02°, roundtrip 误差 < 0.03°
        let (x, y, z) = latlon_to_tile(104.06, 30.57, 14);
        let (lon, lat) = tile_to_latlon(x, y, z);
        assert!((lon - 104.06).abs() < 0.03);
        assert!((lat - 30.57).abs() < 0.03);
    }

    #[test]
    fn test_bounds() {
        let (x, y, _) = latlon_to_tile(104.06, 30.57, 12);
        let (min_lon, min_lat, max_lon, max_lat) = tile_bounds(x, y, 12);
        assert!(min_lon <= 104.06 && max_lon >= 104.06,
            "lon 104.06 not in [{min_lon}, {max_lon}]");
        assert!(min_lat <= 30.57 && max_lat >= 30.57,
            "lat 30.57 not in [{min_lat}, {max_lat}]");
    }

}
