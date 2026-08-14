/**
 * vector.js — 矢量几何运算。
 *
 * 依赖：ctx.wasmModule()（raw wasm 模块的 computeBuffer/computeIntersect/
 *       unionAll/computeArea/computeBbox/computeCentroid/simplifyGeometry/
 *       convexHull 自由函数）。
 * 职责：缓冲区、交/并、面积、包围盒、质心、化简、凸包等空间分析。
 *
 * ctx 约定见 crs.js 顶部注释。
 */


/** 将 GeoJSON 对象或字符串统一为 JSON 字符串。 */
function toStr(geojson) {
  return typeof geojson === 'string' ? geojson : JSON.stringify(geojson);
}


/**
 * Buffer a polygon geometry.
 * @param {object} ctx
 * @param {object|string} geojson   GeoJSON geometry or string
 * @param {number} distance         buffer distance (degrees)
 * @param {Object} [opts]
 * @param {string} [opts.mode='precise']  'bbox' | 'convex_hull' | 'precise'
 * @param {number} [opts.quadrantSegments=8]
 * @returns {object} GeoJSON MultiPolygon
 */
export function computeBuffer(ctx, geojson, distance, opts = {}) {
  ctx.assertReady();
  const geomStr = toStr(geojson);
  const mode = opts.mode || 'precise';
  const qs = opts.quadrantSegments ?? 8;
  const wasmMod = ctx.wasmModule();
  const json = wasmMod.computeBuffer(geomStr, distance, mode, qs);
  return JSON.parse(json);
}


/**
 * Compute intersection of two polygons.
 * @param {object} ctx
 * @param {object|string} a  GeoJSON Polygon
 * @param {object|string} b  GeoJSON Polygon
 * @returns {object|null} GeoJSON MultiPolygon or null
 */
export function computeIntersect(ctx, a, b) {
  ctx.assertReady();
  const aStr = toStr(a);
  const bStr = toStr(b);
  const wasmMod = ctx.wasmModule();
  const result = wasmMod.computeIntersect(aStr, bStr);
  if (result === 'null' || result === null) return null;
  return JSON.parse(result);
}


/**
 * Compute union of polygon array.
 * @param {object} ctx
 * @param {object[]|string[]} polygons  array of GeoJSON Polygons or strings
 * @returns {object} GeoJSON MultiPolygon
 */
export function unionAll(ctx, polygons) {
  ctx.assertReady();
  if (!Array.isArray(polygons) || polygons.length === 0) {
    throw new Error('unionAll: requires array of GeoJSON Polygons');
  }
  const arr = polygons.map(p => typeof p === 'string' ? JSON.parse(p) : p);
  const json = JSON.stringify(arr);
  const wasmMod = ctx.wasmModule();
  const result = wasmMod.unionAll(json);
  return JSON.parse(result);
}


/**
 * Compute area of a GeoJSON geometry (hectares).
 * @param {object} ctx
 * @param {object|string} geojson  GeoJSON geometry
 * @returns {number} area in hectares
 */
export function computeArea(ctx, geojson) {
  ctx.assertReady();
  const geomStr = toStr(geojson);
  const wasmMod = ctx.wasmModule();
  const json = wasmMod.computeArea(geomStr);
  const result = JSON.parse(json);
  return result.area_ha ?? result.area ?? 0;
}


/**
 * Compute bounding box of a GeoJSON geometry.
 * @param {object} ctx
 * @param {object|string} geojson
 * @returns {{minLon:number,minLat:number,maxLon:number,maxLat:number}}
 */
export function computeBbox(ctx, geojson) {
  ctx.assertReady();
  const geomStr = toStr(geojson);
  const wasmMod = ctx.wasmModule();
  const json = wasmMod.computeBbox(geomStr);
  return JSON.parse(json);
}


/**
 * Compute centroid of a GeoJSON geometry.
 * @param {object} ctx
 * @param {object|string} geojson
 * @returns {{lat:number,lon:number}}
 */
export function computeCentroid(ctx, geojson) {
  ctx.assertReady();
  const geomStr = toStr(geojson);
  const wasmMod = ctx.wasmModule();
  const json = wasmMod.computeCentroid(geomStr);
  return JSON.parse(json);
}


/**
 * Simplify a geometry using Douglas-Peucker.
 * @param {object} ctx
 * @param {object|string} geojson
 * @param {number} [epsilon=0.001]  simplification tolerance
 * @returns {object} simplified GeoJSON
 */
export function simplify(ctx, geojson, epsilon = 0.001) {
  ctx.assertReady();
  const geomStr = toStr(geojson);
  if (epsilon <= 0) throw new Error('simplify: epsilon must be > 0');
  const wasmMod = ctx.wasmModule();
  const json = wasmMod.simplifyGeometry(geomStr, epsilon);
  return JSON.parse(json);
}


/**
 * Compute convex hull of a geometry.
 * @param {object} ctx
 * @param {object|string} geojson
 * @returns {object} convex hull GeoJSON
 */
export function convexHull(ctx, geojson) {
  ctx.assertReady();
  const geomStr = toStr(geojson);
  const wasmMod = ctx.wasmModule();
  const json = wasmMod.convexHull(geomStr);
  return JSON.parse(json);
}
