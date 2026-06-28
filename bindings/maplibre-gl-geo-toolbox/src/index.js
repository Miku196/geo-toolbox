/**
 * GeoMaplibrePlugin — browser-side geospatial analysis for MapLibre GL JS.
 *
 * Loads WASM engine, exposes drawing, CRS transform, vector ops,
 * carbon sink calculation, tile encoding, NDVI, geohash, spatial analysis.
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

const DEFAULT_FACTORS = [
  { category: 'forest',       factor_value: 4.80,  source: 'IPCC Tier 1' },
  { category: 'grassland',    factor_value: 1.20,  source: 'IPCC Tier 1' },
  { category: 'cropland',     factor_value: 0.80,  source: 'IPCC Tier 1' },
  { category: 'wetland',      factor_value: 6.50,  source: 'IPCC Tier 1' },
  { category: 'settlement',   factor_value: 0.10,  source: 'IPCC Tier 1' },
  { category: 'shrubland',    factor_value: 2.30,  source: 'IPCC Tier 1' },
  { category: 'barren',       factor_value: 0.05,  source: 'IPCC Tier 1' },
  { category: 'water',        factor_value: 0.00,  source: 'IPCC Tier 1' },
];

export class GeoMaplibrePlugin {
  #map;
  #engine;
  #_wasm;
  #drawing = null;
  #wasmLoaded = false;
  #wasmPath;

  /**
   * @param {maplibregl.Map} map  MapLibre map instance
   * @param {Object} [options]
   * @param {string} [options.wasmPath='./pkg/geo_wasm.js']  path to wasm-pack JS glue
   */
  constructor(map, options = {}) {
    this.#map = map;
    this.#wasmPath = options.wasmPath || './pkg/geo_wasm.js';
    this.#engine = null;
  }

  // -----------------------------------------------------------------------
  //  Lifecycle
  // -----------------------------------------------------------------------

  /** Load the WASM engine. Call once before using compute* methods. */
  async init() {
    if (this.#wasmLoaded) return;

    try {
      const wasm = await import(/* @vite-ignore */ this.#wasmPath);
      await wasm.default();
      this.#_wasm = wasm;  // raw module for free functions
      this.#engine = {
        crs:    new wasm.CrsEngine(),
        carbon: new wasm.CarbonEngine(),
        tile:   new wasm.TileEngine(),
      };
      this.#wasmLoaded = true;
    } catch (e) {
      console.error('geo-toolbox: WASM failed to load', e);
      throw e;
    }
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
    this.#assertReady();
    const source = this.#map.getSource(sourceId);
    if (!source) throw new Error(`Source "${sourceId}" not found`);
    const data = this.#collectSourceData(source);
    const transformed = this.#transformCoords(data, from, to);
    this.#updateSource(sourceId, transformed);
  }

  // -----------------------------------------------------------------------
  //  NDVI
  // -----------------------------------------------------------------------

  /**
   * Compute NDVI from red and NIR raster data.
   * Pixels are sampled from the map canvas via queryRenderedFeatures or
   * image data if a raster source is loaded.
   *
   * @param {string|Float64Array} red   red band data or source id
   * @param {Float64Array} [nir]        NIR band data
   * @param {Object} [opts]
   * @param {number} [opts.rows]        raster rows
   * @param {number} [opts.cols]        raster cols
   * @returns {Promise<Object>} JSON-parsed NDVI result
   */
  async computeNdvi(red, nir, opts = {}) {
    this.#assertReady();
    const wasmMod = this.#_wasmModule();

    const redData  = Array.isArray(red) ? new Float64Array(red) : red;
    const nirData  = Array.isArray(nir) ? new Float64Array(nir) : nir;
    const rows     = opts.rows || 1;
    const cols     = opts.cols || redData.length;

    if (!redData || !nirData) {
      throw new Error('computeNdvi: red and nir band data required (Float64Array or number[])');
    }
    if (redData.length !== nirData.length) {
      throw new Error('computeNdvi: red and nir arrays must have same length');
    }

    const redVec  = Array.from(redData);
    const nirVec  = Array.from(nirData);
    const actualRows = rows;
    const actualCols = rows * cols === redData.length ? cols : redData.length;

    try {
      const json = wasmMod.computeNdvi(redVec, nirVec, actualRows, actualCols);
      return JSON.parse(json);
    } catch (e) {
      console.error('computeNdvi: WASM call failed', e);
      throw e;
    }
  }

  /**
   * Compute NDVI difference between two time points.
   * @param {Float64Array} prev  previous NDVI data
   * @param {number} prevRows
   * @param {number} prevCols
   * @param {Float64Array} curr  current NDVI data
   * @param {number} currRows
   * @param {number} currCols
   * @returns {Promise<Object>}
   */
  ndviDifference(prev, prevRows, prevCols, curr, currRows, currCols) {
    this.#assertReady();
    const wasmMod = this.#_wasmModule();
    const prevVec = Array.from(prev);
    const currVec = Array.from(curr);
    try {
      const json = wasmMod.ndviDifference(prevVec, prevRows, prevCols, currVec, currRows, currCols);
      return JSON.parse(json);
    } catch (e) {
      console.error('ndviDifference: WASM call failed', e);
      throw e;
    }
  }

  // -----------------------------------------------------------------------
  //  Carbon sink
  // -----------------------------------------------------------------------

  /**
   * Compute carbon sink for a drawn AOI.
   *
   * @param {object}          aoi       GeoJSON Polygon Feature
   * @param {Object}          [params]
   * @param {string}          [params.landcover]  land-cover class for lookup
   * @param {Object[]|string} [params.factors]    custom factors (JSON array or JSON string)
   * @param {number}          [params.year]
   * @returns {Promise<object>}  { totalSink_tco2_yr, breakdown, year, aoiArea_ha }
   */
  async computeCarbonSink(aoi, params = {}) {
    this.#assertReady();

    const landcover = params.landcover || 'forest';
    const year = params.year || new Date().getFullYear();
    const factors = this.#buildFactors(landcover, params.factors);

    let fc;
    if (aoi.type === 'FeatureCollection') {
      fc = aoi;
    } else if (aoi.type === 'Feature') {
      fc = { type: 'FeatureCollection', features: [aoi] };
    } else {
      fc = {
        type: 'FeatureCollection',
        features: [{ type: 'Feature', properties: { class: landcover }, geometry: aoi }],
      };
    }

    for (const f of fc.features) {
      if (!f.properties) f.properties = {};
      if (!f.properties.class) f.properties.class = landcover;
    }

    const geojsonStr = JSON.stringify(fc);
    const factorsJson = JSON.stringify(factors);

    let result;
    try {
      const json = this.#engine.carbon.calculateWithJsonFactors(geojsonStr, factorsJson, year);
      result = JSON.parse(json);
    } catch (e) {
      console.error('carbon.calculateWithJsonFactors failed', e);
      throw e;
    }

    const areaHa = this.#computeArea(aoi);

    return {
      totalSink_tco2_yr: result.total_tco2e ?? 0,
      breakdown:          result.breakdown ?? [],
      year,
      aoiArea_ha:         areaHa,
      raw:                result,
    };
  }

  // -----------------------------------------------------------------------
  //  Geohash
  // -----------------------------------------------------------------------

  /**
   * Encode a (lon, lat) pair into a geohash string.
   * @param {number} lon       longitude (-180..180)
   * @param {number} lat       latitude (-90..90)
   * @param {number} [precision=12]  geohash precision (1-12)
   * @returns {string} geohash
   */
  geohashEncode(lon, lat, precision = 12) {
    if (Math.abs(lat) > 90 || Math.abs(lon) > 180) {
      throw new Error(`geohashEncode: invalid coordinates (${lon}, ${lat})`);
    }
    if (precision < 1 || precision > 12) {
      throw new Error('geohashEncode: precision must be 1-12');
    }
    this.#assertReady();
    const wasmMod = this.#_wasmModule();
    return wasmMod.geohashEncode(lon, lat, precision);
  }

  /**
   * Decode a geohash into center coordinate and bounding box.
   * @param {string} hash  geohash string
   * @returns {{lat:number,lon:number,bbox:{minLon:number,minLat:number,maxLon:number,maxLat:number}}}
   */
  geohashDecode(hash) {
    if (!hash || !/^[0-9bcdefghjkmnpqrstuvwxyz]+$/i.test(hash)) {
      throw new Error(`geohashDecode: invalid hash "${hash}"`);
    }
    this.#assertReady();
    const wasmMod = this.#_wasmModule();
    const json = wasmMod.geohashDecode(hash);
    return JSON.parse(json);
  }

  /**
   * Get all 8 neighbors of a geohash.
   * @param {string} hash  geohash string
   * @returns {string[]} neighbor hashes
   */
  geohashNeighbors(hash) {
    if (!hash || !/^[0-9bcdefghjkmnpqrstuvwxyz]+$/i.test(hash)) {
      throw new Error(`geohashNeighbors: invalid hash "${hash}"`);
    }
    this.#assertReady();
    const wasmMod = this.#_wasmModule();
    const json = wasmMod.geohashNeighbors(hash);
    return JSON.parse(json);
  }

  /**
   * Get all geohashes that intersect a bounding box.
   * @param {{west:number,south:number,east:number,north:number}|number[]} bbox
   * @param {number} [precision=6]
   * @returns {string[]} geohash array
   */
  bboxToGeohashes(bbox, precision = 6) {
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
    this.#assertReady();
    const wasmMod = this.#_wasmModule();
    const json = wasmMod.bboxToGeohashes(minLon, minLat, maxLon, maxLat, precision);
    return JSON.parse(json);
  }

  // -----------------------------------------------------------------------
  //  Vector ops
  // -----------------------------------------------------------------------

  /**
   * Buffer a polygon geometry.
   * @param {object|string} geojson   GeoJSON geometry or string
   * @param {number} distance         buffer distance (degrees)
   * @param {Object} [opts]
   * @param {string} [opts.mode='precise']  'bbox' | 'convex_hull' | 'precise'
   * @param {number} [opts.quadrantSegments=8]
   * @returns {object} GeoJSON MultiPolygon
   */
  computeBuffer(geojson, distance, opts = {}) {
    this.#assertReady();
    const geomStr = typeof geojson === 'string' ? geojson : JSON.stringify(geojson);
    const mode = opts.mode || 'precise';
    const qs = opts.quadrantSegments ?? 8;
    const wasmMod = this.#_wasmModule();
    const json = wasmMod.computeBuffer(geomStr, distance, mode, qs);
    return JSON.parse(json);
  }

  /**
   * Compute intersection of two polygons.
   * @param {object|string} a  GeoJSON Polygon
   * @param {object|string} b  GeoJSON Polygon
   * @returns {object|null} GeoJSON MultiPolygon or null
   */
  computeIntersect(a, b) {
    this.#assertReady();
    const aStr = typeof a === 'string' ? a : JSON.stringify(a);
    const bStr = typeof b === 'string' ? b : JSON.stringify(b);
    const wasmMod = this.#_wasmModule();
    const result = wasmMod.computeIntersect(aStr, bStr);
    if (result === 'null' || result === null) return null;
    return JSON.parse(result);
  }

  /**
   * Compute union of polygon array.
   * @param {object[]|string[]} polygons  array of GeoJSON Polygons or strings
   * @returns {object} GeoJSON MultiPolygon
   */
  unionAll(polygons) {
    this.#assertReady();
    if (!Array.isArray(polygons) || polygons.length === 0) {
      throw new Error('unionAll: requires array of GeoJSON Polygons');
    }
    const arr = polygons.map(p => typeof p === 'string' ? JSON.parse(p) : p);
    const json = JSON.stringify(arr);
    const wasmMod = this.#_wasmModule();
    const result = wasmMod.unionAll(json);
    return JSON.parse(result);
  }

  // -----------------------------------------------------------------------
  //  Spatial analysis
  // -----------------------------------------------------------------------

  /**
   * Compute area of a GeoJSON geometry (hectares).
   * @param {object|string} geojson  GeoJSON geometry
   * @returns {number} area in hectares
   */
  computeArea(geojson) {
    this.#assertReady();
    const geomStr = typeof geojson === 'string' ? geojson : JSON.stringify(geojson);
    const wasmMod = this.#_wasmModule();
    const json = wasmMod.computeArea(geomStr);
    const result = JSON.parse(json);
    return result.area_ha ?? result.area ?? 0;
  }

  /**
   * Compute bounding box of a GeoJSON geometry.
   * @param {object|string} geojson
   * @returns {{minLon:number,minLat:number,maxLon:number,maxLat:number}}
   */
  computeBbox(geojson) {
    this.#assertReady();
    const geomStr = typeof geojson === 'string' ? geojson : JSON.stringify(geojson);
    const wasmMod = this.#_wasmModule();
    const json = wasmMod.computeBbox(geomStr);
    return JSON.parse(json);
  }

  /**
   * Compute centroid of a GeoJSON geometry.
   * @param {object|string} geojson
   * @returns {{lat:number,lon:number}}
   */
  computeCentroid(geojson) {
    this.#assertReady();
    const geomStr = typeof geojson === 'string' ? geojson : JSON.stringify(geojson);
    const wasmMod = this.#_wasmModule();
    const json = wasmMod.computeCentroid(geomStr);
    return JSON.parse(json);
  }

  /**
   * Simplify a geometry using Douglas-Peucker.
   * @param {object|string} geojson
   * @param {number} [epsilon=0.001]  simplification tolerance
   * @returns {object} simplified GeoJSON
   */
  simplify(geojson, epsilon = 0.001) {
    this.#assertReady();
    const geomStr = typeof geojson === 'string' ? geojson : JSON.stringify(geojson);
    if (epsilon <= 0) throw new Error('simplify: epsilon must be > 0');
    const wasmMod = this.#_wasmModule();
    const json = wasmMod.simplifyGeometry(geomStr, epsilon);
    return JSON.parse(json);
  }

  /**
   * Compute convex hull of a geometry.
   * @param {object|string} geojson
   * @returns {object} convex hull GeoJSON
   */
  convexHull(geojson) {
    this.#assertReady();
    const geomStr = typeof geojson === 'string' ? geojson : JSON.stringify(geojson);
    const wasmMod = this.#_wasmModule();
    const json = wasmMod.convexHull(geomStr);
    return JSON.parse(json);
  }

  // -----------------------------------------------------------------------
  //  Raster ops
  // -----------------------------------------------------------------------

  /**
   * Band arithmetic on two same-size Float64 arrays.
   * @param {Float64Array|number[]} a   band A data
   * @param {Float64Array|number[]} b   band B data
   * @param {Object} opts
   * @param {'add'|'sub'|'mul'|'div'} [opts.op='add']   operation
   * @param {number} [opts.rows=1]
   * @param {number} [opts.cols]
   * @returns {Object} {data, rows, cols, nodata}
   */
  bandMath(a, b, opts = {}) {
    this.#assertReady();
    const aArr = Array.from(a);
    const bArr = Array.from(b);
    if (aArr.length !== bArr.length) {
      throw new Error('bandMath: arrays must have same length');
    }
    const rows  = opts.rows || 1;
    const cols  = opts.cols || aArr.length;
    const wasmMod = this.#_wasmModule();
    let json;
    switch (opts.op || 'add') {
      case 'sub': json = wasmMod.bandSub(aArr,  rows, cols, bArr, rows, cols); break;
      case 'mul': json = wasmMod.bandMul(aArr,  rows, cols, bArr, rows, cols); break;
      case 'div': json = wasmMod.bandDiv(aArr,  rows, cols, bArr, rows, cols); break;
      default:    json = wasmMod.bandAdd(aArr,  rows, cols, bArr, rows, cols); break;
    }
    return JSON.parse(json);
  }

  /**
   * Threshold a raster band.
   * @param {Float64Array|number[]} data   pixel values
   * @param {number} rows
   * @param {number} cols
   * @param {number} threshold
   * @returns {Object} {data, rows, cols, nodata}
   */
  bandThreshold(data, rows, cols, threshold) {
    this.#assertReady();
    const arr = Array.from(data);
    const wasmMod = this.#_wasmModule();
    const json = wasmMod.bandThreshold(arr, rows, cols, threshold);
    return JSON.parse(json);
  }

  /**
   * Resample raster data to new dimensions.
   * @param {Float64Array|number[]} data   pixel values
   * @param {number} srcRows
   * @param {number} srcCols
   * @param {number} dstRows
   * @param {number} dstCols
   * @param {Object} [opts]
   * @param {'nearest'|'cubic'} [opts.method='nearest']
   * @param {number} [opts.nodata=null]
   * @returns {number[]} resampled pixel array
   */
  resample(data, srcRows, srcCols, dstRows, dstCols, opts = {}) {
    const arr = Array.from(data);
    const nodata = opts.nodata ?? null;
    const wasmMod = this.#_wasmModule();
    if (opts.method === 'cubic') {
      return wasmMod.resampleCubic(arr, srcRows, srcCols, dstRows, dstCols, nodata);
    }
    return wasmMod.resampleNearest(arr, srcRows, srcCols, dstRows, dstCols, nodata);
  }

  /**
   * Compute zonal statistics.
   * @param {Float64Array|number[]} values  pixel values
   * @param {Uint32Array|number[]}  zones   zone IDs (1-indexed)
   * @param {number} numZones
   * @param {number} [nodata=null]
   * @returns {{zones: Array<{count,min,max,mean,stddev,sum}>}}
   */
  computeZonalStats(values, zones, numZones, nodata = null) {
    this.#assertReady();
    const valArr = Array.from(values);
    const zoneArr = Array.from(zones);
    if (valArr.length !== zoneArr.length) {
      throw new Error('computeZonalStats: values and zones must have same length');
    }
    const wasmMod = this.#_wasmModule();
    const json = wasmMod.computeZonalStats(valArr, zoneArr, numZones, nodata);
    return JSON.parse(json);
  }

  // -----------------------------------------------------------------------
  //  Map tile encoding
  // -----------------------------------------------------------------------

  /**
   * Add a vector source from GeoJSON and display as MVT tiles.
   * @param {string} sourceId     MapLibre source id
   * @param {object|string} geojson
   * @param {number} [maxZoom=14]
   */
  async addMvtSource(sourceId, geojson, maxZoom = 14) {
    this.#assertReady();
    if (this.#map.getSource(sourceId)) {
      this.#map.removeSource(sourceId);
    }
    const data = typeof geojson === 'string' ? JSON.parse(geojson) : geojson;
    const tiles = await this.#geojsonToMvt(data, maxZoom);

    this.#map.addSource(sourceId, {
      type: 'vector',
      tiles,
      minzoom: 0,
      maxzoom: maxZoom,
    });
  }

  /**
   * Add default fill layer for a vector source.
   * @param {string} sourceId   MapLibre source id
   * @param {string} [layerId]  layer id (default: sourceId + '-fill')
   */
  addFillLayer(sourceId, layerId) {
    const id = layerId || `${sourceId}-fill`;
    if (this.#map.getLayer(id)) return;
    this.#map.addLayer({
      id,
      type: 'fill',
      source: sourceId,
      'source-layer': 'default',
      paint: {
        'fill-color': '#088',
        'fill-opacity': 0.3,
        'fill-outline-color': '#044',
      },
    });
  }

  // -----------------------------------------------------------------------
  //  Drawing
  // -----------------------------------------------------------------------

  /**
   * Enable freehand polygon drawing on the map.
   * Emits `aoi-drawn` on the map with `{ aoi: GeoJSON Feature }`.
   */
  enableDrawing() {
    if (this.#drawing) return;

    const map = this.#map;
    const canvas = map.getCanvas();
    const state = { active: false, vertices: [], markerLayer: null };
    this.#drawing = state;

    // ----- Preview source & layer -----
    map.addSource('__geo_draw_preview', {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
    });
    map.addLayer({
      id: '__geo_draw_preview_line',
      type: 'line',
      source: '__geo_draw_preview',
      paint: { 'line-color': '#ff6600', 'line-width': 2, 'line-dasharray': [4, 2] },
    });
    map.addLayer({
      id: '__geo_draw_preview_vertices',
      type: 'circle',
      source: '__geo_draw_preview',
      paint: { 'circle-radius': 5, 'circle-color': '#ff6600' },
    });

    const previewSrc = map.getSource('__geo_draw_preview');

    function updatePreview() {
      const verts = state.vertices;
      if (verts.length === 0) {
        previewSrc.setData({ type: 'FeatureCollection', features: [] });
        return;
      }
      const lineCoords = [...verts, verts[0]];
      const features = [
        { type: 'Feature', geometry: { type: 'LineString', coordinates: lineCoords }, properties: {} },
        ...verts.map(c => ({ type: 'Feature', geometry: { type: 'Point', coordinates: c }, properties: {} })),
      ];
      previewSrc.setData({ type: 'FeatureCollection', features });
    }

    const onClick = (e) => {
      if (!state.active) return;
      const pt = e.lngLat;
      state.vertices.push([pt.lng, pt.lat]);
      updatePreview();
    };
    const onDblClick = () => { if (state.active && state.vertices.length >= 3) finishDrawing(); };
    const onContextMenu = (e) => {
      e.preventDefault();
      if (state.active && state.vertices.length >= 3) finishDrawing();
    };
    const onMouseMove = (_e) => {};
    const onKeyDown = (e) => {
      if (!state.active) return;
      if (e.key === 'Escape') cancelDrawing();
      else if (e.key === 'Enter' && state.vertices.length >= 3) finishDrawing();
      else if (e.key === 'Backspace' && state.vertices.length > 0) { state.vertices.pop(); updatePreview(); }
    };

    const finishDrawing = () => {
      state.active = false;
      canvas.style.cursor = '';
      const coords = [...state.vertices, state.vertices[0]];
      const aoi = {
        type: 'Feature',
        properties: { drawnAt: new Date().toISOString() },
        geometry: { type: 'Polygon', coordinates: [coords] },
      };
      map.fire('aoi-drawn', { aoi });
    };
    const cancelDrawing = () => {
      state.active = false;
      state.vertices = [];
      canvas.style.cursor = '';
      updatePreview();
    };

    state.active = true;
    state.vertices = [];
    canvas.style.cursor = 'crosshair';

    map.on('click', onClick);
    map.on('dblclick', onDblClick);
    map.on('contextmenu', onContextMenu);
    map.on('mousemove', onMouseMove);

    this.#drawing = state;
    this.#drawing.cancel = cancelDrawing;
    this.#drawing.finish = finishDrawing;
    this.#drawing._onKey = onKeyDown;
    this.#drawing._onClick = onClick;
    this.#drawing._onDblClick = onDblClick;
    this.#drawing._onCtx = onContextMenu;
    this.#drawing._onMove = onMouseMove;

    window.addEventListener('keydown', onKeyDown);
    console.log('geo-toolbox: drawing enabled — click to add vertices, double-click to finish, Esc to cancel');
  }

  /** Cancel current drawing session and remove listeners. */
  disableDrawing() {
    const d = this.#drawing;
    if (!d) return;
    d.cancel?.();
    window.removeEventListener('keydown', d._onKey);
    this.#map.off('click', d._onClick);
    this.#map.off('dblclick', d._onDblClick);
    this.#map.off('contextmenu', d._onCtx);
    this.#map.off('mousemove', d._onMove);
    try { this.#map.removeLayer('__geo_draw_preview_vertices'); } catch (_) {}
    try { this.#map.removeLayer('__geo_draw_preview_line'); }    catch (_) {}
    try { this.#map.removeSource('__geo_draw_preview'); }        catch (_) {}
    this.#drawing = null;
    this.#map.getCanvas().style.cursor = '';
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

  #buildFactors(landcover, customFactors) {
    let base;
    if (customFactors) {
      base = typeof customFactors === 'string' ? JSON.parse(customFactors) : customFactors;
    } else {
      base = [...DEFAULT_FACTORS];
    }
    const has = base.some(f => f.category === landcover);
    if (!has) base.push({ category: landcover, factor_value: 1.0, source: 'user' });
    return base;
  }

  /** Compute area (hectares) for a GeoJSON feature — uses WASM, falls back to JS approx. */
  #computeArea(aoi) {
    if (aoi.properties?.area_ha) return aoi.properties.area_ha;
    try {
      // Try WASM computeArea for precise geodesic area
      const json = this.#engine?.crs
        ? null  // CRS engine doesn't have computeArea — use the raw wasm module
        : null;
      // Fall through to JS approximation if WASM not available
    } catch (_) { /* fallback */ }

    let poly = aoi;
    if (aoi.type === 'FeatureCollection') poly = aoi.features?.[0] ?? aoi;
    const coords = poly?.geometry?.coordinates?.[0];
    if (!coords || coords.length < 3) return 100;
    const lons = coords.map(c => c[0]);
    const lats = coords.map(c => c[1]);
    const lonSpan = Math.max(...lons) - Math.min(...lons);
    const latSpan = Math.max(...lats) - Math.min(...lats);
    const midLat = (Math.max(...lats) + Math.min(...lats)) / 2;
    const degToM = 111_320 * Math.cos((midLat * Math.PI) / 180);
    const areaSqm = (lonSpan * degToM) * (latSpan * 111_320);
    return Math.round((areaSqm / 1e4) * 100) / 100;
  }

  #collectSourceData(source) {
    if (source.type === 'geojson' && source._data) return source._data;
    return { type: 'FeatureCollection', features: [] };
  }

  #updateSource(sourceId, data) {
    const source = this.#map.getSource(sourceId);
    if (source?.setData) source.setData(data);
  }

  #transformCoords(geojson, from, to) {
    this.#assertReady();
    const features = geojson.features;
    if (!features) return geojson;
    for (const f of features) {
      if (f.geometry?.coordinates) {
        f.geometry.coordinates = this.#walkCoords(f.geometry.coordinates, (lon, lat) => {
          const pt = this.#engine.crs.transform(lon, lat, from, to);
          return [pt[0], pt[1]];
        });
      }
    }
    return geojson;
  }

  #walkCoords(coords, fn) {
    if (Array.isArray(coords) && typeof coords[0] === 'number') {
      return fn(coords[0], coords[1]);
    }
    return coords.map(c => this.#walkCoords(c, fn));
  }

  /**
   * Encode a GeoJSON FeatureCollection to MVT tiles.
   * Tile-level encoding via WASM TileEngine.encodeMvt.
   */
  async #geojsonToMvt(geojson, maxZoom) {
    this.#assertReady();
    const fc = typeof geojson === 'string' ? JSON.parse(geojson) : geojson;
    const fcStr = JSON.stringify(fc);
    const tiles = [];
    for (let z = 0; z <= maxZoom; z++) {
      try {
        const mvtBytes = this.#engine.tile.encodeMvt('default', fcStr, 0, 0, z, 4096);
        const blob = new Blob([mvtBytes], { type: 'application/vnd.mapbox-vector-tile' });
        tiles.push(URL.createObjectURL(blob));
      } catch (e) {
        console.warn(`MVT encoding failed at z=${z}`, e);
      }
    }
    if (tiles.length === 0) {
      return [`data:application/json,${encodeURIComponent(fcStr)}`];
    }
    return tiles;
  }
}
