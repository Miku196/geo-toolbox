/**
 * geohash.js — Geohash 编解码与邻域查询。
 *
 * 依赖：ctx.wasmModule()（raw wasm 模块的 geohashEncode/geohashDecode/
 *       geohashNeighbors/bboxToGeohashes 自由函数）。
 * 职责：经纬度 ↔ geohash 互转、8 邻域、bbox 覆盖 geohash 集合。
 *
 * ctx 约定见 crs.js 顶部注释。
 */


const GEOHASH_RE = /^[0-9bcdefghjkmnpqrstuvwxyz]+$/i;


/**
 * Encode a (lon, lat) pair into a geohash string.
 * @param {object} ctx
 * @param {number} lon       longitude (-180..180)
 * @param {number} lat       latitude (-90..90)
 * @param {number} [precision=12]  geohash precision (1-12)
 * @returns {string} geohash
 */
export function geohashEncode(ctx, lon, lat, precision = 12) {
  if (Math.abs(lat) > 90 || Math.abs(lon) > 180) {
    throw new Error(`geohashEncode: invalid coordinates (${lon}, ${lat})`);
  }
  if (precision < 1 || precision > 12) {
    throw new Error('geohashEncode: precision must be 1-12');
  }
  ctx.assertReady();
  const wasmMod = ctx.wasmModule();
  return wasmMod.geohashEncode(lon, lat, precision);
}


/**
 * Decode a geohash into center coordinate and bounding box.
 * @param {object} ctx
 * @param {string} hash  geohash string
 * @returns {{lat:number,lon:number,bbox:{minLon:number,minLat:number,maxLon:number,maxLat:number}}}
 */
export function geohashDecode(ctx, hash) {
  if (!hash || !GEOHASH_RE.test(hash)) {
    throw new Error(`geohashDecode: invalid hash "${hash}"`);
  }
  ctx.assertReady();
  const wasmMod = ctx.wasmModule();
  const json = wasmMod.geohashDecode(hash);
  return JSON.parse(json);
}


/**
 * Get all 8 neighbors of a geohash.
 * @param {object} ctx
 * @param {string} hash  geohash string
 * @returns {string[]} neighbor hashes
 */
export function geohashNeighbors(ctx, hash) {
  if (!hash || !GEOHASH_RE.test(hash)) {
    throw new Error(`geohashNeighbors: invalid hash "${hash}"`);
  }
  ctx.assertReady();
  const wasmMod = ctx.wasmModule();
  const json = wasmMod.geohashNeighbors(hash);
  return JSON.parse(json);
}


/**
 * Get all geohashes that intersect a bounding box.
 * @param {object} ctx
 * @param {{west:number,south:number,east:number,north:number}|number[]} bbox
 * @param {number} [precision=6]
 * @returns {string[]} geohash array
 */
export function bboxToGeohashes(ctx, bbox, precision = 6) {
  const [minLon, minLat, maxLon, maxLat] = Array.isArray(bbox)
    ? bbox
    : [bbox.west, bbox.south, bbox.east, bbox.north];
  if (minLat == null || maxLat == null || minLon == null || maxLon == null) {
    throw new Error('bboxToGeohashes: invalid bbox');
  }
  if (minLat < -90 || maxLat > 90 || minLon < -180 || maxLon > 180) {
    throw new Error('bboxToGeohashes: coordinates out of range');
  }
  if (precision < 1 || precision > 12) {
    throw new Error('bboxToGeohashes: precision must be 1-12');
  }
  ctx.assertReady();
  const wasmMod = ctx.wasmModule();
  const json = wasmMod.bboxToGeohashes(minLon, minLat, maxLon, maxLat, precision);
  return JSON.parse(json);
}
