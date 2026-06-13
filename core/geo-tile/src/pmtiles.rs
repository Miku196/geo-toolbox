//! PMTiles v3 读写器。
//!
//! PMTiles 是 Protomaps 提出的单一文件栅格瓦片归档格式。
//! 将百万级瓦片打包成一个文件，支持 HTTP Range 请求按需拉取。
//!
//! ## 格式 (v3)
//!
//! ```text
//! [Header 127B] [Root Dir] [Leaf Dirs...] [Tile Data...]
//! ```
//!
//! ## 参考
//!
//! <https://github.com/protomaps/PMTiles/blob/main/spec/v3/spec.md>

use geo_core::errors::{GeoError, GeoResult};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};

/// PMTiles Header (127 bytes, v3).
#[derive(Debug, Clone)]
pub struct PmtilesHeader {
    /// Magic: "PM"
    pub magic: [u8; 2],
    /// Version: 3
    pub version: u8,
    /// Tile type.
    pub tile_type: TileType,
    /// Min zoom level.
    pub min_zoom: u8,
    /// Max zoom level.
    pub max_zoom: u8,
    /// Min lat/lon position.
    pub min_lat: f32,
    pub min_lon: f32,
    /// Max lat/lon position.
    pub max_lat: f32,
    pub max_lon: f32,
    /// Center zoom.
    pub center_zoom: u8,
    /// Center lat/lon.
    pub center_lat: f32,
    pub center_lon: f32,
    /// Number of tiles.
    pub num_tiles: u64,
    /// Number of unique tile values.
    pub num_unique_tiles: u64,
    /// Clustered or not.
    pub clustered: bool,
    /// Internal compression.
    pub tile_compression: Compression,
}

/// Tile type enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileType {
    /// Unknown / unset.
    Unknown = 0,
    /// MVT (Mapbox Vector Tile).
    Mvt = 1,
    /// PNG.
    Png = 2,
    /// JPEG.
    Jpeg = 3,
    /// WebP.
    Webp = 4,
    /// AVIF.
    Avif = 5,
}

/// Compression type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    /// No compression.
    None = 0,
    /// Gzip.
    Gzip = 1,
    /// Brotli.
    Brotli = 2,
    /// Zstd.
    Zstd = 3,
}

/// 瓦片条目。
#[derive(Debug, Clone)]
pub struct TileEntry {
    /// z/x/y
    pub z: u8,
    pub x: u32,
    pub y: u32,
    /// 文件偏移（绝对字节位置）。
    pub offset: u64,
    /// 瓦片数据长度。
    pub length: u32,
}

/// PMTiles v3 读取器。
pub struct PmtilesReader<R: Read + Seek> {
    reader: R,
    /// PMTiles 文件头信息。
    pub header: PmtilesHeader,
    /// 内存中的瓦片索引 (z, x, y) → (offset, length)。
    tile_index: HashMap<(u8, u32, u32), (u64, u32)>,
}

impl<R: Read + Seek> PmtilesReader<R> {
    /// 从实现了 Read + Seek 的源（如 File）打开 PMTiles。
    pub fn open(mut reader: R) -> GeoResult<Self> {
        let header = read_header(&mut reader)?;
        if header.magic != *b"PM" || header.version != 3 {
            return Err(GeoError::Validation(format!(
                "Invalid PMTiles: magic={:?} version={}",
                header.magic, header.version
            )));
        }

        // 读取 root directory（紧跟 header 之后）
        // 每个 entry 最多约 25 字节 (varint encoded)
        let buf_size = (header.num_tiles as usize * 30).max(1024);
        let mut root_buf = vec![0u8; buf_size];
        reader.seek(SeekFrom::Start(127))?;
        let n = reader.read(&mut root_buf)?;
        root_buf.truncate(n);

        let entries = parse_directory_entries(&root_buf, header.num_tiles)?;
        let tile_index: HashMap<(u8, u32, u32), (u64, u32)> = entries
            .into_iter()
            .map(|e| ((e.z, e.x, e.y), (e.offset, e.length)))
            .collect();

        Ok(Self {
            reader,
            header,
            tile_index,
        })
    }

