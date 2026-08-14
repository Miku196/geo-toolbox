/**
 * GeoMaplibrePlugin — browser-side geospatial analysis for MapLibre GL JS.
 *
 * 入口 & 组装层：本文件仅负责 GeoMaplibrePlugin 类的骨架（私有状态、生命周期、
 * 引擎访问器）与对外 API 的委托，具体功能实现拆分到同目录的功能模块：
 *
 *   wasm-loader.js  WASM 模块加载 + 引擎实例化
 *   crs.js          CRS 重投影（transformLayer）
 *   carbon.js       碳汇核算（computeCarbonSink）
 *   ndvi.js         NDVI 计算（computeNdvi / ndviDifference）
 *   geohash.js      Geohash 编解码 / 邻域 / bbox 覆盖
 *   vector.js       矢量几何运算（buffer/intersect/union/area/bbox/centroid/simplify/hull）
 *   raster.js       栅格波段运算（bandMath/threshold/resample/zonalStats）
 *   tile.js         MVT 切片编码与图层注册（addMvtSource / addFillLayer）
 *   drawing.js      手绘多边形 AOI 交互（enableDrawing / disableDrawing）
 *
 * 对外 API 完全不变：类名、方法名、事件名、构造函数签名与原单文件版一致。
 *
 * @example
 *   import { GeoMaplibrePlugin } from 'maplibre-gl-geo-toolbox';
 *   const geo = new GeoMaplibrePlugin(map);
 *   await geo.init();
 *   geo.enableDrawing();
 *   map.on('aoi-drawn', async (e) => {
 *     const result = await geo.computeCarbonSink(e.aoi, { landcover: 'forest' });
 *     console.log(result);
 *   });
 */

import { loadWasmEngine } from './wasm-loader.js';
import { transformLayer } from './crs.js';
import { computeCarbonSink } from './carbon.js';
import { computeNdvi, ndviDifference } from './ndvi.js';
import { geohashEncode, geohashDecode, geohashNeighbors, bboxToGeohashes } from './geohash.js';
import {
  computeBuffer, computeIntersect, unionAll, computeArea,
  computeBbox, computeCentroid, simplify, convexHull,
} from './vector.js';
import { bandMath, bandThreshold, resample, computeZonalStats } from './raster.js';
import { addMvtSource, addFillLayer } from './tile.js';
import { enableDrawing, disableDrawing } from './drawing.js';


export class GeoMaplibrePlugin {
  #map;
  #engine;
  #_wasm;
  #drawing = null;
  #wasmLoaded = false;
  #wasmPath;
  #ctx;

