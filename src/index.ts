/// <reference types="geo_wasm" />

// ── Re-export WASM module with ergonomic JS wrappers ────────────

import init, {
  CrsEngine as WasmCrsEngine,
  CarbonEngine as WasmCarbonEngine,
  GeoStore as WasmGeoStore,
  parseNmea,
  parseNmeaBatch,
  validateGpsFix,
  validateSensorReading,
  validateCoord,
  computeArea,
  computeBbox,
  computeCentroid,
  simplifyGeometry,
  convexHull,
  exportExcel,
  exportGeoJson,
  exportCarbonReport,
  csvToJson,
  getVersion,
  getBuildInfo,
  consoleLog,
  getMemoryStats,
} from './pkg/geo_wasm.js';

// Auto-init on import
let initialized = false;

async function ensureInit() {
  if (!initialized) {
    await init();
    initialized = true;
  }
}

// ── CRS Engine ───────────────────────────────────────────────────

export class CrsEngine {
  private inner: WasmCrsEngine | null = null;

  private async get() {
    await ensureInit();
    if (!this.inner) this.inner = new WasmCrsEngine();
    return this.inner;
  }

  async listAll(): Promise<CrsDef[]> {
    const engine = await this.get();
    return JSON.parse(engine.listAll());
  }

  async transform(fromEpsg: number, toEpsg: number, x: number, y: number): Promise<[number, number]> {
    const engine = await this.get();
    const result = engine.transform(fromEpsg, toEpsg, x, y);
    return [result[0], result[1]];
  }

  async transformBatch(
    fromEpsg: number,
    toEpsg: number,
    coords: Float64Array | number[],
  ): Promise<Float64Array> {
    const engine = await this.get();
    const inputArray = coords instanceof Float64Array ? coords : new Float64Array(coords);
    const result = engine.transformBatch(fromEpsg, toEpsg, inputArray);
    return new Float64Array(result.buffer);
  }
}

// ── Carbon Engine ────────────────────────────────────────────────

export class CarbonEngine {
  private inner: WasmCarbonEngine | null = null;

  private async get() {
    await ensureInit();
    if (!this.inner) this.inner = new WasmCarbonEngine();
    return this.inner;
  }

  /** Calculate emissions from GeoJSON FeatureCollection + emission factors CSV */
  async calculate(geojson: string | GeoJSON.FeatureCollection, factorsCsv: string, year: number): Promise<CarbonReport> {
    const engine = await this.get();
    const geojsonStr = typeof geojson === 'string' ? geojson : JSON.stringify(geojson);
    const json = engine.calculate(geojsonStr, factorsCsv, year);
    return JSON.parse(json);
  }

  /** Calculate with JSON-formatted factors */
  async calculateWithJsonFactors(
    geojson: string | GeoJSON.FeatureCollection,
    factors: EmissionFactor[],
    year: number,
  ): Promise<CarbonReport> {
    const engine = await this.get();
    const geojsonStr = typeof geojson === 'string' ? geojson : JSON.stringify(geojson);
    const json = engine.calculateWithJsonFactors(geojsonStr, JSON.stringify(factors), year);
    return JSON.parse(json);
  }
}

// ── GeoStore (IndexedDB) ─────────────────────────────────────────

export class GeoStore {
  private inner: WasmGeoStore | null = null;

  private async get() {
    await ensureInit();
    if (!this.inner) this.inner = new WasmGeoStore('geo-toolbox-db');
    return this.inner;
  }

  async init() {
    const store = await this.get();
    await store.init();
  }

  async putFeature(id: string, feature: GeoJSON.Feature) {
    const store = await this.get();
    await store.putFeature(id, JSON.stringify(feature));
  }

  async getFeature(id: string): Promise<GeoJSON.Feature | null> {
    const store = await this.get();
    const result = await store.getFeature(id);
    return result ?? null;
  }

  async getAllFeatures(): Promise<GeoJSON.FeatureCollection> {
    const store = await this.get();
    const json = await store.getAllFeatures();
    return JSON.parse(json);
  }

  async deleteFeature(id: string) {
    const store = await this.get();
    await store.deleteFeature(id);
  }

  async clearAll() {
    const store = await this.get();
    await store.clearAll();
  }

  async count(): Promise<number> {
    const store = await this.get();
    return store.count();
  }
}