    /// 读取指定瓦片的数据。
    pub fn get_tile(&mut self, z: u8, x: u32, y: u32) -> GeoResult<Vec<u8>> {
        let &(offset, length) = self
            .tile_index
            .get(&(z, x, y))
            .ok_or_else(|| GeoError::not_found("tile", format!("{z}/{x}/{y}")))?;

        self.reader.seek(SeekFrom::Start(offset))?;
        let mut buf = vec![0u8; length as usize];
        self.reader.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// 列出所有可用瓦片。
    pub fn list_tiles(&self) -> Vec<(u8, u32, u32)> {
        self.tile_index.keys().copied().collect()
    }

    /// 瓦片总数。
    pub fn tile_count(&self) -> usize {
        self.tile_index.len()
    }
}

/// PMTiles v3 写入器。
pub struct PmtilesWriter<W: Write + Seek> {
    writer: W,
    header: PmtilesHeader,
    tiles: HashMap<(u8, u32, u32), Vec<u8>>,
}

impl<W: Write + Seek> PmtilesWriter<W> {
    /// 创建 PMTiles 写入器。
    pub fn new(writer: W, tile_type: TileType, compression: Compression) -> Self {
        Self {
            writer,
            header: PmtilesHeader {
                magic: *b"PM",
                version: 3,
                tile_type,
                min_zoom: 22,
                max_zoom: 0,
                min_lat: 90.0,
                min_lon: 180.0,
                max_lat: -90.0,
                max_lon: -180.0,
                center_zoom: 0,
                center_lat: 0.0,
                center_lon: 0.0,
                num_tiles: 0,
                num_unique_tiles: 0,
                clustered: false,
                tile_compression: compression,
            },
            tiles: HashMap::new(),
        }
    }

    /// 添加一个瓦片。
    pub fn add_tile(&mut self, z: u8, x: u32, y: u32, data: Vec<u8>) {
        // 更新 header 范围
        if z < self.header.min_zoom {
            self.header.min_zoom = z;
        }
        if z > self.header.max_zoom {
            self.header.max_zoom = z;
        }

        let (lon, lat) = crate::tile_index::tile_to_latlon(x, y, z);
        let (lat, lon) = (lat as f32, lon as f32);
        if lat < self.header.min_lat {
            self.header.min_lat = lat;
        }
        if lat > self.header.max_lat {
            self.header.max_lat = lat;
        }
        if lon < self.header.min_lon {
            self.header.min_lon = lon;
        }
        if lon > self.header.max_lon {
            self.header.max_lon = lon;
        }

        self.tiles.insert((z, x, y), data);
    }

    /// 添加栅格瓦片 (PNG/JPEG)。
    pub fn add_raster_tile(&mut self, z: u8, x: u32, y: u32, data: Vec<u8>) {
        self.add_tile(z, x, y, data);
    }