  /**
   * @param {maplibregl.Map} map  MapLibre map instance
   * @param {Object} [options]
   * @param {string} [options.wasmPath='./pkg/geo_wasm.js']  path to wasm-pack JS glue
   */
  constructor(map, options = {}) {
    this.#map = map;
    this.#wasmPath = options.wasmPath || './pkg/geo_wasm.js';
    this.#engine = null;

    // 功能模块共享的上下文：把私有状态/助手以只读视图暴露给各模块函数。
    const self = this;
    this.#ctx = {
      get map()       { return self.#map; },
      get engine()    { return self.#engine; },
      get wasm()      { return self.#_wasm; },
      get wasmLoaded(){ return self.#wasmLoaded; },
      get drawing()   { return self.#drawing; },
      set drawing(v)  { self.#drawing = v; },
      assertReady: () => self.#assertReady(),
      wasmModule: () => self.#_wasmModule(),
    };
  }

  // -----------------------------------------------------------------------
  //  Lifecycle
  // -----------------------------------------------------------------------

  /** Load the WASM engine. Call once before using compute* methods. */
  async init() {
    if (this.#wasmLoaded) return;

    const { wasm, engine } = await loadWasmEngine(this.#wasmPath);
    this.#_wasm = wasm;  // raw module for free functions
    this.#engine = engine;
    this.#wasmLoaded = true;
  }

  /** True once init() succeeded. */
  get ready() { return this.#wasmLoaded; }

  // -----------------------------------------------------------------------
  //  Engine accessors
  // -----------------------------------------------------------------------

  /** @returns {import('./pkg/geo_wasm').CrsEngine} */
  getCrsEngine() { this.#assertReady(); return this.#engine.crs; }
  /** @returns {import('./pkg/geo_wasm').CarbonEngine} */
  getCarbonEngine() { this.#assertReady(); return this.#engine.carbon; }
  /** @returns {import('./pkg/geo_wasm').TileEngine} */
  getTileEngine() { this.#assertReady(); return this.#engine.tile; }

  // -----------------------------------------------------------------------
  //  CRS transform
  // -----------------------------------------------------------------------

  /**
   * Reproject a vector source from one CRS to another.
   * @param {string} sourceId  MapLibre source id
   * @param {{from: number, to: number}} crs  EPSG codes
   */
  async transformLayer(sourceId, { from, to }) {
    await transformLayer(this.#ctx, sourceId, { from, to });
  }

  // -----------------------------------------------------------------------
  //  NDVI
  // -----------------------------------------------------------------------

  async computeNdvi(red, nir, opts = {}) {
    return computeNdvi(this.#ctx, red, nir, opts);
  }

  ndviDifference(prev, prevRows, prevCols, curr, currRows, currCols) {
    return ndviDifference(this.#ctx, prev, prevRows, prevCols, curr, currRows, currCols);
  }

  // -----------------------------------------------------------------------
  //  Carbon sink
  // -----------------------------------------------------------------------

  async computeCarbonSink(aoi, params = {}) {
    return computeCarbonSink(this.#ctx, aoi, params);
  }

  // -----------------------------------------------------------------------
  //  Geohash
  // -----------------------------------------------------------------------

  geohashEncode(lon, lat, precision = 12) {
    return geohashEncode(this.#ctx, lon, lat, precision);
  }

  geohashDecode(hash) {
    return geohashDecode(this.#ctx, hash);
  }

  geohashNeighbors(hash) {
    return geohashNeighbors(this.#ctx, hash);
  }

  bboxToGeohashes(bbox, precision = 6) {
    return bboxToGeohashes(this.#ctx, bbox, precision);
  }

  // -----------------------------------------------------------------------
  //  Vector ops
  // -----------------------------------------------------------------------

  computeBuffer(geojson, distance, opts = {}) {
    return computeBuffer(this.#ctx, geojson, distance, opts);
  }

  computeIntersect(a, b) {
    return computeIntersect(this.#ctx, a, b);
  }

  unionAll(polygons) {
    return unionAll(this.#ctx, polygons);
  }

  // -----------------------------------------------------------------------
  //  Spatial analysis
  // -----------------------------------------------------------------------

  computeArea(geojson) {
    return computeArea(this.#ctx, geojson);
  }

  computeBbox(geojson) {
    return computeBbox(this.#ctx, geojson);
  }

  computeCentroid(geojson) {
    return computeCentroid(this.#ctx, geojson);
  }

  simplify(geojson, epsilon = 0.001) {
    return simplify(this.#ctx, geojson, epsilon);
  }

  convexHull(geojson) {
    return convexHull(this.#ctx, geojson);
  }

  // -----------------------------------------------------------------------
  //  Raster ops
  // -----------------------------------------------------------------------

  bandMath(a, b, opts = {}) {
    return bandMath(this.#ctx, a, b, opts);
  }

  bandThreshold(data, rows, cols, threshold) {
    return bandThreshold(this.#ctx, data, rows, cols, threshold);
  }

  resample(data, srcRows, srcCols, dstRows, dstCols, opts = {}) {
    return resample(this.#ctx, data, srcRows, srcCols, dstRows, dstCols, opts);
  }

  computeZonalStats(values, zones, numZones, nodata = null) {
    return computeZonalStats(this.#ctx, values, zones, numZones, nodata);
  }

  // -----------------------------------------------------------------------
  //  Map tile encoding
  // -----------------------------------------------------------------------

  async addMvtSource(sourceId, geojson, maxZoom = 14) {
    return addMvtSource(this.#ctx, sourceId, geojson, maxZoom);
  }

  addFillLayer(sourceId, layerId) {
    return addFillLayer(this.#ctx, sourceId, layerId);
  }

  // -----------------------------------------------------------------------
  //  Drawing
  // -----------------------------------------------------------------------

  enableDrawing() {
    return enableDrawing(this.#ctx);
  }

  disableDrawing() {
    return disableDrawing(this.#ctx);
  }

  // -----------------------------------------------------------------------
  //  Private helpers
  // -----------------------------------------------------------------------

  #assertReady() {
    if (!this.#wasmLoaded || !this.#engine) {
      throw new Error('geo-toolbox: WASM not loaded. Call await geo.init() first.');
    }
  }

  /** Lazy-load the WASM module singleton for direct function calls. */
  #_wasmModule() {
    this.#assertReady();
    if (!this.#_wasm) {
      throw new Error('geo-toolbox: WASM module not bound — use init() first. Direct module access requires import() pattern.');
    }
    return this.#_wasm;
  }
}
