//! MVT (Mapbox Vector Tile) 编码器。
//!
//! 实现 [Mapbox Vector Tile Specification v2.1](https://github.com/mapbox/vector-tile-spec)。
//! 纯 Rust 手工 protobuf 编码，零外部 schema 依赖。
//!
//! ## 几何编码
//!
//! MVT 使用"命令整数"编码几何：
//! - MoveTo(1): 移动到绝对坐标 (x,y)
//! - LineTo(2): 画线到绝对坐标 (x,y)
//! - ClosePath(7): 闭合当前环
//!
//! 坐标存储为相对于前一点的增量 (delta encoding)。

use geo_core::errors::{GeoError, GeoResult};
use serde_json::Value;

// ═══════════════════════════════════════════════════════════
// 低层 Protobuf 编码
// ═══════════════════════════════════════════════════════════

/// Protobuf wire types.
#[derive(Debug, Clone, Copy)]
enum WireType {
    Varint = 0,
    LengthDelimited = 2,
}

/// 写入 varint。
fn write_varint(buf: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        buf.push((v as u8 & 0x7F) | 0x80);
        v >>= 7;
    }
    buf.push(v as u8);
}

/// 写入 field tag。
fn write_tag(buf: &mut Vec<u8>, field_number: u32, wire_type: WireType) {
    write_varint(buf, ((field_number << 3) | wire_type as u32) as u64);
}

/// 写入 uint32 字段。
fn write_uint32(buf: &mut Vec<u8>, field_number: u32, v: u32) {
    write_tag(buf, field_number, WireType::Varint);
    write_varint(buf, v as u64);
}

/// 写入 string 字段。
fn write_string(buf: &mut Vec<u8>, field_number: u32, s: &str) {
    write_tag(buf, field_number, WireType::LengthDelimited);
    write_varint(buf, s.len() as u64);
    buf.extend_from_slice(s.as_bytes());
}

/// 写入 bytes 字段。
fn write_bytes(buf: &mut Vec<u8>, field_number: u32, data: &[u8]) {
    write_tag(buf, field_number, WireType::LengthDelimited);
    write_varint(buf, data.len() as u64);
    buf.extend_from_slice(data);
}

/// 写入 packed uint32 数组。
fn write_packed_uint32(buf: &mut Vec<u8>, field_number: u32, values: &[u32]) {
    let mut payload = Vec::with_capacity(values.len() * 5);
    for &v in values {
        write_varint(&mut payload, v as u64);
    }
    write_tag(buf, field_number, WireType::LengthDelimited);
    write_varint(buf, payload.len() as u64);
    buf.extend_from_slice(&payload);
}

// ═══════════════════════════════════════════════════════════
// MVT 数据类型
// ═══════════════════════════════════════════════════════════

/// MVT 几何类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeomType {
    /// 未知/未设置。
    Unknown = 0,
    /// 点。
    Point = 1,
    /// 线。
    Linestring = 2,
    /// 多边形。
    Polygon = 3,
}

/// 单条 MVT Feature。
#[derive(Debug, Clone)]
pub struct MvtFeature {
    /// Feature ID（可选，用于增量更新）。
    pub id: Option<u64>,
    /// 属性键值对。会自动去重编码为 MVT 的 keys/values 表。
    pub tags: Vec<(String, MvtValue)>,
    /// 几何类型。
    pub geom_type: GeomType,
    /// 已编码的命令整数序列（delta-encoded）。
    pub geometry: Vec<u32>,
}

/// MVT 值类型（对应 protobuf Value 消息）。
#[derive(Debug, Clone, PartialEq)]
pub enum MvtValue {
    /// 字符串。
    String(String),
    /// 浮点数。
    Float(f32),
    /// 双精度浮点数。
    Double(f64),
    /// 有符号整数。
    Int(i64),
    /// 无符号整数。
    Uint(u64),
    /// zigzag 编码整数。
    Sint(i64),
    /// 布尔。
    Bool(bool),
}

/// 一个 MVT 图层（一个 Tile 可含多个 Layer）。
#[derive(Debug, Clone)]
pub struct MvtLayer {
    /// 图层名。
    pub name: String,
    /// 瓦片范围（通常 4096）。
    pub extent: u32,
    /// 该图层的 features。
    pub features: Vec<MvtFeature>,
}

/// MVT 编码器。
pub struct MvtEncoder {
    extent: u32,
}

impl MvtEncoder {
    /// 创建编码器，指定瓦片坐标空间范围（通常 4096）。
    pub fn new(extent: u32) -> Self {
        Self { extent }
    }

