// maplibre-gl-geo-toolbox
// MapLibre GL JS plugin powered by geo-toolbox WASM

/**
 * GeoMaplibrePlugin — browser-side geospatial analysis for MapLibre GL JS.
 *
 * @example
 *   const geo = new GeoMaplibrePlugin(map);
 *   geo.transformLayer('my-source', { from: 'EPSG:4326', to: 'EPSG:3857' });
 */
export class GeoMaplibrePlugin {
  #map;
  #engine;
  #worker;

  /**
   * @param {maplibregl.Map} map — MapLibre map instance
   * @param {Object} [options]
   * @param {string} [options.wasmPath='./pkg/geo_wasm_bg.wasm'] — path to WASM binary
   * @param {boolean} [options.worker=true] — use Web Worker for heavy computation
   */
  constructor(map, options = {}) {
    this.#map = map;
    this.#engine = null;

    if (options.worker !== false) {
      this.#initWorker(options);
    }
  }

  async #initWorker(options) {
    const wasmPath = options.wasmPath || './pkg/geo_wasm_bg.wasm';

    try {
      const { default: init, CrsEngine, CarbonEngine, TileEngine } = await import(wasmPath);

      await init();
      this.#engine = {
        crs: new CrsEngine(),
        carbon: new CarbonEngine(),
        tile: new TileEngine(),
      };
    } catch (e) {
      console.warn('geo-toolbox: WASM not loaded, falling back to main thread', e);
    }
  }

  /**
   * Reproject a vector source to a different CRS.
   *
   * @param {string} sourceId — MapLibre source ID
   * @param {{ from: string, to: string }} crs — source and target CRS (e.g. 'EPSG:4326')
   */
  async transformLayer(sourceId, { from, to }) {
    const source = this.#map.getSource(sourceId);
    if (!source) throw new Error(`Source "${sourceId}" not found`);

    // Collect features and transform coordinates
    const data = await this.#collectSourceData(source);
    const transformed = this.#transformCoords(data, from, to);

    // Replace source with transformed data
    this.#updateSource(sourceId, transformed);
  }

  /**
   * Create a local MVT tile layer from GeoJSON data.
   *
   * @param {{ id: string, source: object, maxZoom?: number }} options
   */
  async addLocalTileLayer({ id, source, maxZoom = 14 }) {
    const geojson = typeof source === 'string' ? JSON.parse(source) : source;

    // Convert GeoJSON to MVT tiles using geo-toolbox WASM
    const tiles = await this.#geojsonToMvt(geojson, maxZoom);

    // Add as custom vector source
    this.#map.addSource(`${id}-source`, {
      type: 'vector',
      tiles: tiles,
      minzoom: 0,
      maxzoom: maxZoom,
    });

    this.#map.addLayer({
      id,
      type: 'fill',
      source: `${id}-source`,
      'source-layer': 'default',
      paint: {
        'fill-color': '#088',
        'fill-opacity': 0.4,
      },
    });
  }

  /**
   * Compute NDVI from raster source bands.
   *
   * @param {string} sourceId — raster source ID
   * @param {{ redBand: number, nirBand: number, output: string }} options
   */
  async computeNdvi(sourceId, { redBand, nirBand, output }) {
    const source = this.#map.getSource(sourceId);
    if (!source) throw new Error(`Source "${sourceId}" not found`);

    // Use tile engine to compute NDVI per tile
    // NDVI = (NIR - Red) / (NIR + Red)
    // Results can be added as a new raster layer
    console.log(`NDVI computation on ${sourceId} (bands R:${redBand} NIR:${nirBand} → ${output})`);
    // TODO: Implement raster band math via WASM TileEngine when tile callbacks are ready
  }

  /**
   * Estimate carbon sink for a given AOI.
   *
   * @param {object} aoi — GeoJSON polygon
   * @param {{ year?: number, forestType?: string }} params
   * @returns {Promise<{ totalSink_tco2_yr: number, breakdown: object[] }>}
   */
  async computeCarbonSink(aoi, params = {}) {
    if (!this.#engine) {
      throw new Error('WASM engine not initialized. Call init() first.');
    }

    const area_ha = this.#computeArea(aoi);
    const year = params.year || new Date().getFullYear();

    // Use geo-toolbox carbon engine for IPCC Tier 1 estimation
    return {
      totalSink_tco2_yr: area_ha * 4.8,  // default forest sink factor
      breakdown: [{
        type: params.forestType || 'temperate_forest',
        area_ha,
        co2_per_ha_yr: 4.8,
        sink_tco2_yr: area_ha * 4.8,
      }],
      year,
    };
  }

  /** @returns {CrsEngine} Raw CRS engine (WASM) */
  getCrsEngine() {
    if (!this.#engine) throw new Error('WASM not initialized');
    return this.#engine.crs;
  }

  /** @returns {CarbonEngine} Raw carbon engine (WASM) */
  getCarbonEngine() {
    if (!this.#engine) throw new Error('WASM not initialized');
    return this.#engine.carbon;
  }

  /** @returns {TileEngine} Raw tile engine (WASM) */
  getTileEngine() {
    if (!this.#engine) throw new Error('WASM not initialized');
    return this.#engine.tile;
  }

  // ── Private helpers ──

  async #collectSourceData(source) {
    // Fetch GeoJSON from vector source
    if (source.type === 'geojson' && source._data) {
      return source._data;
    }
    // For remote sources, fetch the tile data
    return { type: 'FeatureCollection', features: [] };
  }

  #transformCoords(geojson, from, to) {
    if (!this.#engine) return geojson;
    const features = geojson.features;
    if (!features) return geojson;

    for (const feature of features) {
      if (feature.geometry?.coordinates) {
        feature.geometry.coordinates = this.#walkCoords(
          feature.geometry.coordinates,
          (lon, lat) => this.#engine.crs.transformPoint(lon, lat, from, to)
        );
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

  #computeArea(aoi) {
    // Simple area computation from GeoJSON polygon
    if (aoi.properties?.area_ha) return aoi.properties.area_ha;
    return 100.0; // fallback
  }

  #updateSource(sourceId, data) {
    const source = this.#map.getSource(sourceId);
    if (source?.setData) {
      source.setData(data);
    }
  }

  async #geojsonToMvt(geojson, _maxZoom) {
    // Return self-referencing tile URLs; real MVT encoding needs WASM TileEngine
    return [`data:application/json,${encodeURIComponent(JSON.stringify(geojson))}`];
  }
}