// ── Spatial Operations ───────────────────────────────────────────

export async function computeAreaSqm(geojsonGeom: string | GeoJSON.Geometry): Promise<{ area_sqm: number; area_ha: number }> {
  await ensureInit();
  const json = typeof geojsonGeom === 'string' ? geojsonGeom : JSON.stringify(geojsonGeom);
  const result = computeArea(json);
  return JSON.parse(JSON.stringify(result)); // unwrap JsValue
}

export async function computeBboxSpatial(geojsonGeom: string | GeoJSON.Geometry): Promise<{ minX: number; minY: number; maxX: number; maxY: number }> {
  await ensureInit();
  const json = typeof geojsonGeom === 'string' ? geojsonGeom : JSON.stringify(geojsonGeom);
  const result = computeBbox(json);
  return JSON.parse(JSON.stringify(result));
}

export async function computeCentroidSpatial(geojsonGeom: string | GeoJSON.Geometry): Promise<{ x: number; y: number }> {
  await ensureInit();
  const json = typeof geojsonGeom === 'string' ? geojsonGeom : JSON.stringify(geojsonGeom);
  const result = computeCentroid(json);
  return JSON.parse(JSON.stringify(result));
}

export async function simplifyGeometryOp(geojsonGeom: string | GeoJSON.Geometry, epsilon: number): Promise<GeoJSON.Geometry> {
  await ensureInit();
  const json = typeof geojsonGeom === 'string' ? geojsonGeom : JSON.stringify(geojsonGeom);
  const result = simplifyGeometry(json, epsilon);
  return JSON.parse(result);
}

export async function convexHullOp(geojsonGeom: string | GeoJSON.Geometry): Promise<GeoJSON.Geometry> {
  await ensureInit();
  const json = typeof geojsonGeom === 'string' ? geojsonGeom : JSON.stringify(geojsonGeom);
  const result = convexHull(json);
  return JSON.parse(result);
}

// ── Export Functions ─────────────────────────────────────────────

export async function generateExcel(
  columns: string[],
  rows: (string | number | boolean)[][],
  sheetName?: string,
): Promise<Uint8Array> {
  await ensureInit();
  return exportExcel(JSON.stringify(columns), JSON.stringify(rows), sheetName ?? null);
}

export async function generateGeoJson(features: GeoJSON.Feature[]): Promise<string> {
  await ensureInit();
  return exportGeoJson(JSON.stringify(features));
}

export async function generateCarbonReportMd(
  report: CarbonReport,
  aoiName: string,
  auditor: string,
): Promise<string> {
  await ensureInit();
  return exportCarbonReport(JSON.stringify(report), aoiName, auditor);
}

export async function parseCsvToJson(csvText: string): Promise<Record<string, unknown>[]> {
  await ensureInit();
  return JSON.parse(csvToJson(csvText));
}

// ── Parsing & Validation (sync, no init needed) ──────────────────

export { parseNmea, parseNmeaBatch, validateGpsFix, validateSensorReading, validateCoord };

// ── Info ─────────────────────────────────────────────────────────

export async function getVersionString(): Promise<string> {
  await ensureInit();
  return getVersion();
}

export async function getBuildInfoJson(): Promise<Record<string, unknown>> {
  await ensureInit();
  return JSON.parse(getBuildInfo());
}

export { consoleLog as logToConsole, getMemoryStats };

// ── Types ────────────────────────────────────────────────────────

export interface CrsDef {
  epsg: number;
  name: string;
  category: string;
}

export interface EmissionFactor {
  category: string;
  /** tCO₂e/ha/yr. Negative = carbon sink. */
  factor_value: number;
  source?: string;
  unit?: string;
  valid_from_year?: number;
  valid_to_year?: number;
  region?: string;
}

export interface ClassResult {
  landcover_class: string;
  area_ha: number;
  factor_value: number;
  emission_tco2e: number;
  factor_source: { source: string; unit: string };
  feature_count: number;
}

export interface CarbonReport {
  aoi_name?: string;
  year: number;
  classes: ClassResult[];
  total_area_ha: number;
  total_emission_tco2e: number;
  total_features: number;
  classified_features: number;
  skipped_features: number;
  calculated_at: string;
  auditor?: string;
  methodology?: string;
}