    /// 从 GeoJSON Feature 构建 MvtFeature。
    ///
    /// 将 WGS84 坐标映射到瓦片局部坐标系 [0, extent]。
    pub fn feature_from_geojson(
        &self,
        feature: &Value,
        tile_x: u32,
        tile_y: u32,
        zoom: u8,
    ) -> GeoResult<MvtFeature> {
        let props = &feature["properties"];
        let geom = &feature["geometry"];
        let gtype = geom["type"].as_str().unwrap_or("Point");
        let coords = &geom["coordinates"];

        let (min_lon, min_lat, max_lon, max_lat) = crate::tile_index::tile_bounds(tile_x, tile_y, zoom);

        let to_tile = |lon: f64, lat: f64| -> (u32, u32) {
            let x = ((lon - min_lon) / (max_lon - min_lon) * self.extent as f64) as u32;
            let y = ((max_lat - lat) / (max_lat - min_lat) * self.extent as f64) as u32;
            (x.min(self.extent), y.min(self.extent))
        };

        let (geom_type, commands) = match gtype {
            "Point" => {
                let x = coords[0].as_f64().unwrap_or(0.0);
                let y = coords[1].as_f64().unwrap_or(0.0);
                let (tx, ty) = to_tile(x, y);
                (GeomType::Point, encode_point(tx, ty))
            }
            "LineString" => {
                let pts: Vec<(u32, u32)> = coords.as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|c| {
                        Some(to_tile(c[0].as_f64()?, c[1].as_f64()?))
                    })
                    .collect();
                (GeomType::Linestring, encode_linestring(&pts))
            }
            "Polygon" => {
                let rings: Vec<Vec<(u32, u32)>> = coords.as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|ring| {
                        ring.as_array()
                            .unwrap_or(&vec![])
                            .iter()
                            .filter_map(|c| {
                                Some(to_tile(c[0].as_f64()?, c[1].as_f64()?))
                            })
                            .collect()
                    })
                    .collect();
                (GeomType::Polygon, encode_polygon(&rings))
            }
            "MultiPoint" => {
                let pts: Vec<(u32, u32)> = coords.as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|c| {
                        Some(to_tile(c[0].as_f64()?, c[1].as_f64()?))
                    })
                    .collect();
                (GeomType::Point, encode_multipoint(&pts))
            }
            "MultiLineString" => {
                let mut all = Vec::new();
                if let Some(lines) = coords.as_array() {
                    for line in lines {
                        let pts: Vec<(u32, u32)> = line.as_array()
                            .unwrap_or(&vec![])
                            .iter()
                            .filter_map(|c| {
                                Some(to_tile(c[0].as_f64()?, c[1].as_f64()?))
                            })
                            .collect();
                        if !pts.is_empty() {
                            all.push(pts);
                        }
                    }
                }
                (GeomType::Linestring, encode_multiline(&all))
            }
            "MultiPolygon" => {
                let mut all_rings = Vec::new();
                if let Some(polys) = coords.as_array() {
                    for poly in polys {
                        let rings: Vec<Vec<(u32, u32)>> = poly.as_array()
                            .unwrap_or(&vec![])
                            .iter()
                            .map(|ring| {
                                ring.as_array()
                                    .unwrap_or(&vec![])
                                    .iter()
                                    .filter_map(|c| {
                                        Some(to_tile(c[0].as_f64()?, c[1].as_f64()?))
                                    })
                                    .collect()
                            })
                            .collect();
                        all_rings.extend(rings);
                    }
                }
                (GeomType::Polygon, encode_polygon(&all_rings))
            }
            _ => {
                return Err(GeoError::Validation(format!("unsupported geometry type: {gtype}")));
            }
        };

        // 属性
        let mut tags = Vec::new();
        if let Some(obj) = props.as_object() {
            for (k, v) in obj {
                let mvt_val = json_to_mvt_value(v);
                tags.push((k.clone(), mvt_val));
            }
        }

        Ok(MvtFeature {
            id: None,
            tags,
            geom_type,
            geometry: commands,
        })
    }

    /// 将 GeoJSON FeatureCollection 编码为 MVT 字节。
    pub fn encode_tile(
        &self,
        layer_name: &str,
        features: &[Value],
        tile_x: u32,
        tile_y: u32,
        zoom: u8,
    ) -> GeoResult<Vec<u8>> {
        let mut mvt_features = Vec::with_capacity(features.len());
        for f in features {
            mvt_features.push(self.feature_from_geojson(f, tile_x, tile_y, zoom)?);
        }

        let layer = MvtLayer {
            name: layer_name.to_string(),
            extent: self.extent,
            features: mvt_features,
        };

        self.encode(&[layer])
    }

    /// 编码多个 MvtLayer 到 protobuf 字节。
    pub fn encode(&self, layers: &[MvtLayer]) -> GeoResult<Vec<u8>> {
        let mut buf = Vec::new();

        for layer in layers {
            let mut layer_buf = Vec::new();

            // field 1: name (string)
            write_string(&mut layer_buf, 1, &layer.name);

            // field 15: version = 2
            write_uint32(&mut layer_buf, 15, 2);

            // field 5: extent
            write_uint32(&mut layer_buf, 5, layer.extent);

            // 构建 keys/values 表（去重）
            let (keys, values) = build_value_tables(&layer.features);

            // field 3: keys
            for k in &keys {
                write_string(&mut layer_buf, 3, k);
            }

            // field 4: values
            for v in &values {
                encode_mvt_value(&mut layer_buf, 4, v);
            }

            // field 2: features
            for feat in &layer.features {
                let mut feat_buf = Vec::new();

                // id (optional)
                if let Some(id) = feat.id {
                    write_varint(&mut feat_buf, (1 << 3) as u64); // field 1, varint
                    write_varint(&mut feat_buf, id);
                }

                // tags (field 2, packed)
                let mut tag_ints = Vec::new();
                for (k, v) in &feat.tags {
                    let ki = keys.iter().position(|x| x == k)
                        .map(|i| i as u32).unwrap_or(0);
                    let vi = values.iter().position(|x| x == v)
                        .map(|i| i as u32).unwrap_or(0);
                    tag_ints.push(ki);
                    tag_ints.push(vi);
                }
                if !tag_ints.is_empty() {
                    write_packed_uint32(&mut feat_buf, 2, &tag_ints);
                }

                // type (field 3, varint)
                write_uint32(&mut feat_buf, 3, feat.geom_type as u32);

                // geometry (field 4, packed)
                if !feat.geometry.is_empty() {
                    write_packed_uint32(&mut feat_buf, 4, &feat.geometry);
                }

                // 写入 layer 的 field 2 (length-delimited feature)
                write_bytes(&mut layer_buf, 2, &feat_buf);
            }

            // 写入 tile 的 field 3 (length-delimited layer)
            write_bytes(&mut buf, 3, &layer_buf);
        }

        Ok(buf)
    }
}

