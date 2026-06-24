/**
 * MapLibre GL JS — WASM-powered spatial operations via @geo-toolbox/wasm.
 *
 * @packageDocumentation
 * @module @geo-toolbox/maplibre
 */

import type {
  GeoJSONGeometry,
  GeoJSONFeature,
  GeoJSONFeatureCollection,
  LngLat,
  BBox,
} from "./types.js";

/**
 * Buffer a GeoJSON geometry by a given distance (meters for geographic CRS,
 * or same unit as input for projected CRS).
 *
 * Internally calls `geo_wasm.vector_buffer(geojson, distance)`.
 *
 * @example
 * ```ts
 * import { buffer } from "@geo-toolbox/maplibre";
 *
 * // Buffer a point by 1000 meters
 * const buffered = await buffer(pointGeoJSON, 1000);
 * map.addSource("buffer", { type: "geojson", data: buffered });
 * ```
 */
export async function buffer(
  geometry: GeoJSONGeometry | GeoJSONFeature | GeoJSONFeatureCollection,
  distance: number
): Promise<GeoJSONFeatureCollection> {
  const wasm = await loadWasmModule();
  const input = JSON.stringify(unwrapGeometry(geometry));
  const resultJson = wasm.vector_buffer(input, distance);
  return JSON.parse(resultJson) as GeoJSONFeatureCollection;
}

/**
 * Intersect two GeoJSON geometries. Returns the overlapping area.
 *
 * @example
 * ```ts
 * const intersection = await intersect(aoiGeoJSON, landcoverGeoJSON);
 * // Intersection is the common area between the two inputs
 * ```
 */
export async function intersect(
  geometryA: GeoJSONGeometry | GeoJSONFeature | GeoJSONFeatureCollection,
  geometryB: GeoJSONGeometry | GeoJSONFeature | GeoJSONFeatureCollection
): Promise<GeoJSONFeatureCollection> {
  const wasm = await loadWasmModule();
  const a = JSON.stringify(unwrapGeometry(geometryA));
  const b = JSON.stringify(unwrapGeometry(geometryB));
  const resultJson = wasm.vector_intersect(a, b);
  return JSON.parse(resultJson) as GeoJSONFeatureCollection;
}

/**
 * Calculate the area of a GeoJSON geometry.
 *
 * For geographic coordinates (long/lat), returns square meters.
 * For projected coordinates, returns the unit squared.
 *
 * @example
 * ```ts
 * const areaSqM = await area(polygonGeoJSON);
 * console.log(`Area: ${(areaSqM / 10_000).toFixed(2)} ha`);
 * ```
 */
export async function area(
  geometry: GeoJSONGeometry | GeoJSONFeature | GeoJSONFeatureCollection
): Promise<number> {
  const wasm = await loadWasmModule();
  const input = JSON.stringify(unwrapGeometry(geometry));
  return wasm.vector_area(input);
}

/**
 * Compute the centroid of a GeoJSON geometry.
 *
 * @example
 * ```ts
 * const [lon, lat] = await centroid(polygonGeoJSON);
 * map.flyTo({ center: [lon, lat] });
 * ```
 */
export async function centroid(
  geometry: GeoJSONGeometry | GeoJSONFeature | GeoJSONFeatureCollection
): Promise<[number, number]> {
  const wasm = await loadWasmModule();
  const input = JSON.stringify(unwrapGeometry(geometry));
  const result = wasm.vector_centroid(input);
  return [result.lon, result.lat];
}

/**
 * Decode a geohash string into latitude, longitude, and precision.
 *
 * @example
 * ```ts
 * const { lat, lon, bbox } = decodeGeohash("wx4g0ec1");
 * map.fitBounds(bbox, { padding: 20 });
 * ```
 */
export async function decodeGeohash(
  hash: string
): Promise<{ lat: number; lon: number; precision: number; bbox: BBox }> {
  const wasm = await loadWasmModule();
  return wasm.geohash_decode(hash);
}

/**
 * Encode lat/lon into a geohash with given precision (1-12).
 *
 * @example
 * ```ts
 * const hash = await encodeGeohash(39.9, 116.4, 8);
 * ```
 */
export async function encodeGeohash(
  lat: number,
  lon: number,
  precision: number
): Promise<string> {
  const wasm = await loadWasmModule();
  return wasm.geohash_encode(lat, lon, precision);
}

// ── Internal helpers ──

/** Lazy-load the WASM module from @geo-toolbox/wasm */
async function loadWasmModule(): Promise<GeoWasmModule> {
  if (!cachedWasmModule) {
    const wasm = await import("@geo-toolbox/wasm");
    // Default export or named export — depends on wasm-pack target
    cachedWasmModule = (wasm.default ?? wasm) as GeoWasmModule;
  }
  return cachedWasmModule;
}

let cachedWasmModule: GeoWasmModule | null = null;

/** Unwrap Feature/FeatureCollection to raw geometry for processing */
function unwrapGeometry(
  input: GeoJSONGeometry | GeoJSONFeature | GeoJSONFeatureCollection
): GeoJSONGeometry {
  if (input.type === "Feature") {
    return (input as GeoJSONFeature).geometry;
  }
  if (input.type === "FeatureCollection") {
    const fc = input as GeoJSONFeatureCollection;
    if (fc.features.length === 1) {
      return fc.features[0].geometry;
    }
    // Multi-feature: return as GeometryCollection-like wrapper
    return {
      type: "GeometryCollection",
      geometries: fc.features.map((f) => f.geometry),
    } as unknown as GeoJSONGeometry;
  }
  return input;
}

/** Internal WASM module shape (to be matched with wasm-bindgen exports) */
interface GeoWasmModule {
  vector_buffer(json: string, distance: number): string;
  vector_intersect(jsonA: string, jsonB: string): string;
  vector_area(json: string): number;
  vector_centroid(json: string): { lon: number; lat: number };
  geohash_decode(hash: string): {
    lat: number;
    lon: number;
    precision: number;
    bbox: [number, number, number, number];
  };
  geohash_encode(lat: number, lon: number, precision: number): string;
}