    /// 写入文件。
    ///
    /// 写入顺序：Header (127B) → Directory → Tile Data。
    pub fn finish(mut self) -> GeoResult<()> {
        self.header.num_tiles = self.tiles.len() as u64;

        // 按 z, x, y 排序
        let mut sorted: Vec<_> = self.tiles.into_iter().collect();
        sorted.sort_by_key(|&((z, x, y), _)| (z, x, y));

        // 计算每个 tile 的偏移（从当前 write position 开始）
        // Header = 127B，Directories 后续计算
        let dir_offset: u64 = 127;
        let mut data_offset = dir_offset;

        // 预留 directory 空间（每个 entry ~20B varint）
        let dir_est = sorted.len() * 25;
        data_offset += dir_est as u64;

        // 构建 entries
        let mut entries = Vec::with_capacity(sorted.len());
        let mut current_offset = data_offset;

        for ((z, x, y), data) in &sorted {
            entries.push(TileEntry {
                z: *z,
                x: *x,
                y: *y,
                offset: current_offset,
                length: data.len() as u32,
            });
            current_offset += data.len() as u64;
        }

        // 计算实际 directory 大小
        let mut dir_bytes = Vec::new();
        write_directory_entries(&mut dir_bytes, &entries)?;

        // 如果预估不准确，重新计算（以实际为准）
        let actual_dir_size = dir_bytes.len() as u64;
        let actual_data_offset = dir_offset + actual_dir_size;

        // 更新 offsets
        let mut final_entries = Vec::with_capacity(entries.len());
        current_offset = actual_data_offset;
        for ((z, x, y), data) in sorted.iter() {
            final_entries.push(TileEntry {
                z: *z,
                x: *x,
                y: *y,
                offset: current_offset,
                length: data.len() as u32,
            });
            current_offset += data.len() as u64;
        }

        // 重新编码 directory
        dir_bytes.clear();
        write_directory_entries(&mut dir_bytes, &final_entries)?;

        // 写入 Header
        let mut header_buf = vec![0u8; 127];
        header_buf[0..2].copy_from_slice(b"PM");
        header_buf[2] = 3;
        header_buf[3] = self.header.tile_type as u8;
        header_buf[4] = self.header.min_zoom;
        header_buf[5] = self.header.max_zoom;
        write_f32_le(&mut header_buf[10..14], self.header.min_lat);
        write_f32_le(&mut header_buf[14..18], self.header.min_lon);
        write_f32_le(&mut header_buf[18..22], self.header.max_lat);
        write_f32_le(&mut header_buf[22..26], self.header.max_lon);
        header_buf[26] = self.header.center_zoom;
        write_f32_le(&mut header_buf[27..31], self.header.center_lat);
        write_f32_le(&mut header_buf[31..35], self.header.center_lon);
        // num_tiles (u64 at offset 35)
        header_buf[35..43].copy_from_slice(&self.header.num_tiles.to_le_bytes());
        header_buf[53] = if self.header.clustered { 1 } else { 0 };
        header_buf[54] = self.header.tile_compression as u8;

        self.writer.write_all(&header_buf)?;

        // 写入 Directory
        self.writer.write_all(&dir_bytes)?;

        // 写入 Tile Data
        for ((_, _, _), data) in sorted {
            self.writer.write_all(&data)?;
        }

        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════
// 内部函数
// ═══════════════════════════════════════════════════════════

fn write_f32_le(buf: &mut [u8], v: f32) {
    buf.copy_from_slice(&v.to_le_bytes());
}

/// 读取 127 字节 Header。
fn read_header<R: Read>(reader: &mut R) -> GeoResult<PmtilesHeader> {
    let mut buf = [0u8; 127];
    reader.read_exact(&mut buf)?;

    Ok(PmtilesHeader {
        magic: [buf[0], buf[1]],
        version: buf[2],
        tile_type: match buf[3] {
            1 => TileType::Mvt,
            2 => TileType::Png,
            3 => TileType::Jpeg,
            4 => TileType::Webp,
            5 => TileType::Avif,
            _ => TileType::Unknown,
        },
        min_zoom: buf[4],
        max_zoom: buf[5],
        min_lat: f32::from_le_bytes([buf[10], buf[11], buf[12], buf[13]]),
        min_lon: f32::from_le_bytes([buf[14], buf[15], buf[16], buf[17]]),
        max_lat: f32::from_le_bytes([buf[18], buf[19], buf[20], buf[21]]),
        max_lon: f32::from_le_bytes([buf[22], buf[23], buf[24], buf[25]]),
        center_zoom: buf[26],
        center_lat: f32::from_le_bytes([buf[27], buf[28], buf[29], buf[30]]),
        center_lon: f32::from_le_bytes([buf[31], buf[32], buf[33], buf[34]]),
        num_tiles: u64::from_le_bytes(buf[35..43].try_into().unwrap()),
        num_unique_tiles: 0,
        clustered: buf[53] != 0,
        tile_compression: match buf[54] {
            1 => Compression::Gzip,
            2 => Compression::Brotli,
            3 => Compression::Zstd,
            _ => Compression::None,
        },
    })
}

/// 从字节流解析 directory entries。
fn parse_directory_entries(data: &[u8], expected_count: u64) -> GeoResult<Vec<TileEntry>> {
    let mut entries = Vec::with_capacity(expected_count as usize);
    let mut pos = 0usize;

    while entries.len() < expected_count as usize && pos + 1 < data.len() {
        // 读取 varint 编码的条目
        let (z, adv) = read_varint_u8(&data[pos..])?;
        pos += adv;
        let (x, adv) = read_varint_u32(&data[pos..])?;
        pos += adv;
        let (y, adv) = read_varint_u32(&data[pos..])?;
        pos += adv;
        let (offset, adv) = read_varint_u64(&data[pos..])?;
        pos += adv;
        let (length, adv) = read_varint_u32(&data[pos..])?;
        pos += adv;

        entries.push(TileEntry {
            z,
            x,
            y,
            offset,
            length,
        });
    }

    Ok(entries)
}

/// 写入 directory entries 到 buffer。
fn write_directory_entries(buf: &mut Vec<u8>, entries: &[TileEntry]) -> GeoResult<()> {
    for e in entries {
        write_varint(buf, e.z as u64);
        write_varint(buf, e.x as u64);
        write_varint(buf, e.y as u64);
        write_varint(buf, e.offset);
        write_varint(buf, e.length as u64);
    }
    Ok(())
}

// ── varint helpers ──

fn write_varint(buf: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        buf.push((v as u8 & 0x7F) | 0x80);
        v >>= 7;
    }
    buf.push(v as u8);
}

fn read_varint_u8(data: &[u8]) -> GeoResult<(u8, usize)> {
    let (v, adv) = read_varint(data)?;
    Ok((v as u8, adv))
}

fn read_varint_u32(data: &[u8]) -> GeoResult<(u32, usize)> {
    let (v, adv) = read_varint(data)?;
    Ok((v as u32, adv))
}

fn read_varint_u64(data: &[u8]) -> GeoResult<(u64, usize)> {
    read_varint(data)
}

fn read_varint(data: &[u8]) -> GeoResult<(u64, usize)> {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    for (i, &byte) in data.iter().enumerate() {
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            return Ok((result, i + 1));
        }
        shift += 7;
        if shift >= 64 {
            return Err(GeoError::Validation("varint too long".into()));
        }
    }
    Err(GeoError::Validation("truncated varint".into()))
}

// ═══════════════════════════════════════════════════════════
// 测试
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pmtiles_write_and_read() {
        let mut buf = Vec::new();
        let cursor = std::io::Cursor::new(&mut buf);

        let mut writer = PmtilesWriter::new(cursor, TileType::Mvt, Compression::None);
        writer.add_tile(0, 0, 0, vec![1, 2, 3]);
        writer.add_tile(1, 0, 0, vec![4, 5]);
        writer.add_tile(1, 1, 0, vec![6]);
        writer.finish().unwrap();

        assert!(!buf.is_empty());

        // 读回
        let reader = std::io::Cursor::new(buf);
        let mut pm = PmtilesReader::open(reader).unwrap();
        assert_eq!(pm.header.tile_type, TileType::Mvt);
        assert_eq!(pm.tile_count(), 3);

        let tile = pm.get_tile(0, 0, 0).unwrap();
        assert_eq!(tile, vec![1, 2, 3]);

        let tile = pm.get_tile(1, 0, 0).unwrap();
        assert_eq!(tile, vec![4, 5]);
    }

    #[test]
    fn test_varint_roundtrip() {
        let mut buf = Vec::new();
        write_varint(&mut buf, 300);
        let (val, adv) = read_varint(&buf).unwrap();
        assert_eq!(val, 300);
        assert_eq!(adv, 2);
    }

    #[test]
    fn test_bad_magic() {
        let buf = vec![0u8; 127];
        let reader = std::io::Cursor::new(buf);
        assert!(PmtilesReader::open(reader).is_err());
    }
}