// ═══════════════════════════════════════════════════════════
// 几何编码算法
// ═══════════════════════════════════════════════════════════

const CMD_MOVE_TO: u32 = 1;
const CMD_LINE_TO: u32 = 2;
const CMD_CLOSE_PATH: u32 = 7;

/// 编码 MoveTo + 坐标。
fn encode_move_to(x: u32, y: u32) -> Vec<u32> {
    vec![
        (CMD_MOVE_TO << 3) | 1,  // command_id=1, count=1
        zigzag(x as i32, 0),
        zigzag(y as i32, 0),
    ]
}

/// 编码 LineTo + 多个坐标（delta encoding）。
fn encode_line_to(points: &[(u32, u32)]) -> Vec<u32> {
    if points.is_empty() { return vec![]; }
    let mut cmds = vec![(CMD_LINE_TO << 3) | (points.len() as u32)];
    let mut prev_x = 0i32;
    let mut prev_y = 0i32;
    for &(x, y) in points {
        let ix = x as i32;
        let iy = y as i32;
        cmds.push(zigzag(ix, prev_x));
        cmds.push(zigzag(iy, prev_y));
        prev_x = ix;
        prev_y = iy;
    }
    cmds
}

/// 编码点。
fn encode_point(x: u32, y: u32) -> Vec<u32> {
    encode_move_to(x, y)
}

/// 编码线 (MoveTo + LineTo)。
fn encode_linestring(points: &[(u32, u32)]) -> Vec<u32> {
    if points.is_empty() { return vec![]; }
    let mut cmds = encode_move_to(points[0].0, points[0].1);
    cmds.extend(encode_line_to(&points[1..]));
    cmds
}

/// 编码多边形 (外环 MoveTo+LineTo+ClosePath，内环同理)。
fn encode_polygon(rings: &[Vec<(u32, u32)>]) -> Vec<u32> {
    let mut cmds = Vec::new();
    for ring in rings {
        if ring.is_empty() { continue; }
        cmds.extend(encode_move_to(ring[0].0, ring[0].1));
        cmds.extend(encode_line_to(&ring[1..]));
        cmds.push((CMD_CLOSE_PATH << 3) | 1);
    }
    cmds
}

/// 编码多点 (多个 MoveTo)。
fn encode_multipoint(points: &[(u32, u32)]) -> Vec<u32> {
    let mut cmds = vec![(CMD_MOVE_TO << 3) | (points.len() as u32)];
    let mut prev_x = 0i32;
    let mut prev_y = 0i32;
    for &(x, y) in points {
        let ix = x as i32;
        let iy = y as i32;
        cmds.push(zigzag(ix, prev_x));
        cmds.push(zigzag(iy, prev_y));
        prev_x = ix;
        prev_y = iy;
    }
    cmds
}

