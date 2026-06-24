/**
 * @geo-toolbox/maplibre
 *
 * MapLibre GL JS plugin for geo-toolbox — WASM-powered spatial operations,
 * CRS transformations, and tile utilities running natively in the browser.
 *
 * @example
 * ```ts
 * import { buffer, centroid, transformCoords, autoDetectUTMZone } from "@geo-toolbox/maplibre";
 *
 * // Buffer a click point by 500m
 * const geom = { type: "Point", coordinates: [116.4, 39.9] };
 * const buffered = await buffer(geom, 500);
 * map.getSource("buffer").setData(buffered);
 *
 * // Auto-UTM and transform
 * const zone = autoDetectUTMZone(116.4, 39.9);
 * const projected = await transformCoords([[116.4, 39.9]], "EPSG:4326", zone);
 * ```
 *
 * @packageDocumentation
 */

// Spatial operations (WASM)
export {
  buffer,
  intersect,
  area,
  centroid,
  decodeGeohash,
  encodeGeohash,
} from "./spatial-operations.js";

// Coordinate transforms
export {
  transformCoords,
  autoDetectUTMZone,
  autoDetectGKZone,
  listSupportedCRS,
} from "./coordinate-transform.js";

// Tile utilities (pure JS)
export {
  latLonToTile,
  tileToBBox,
  geojsonTileUrl,
  parentTile,
  childTiles,
  tilesInBBox,
  isInTile,
} from "./tile-utils.js";

// Types
export type {
  GeoJSONGeometry,
  GeoJSONFeature,
  GeoJSONFeatureCollection,
  BBox,
  LngLat,
  CRS,
  GeohashInfo,
  TileCoord,
  TransformResult,
  CarbonPool,
  NdviValue,
} from "./types.js";
