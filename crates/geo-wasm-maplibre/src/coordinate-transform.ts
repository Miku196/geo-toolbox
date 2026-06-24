/**
 * Coordinate Reference System (CRS) transformations via @geo-toolbox/wasm.
 *
 * @packageDocumentation
 * @module @geo-toolbox/maplibre
 */

import type { LngLat, CRS, TransformResult } from "./types.js";

/**
 * Transform an array of coordinate pairs from one CRS to another.
 *
 * Supports EPSG codes (e.g. "EPSG:4326", "EPSG:3857", "EPSG:4490") and
 * authority strings (e.g. "urn:ogc:def:crs:EPSG::4326").
 *
 * @example
 * ```ts
 * import { transformCoords } from "@geo-toolbox/maplibre";
 *
 * // Beijing 1954 → WGS84
 * const wgs84Coords = await transformCoords(
 *   [[116.4, 39.9]],
 *   "EPSG:4214",  // Beijing 1954
 *   "EPSG:4326"   // WGS84
 * );
 * ```
 *
 * @example
 * ```ts
 * // Batch transform: WGS84 → Web Mercator
 * const mercator = await transformCoords(
 *   [[116.4, 39.9], [120.2, 30.3]],
 *   "EPSG:4326",
 *   "EPSG:3857"
 * );
 * ```
 */
export async function transformCoords(
  coords: Array<[number, number]>,
  fromCRS: CRS,
  toCRS: CRS
): Promise<Array<[number, number]>> {
  const wasm = await loadWasmModule();
  // Convert to the format WASM expects: array of [x, y] objects
  const result = wasm.crs_transform(
    coords.map(([lng, lat]) => ({ x: lng, y: lat })),
    fromCRS,
    toCRS
  );
  return result.map((p: { x: number; y: number }) => [p.x, p.y]);
}

/**
 * Auto-detect the UTM zone for a given longitude.
 *
 * Returns the EPSG code for the appropriate UTM zone in the northern
 * or southern hemisphere.
 *
 * @example
 * ```ts
 * const epsg = autoDetectUTMZone(116.4, 39.9); // => "EPSG:32650" (UTM 50N)
 * ```
 *
 * @example
 * ```ts
 * // Use with transformCoords for auto-UTM
 * const zone = autoDetectUTMZone(lon, lat);
 * const projected = await transformCoords(
 *   [[lon, lat]],
 *   "EPSG:4326",
 *   zone
 * );
 * ```
 */
export function autoDetectUTMZone(lon: number, lat?: number): CRS {
  const zone = Math.floor((lon + 180) / 6) + 1;
  const clamped = Math.max(1, Math.min(60, zone));
  const north = lat === undefined || lat >= 0;
  const epsgCode = north ? 32600 + clamped : 32700 + clamped;
  return `EPSG:${epsgCode}`;
}

/**
 * Auto-detect the China Gauss-Krüger 3° or 6° zone for a given longitude.
 *
 * @example
 * ```ts
 * const gkZone = autoDetectGKZone(116.4); // => "EPSG:4547"
 * ```
 */
export function autoDetectGKZone(lon: number): CRS {
  // China 3° zone: zone number 24-45, each 3° wide
  // 6° zone: zone number 13-23, each 6° wide
  const zone3 = Math.round((lon - 1.5) / 3) + 1;
  // CGCS2000 3° Gauss-Krüger: EPSG:4525-4548
  const epsgCode = 4524 + zone3;
  return `EPSG:${epsgCode}`;
}

/**
 * List all supported CRS codes that can be used for transformation.
 *
 * @example
 * ```ts
 * const crsList = await listSupportedCRS();
 * console.log(crsList); // ["EPSG:4326", "EPSG:3857", "EPSG:4490", ...]
 * ```
 */
export async function listSupportedCRS(): Promise<string[]> {
  const wasm = await loadWasmModule();
  return wasm.crs_list();
}

// ── Internal helpers ──

let cachedWasmModule: CrsWasmModule | null = null;

async function loadWasmModule(): Promise<CrsWasmModule> {
  if (!cachedWasmModule) {
    const wasm = await import("@geo-toolbox/wasm");
    cachedWasmModule = (wasm.default ?? wasm) as CrsWasmModule;
  }
  return cachedWasmModule;
}

/** Internal WASM module shape for CRS operations */
interface CrsWasmModule {
  crs_transform(
    coords: Array<{ x: number; y: number }>,
    fromCRS: string,
    toCRS: string
  ): Array<{ x: number; y: number }>;
  crs_list(): string[];
}