/// 编码多线 (每条线单独 MoveTo + LineTo)。
fn encode_multiline(lines: &[Vec<(u32, u32)>]) -> Vec<u32> {
    let mut cmds = Vec::new();
    for line in lines {
        cmds.extend(encode_linestring(line));
    }
    cmds
}

/// Zigzag delta 编码。
fn zigzag(current: i32, previous: i32) -> u32 {
    let delta = current - previous;
    ((delta << 1) ^ (delta >> 31)) as u32
}

// ═══════════════════════════════════════════════════════════
// 属性表编码
// ═══════════════════════════════════════════════════════════

/// 从所有 features 构建去重的 keys/values 表。
fn build_value_tables(features: &[MvtFeature]) -> (Vec<String>, Vec<MvtValue>) {
    let mut keys = Vec::new();
    let mut values = Vec::new();

    for feat in features {
        for (k, v) in &feat.tags {
            if !keys.contains(k) {
                keys.push(k.clone());
            }
            if !values.contains(v) {
                values.push(v.clone());
            }
        }
    }

    (keys, values)
}

/// 将 MvtValue 编码为 protobuf Value 消息。
fn encode_mvt_value(buf: &mut Vec<u8>, field_number: u32, value: &MvtValue) {
    let mut val_buf = Vec::new();
    match value {
        MvtValue::String(s)    => write_string(&mut val_buf, 1, s),
        MvtValue::Float(f)     => { write_tag(&mut val_buf, 2, WireType::Varint); write_varint(&mut val_buf, f.to_bits() as u64); }
        MvtValue::Double(d)    => { write_tag(&mut val_buf, 3, WireType::Varint); write_varint(&mut val_buf, d.to_bits()); }
        MvtValue::Int(i)       => { write_tag(&mut val_buf, 4, WireType::Varint); write_varint(&mut val_buf, *i as u64); }
        MvtValue::Uint(u)      => { write_tag(&mut val_buf, 5, WireType::Varint); write_varint(&mut val_buf, *u); }
        MvtValue::Sint(s)      => { write_tag(&mut val_buf, 6, WireType::Varint); write_varint(&mut val_buf, ((s << 1) ^ (s >> 63)) as u64); }
        MvtValue::Bool(b)      => { write_tag(&mut val_buf, 7, WireType::Varint); write_varint(&mut val_buf, if *b { 1 } else { 0 }); }
    }
    write_bytes(buf, field_number, &val_buf);
}

/// serde_json::Value → MvtValue。
fn json_to_mvt_value(v: &Value) -> MvtValue {
    match v {
        Value::String(s)  => MvtValue::String(s.clone()),
        Value::Number(n)  => {
            if let Some(i) = n.as_i64() { MvtValue::Sint(i) }
            else if let Some(f) = n.as_f64() { MvtValue::Double(f) }
            else { MvtValue::String(n.to_string()) }
        }
        Value::Bool(b)    => MvtValue::Bool(*b),
        Value::Null       => MvtValue::String("".into()),
        _                 => MvtValue::String(v.to_string()),
    }
}

// ═══════════════════════════════════════════════════════════
// 测试
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_point_mvt() {
        let encoder = MvtEncoder::new(4096);
        let features = vec![serde_json::json!({
            "type": "Feature",
            "properties": {"name": "test", "value": 42},
            "geometry": {"type": "Point", "coordinates": [104.06, 30.57]}
        })];
        let bytes = encoder.encode_tile("test", &features, 3270, 1671, 12).unwrap();
        assert!(!bytes.is_empty(), "MVT should produce bytes");
        // 应该以 protobuf 方式开头
        assert!(bytes.len() > 10, "MVT tile too small");
    }

    #[test]
    fn test_encode_polygon_mvt() {
        let encoder = MvtEncoder::new(4096);
        let features = vec![serde_json::json!({
            "type": "Feature",
            "properties": {"area": "zone"},
            "geometry": {
                "type": "Polygon",
                "coordinates": [[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]
            }
        })];
        let bytes = encoder.encode_tile("zones", &features, 3270, 1671, 12).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_varint_encoding() {
        let mut buf = Vec::new();
        write_varint(&mut buf, 1);
        assert_eq!(buf, vec![0x01]);

        buf.clear();
        write_varint(&mut buf, 300);
        assert_eq!(buf, vec![0xAC, 0x02]);
    }

    #[test]
    fn test_geometry_commands_point() {
        let cmds = encode_point(100, 200);
        // MoveTo(1), count=1, zigzag(100,0), zigzag(200,0)
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0], (1 << 3) | 1); // MoveTo, count=1
    }

    #[test]
    fn test_zigzag() {
        assert_eq!(zigzag(0, 0), 0);
        assert_eq!(zigzag(-1, 0), 1);
        assert_eq!(zigzag(1, 0), 2);
        assert_eq!(zigzag(-2, 0), 3);
    }
}
