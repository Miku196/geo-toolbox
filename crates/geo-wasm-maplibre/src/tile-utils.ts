/**
 * Tile utility functions for MapLibre GL JS.
 *
 * @packageDocumentation
 * @module @geo-toolbox/maplibre
 */

import type { TileCoord, LngLat, BBox } from "./types.js";

/**
 * Convert latitude/longitude to tile coordinates (XYZ scheme).
 *
 * @example
 * ```ts
 * import { latLonToTile } from "@geo-toolbox/maplibre";
 *
 * const tile = latLonToTile(39.9, 116.4, 10);
 * // => { z: 10, x: 843, y: 388 }
 * ```
 */
export function latLonToTile(lat: number, lon: number, zoom: number): TileCoord {
  const n = Math.pow(2, zoom);
  const x = Math.floor(((lon + 180) / 360) * n);
  const latRad = (lat * Math.PI) / 180;
  const y = Math.floor(
    ((1 - Math.log(Math.tan(latRad) + 1 / Math.cos(latRad)) / Math.PI) / 2) *
      n
  );
  return { z: zoom, x, y: Math.max(0, Math.min(n - 1, y)) };
}

/**
 * Convert tile coordinates back to a bounding box [west, south, east, north].
 *
 * @example
 * ```ts
 * const bbox = tileToBBox({ z: 10, x: 843, y: 388 });
 * map.fitBounds(bbox);
 * ```
 */
export function tileToBBox(tile: TileCoord): BBox {
  const n = Math.pow(2, tile.z);
  const west = (tile.x / n) * 360 - 180;
  const east = ((tile.x + 1) / n) * 360 - 180;
  const south =
    (Math.atan(Math.sinh(Math.PI * (1 - (2 * (tile.y + 1)) / n))) * 180) /
    Math.PI;
  const north =
    (Math.atan(Math.sinh(Math.PI * (1 - (2 * tile.y) / n))) * 180) / Math.PI;
  return [west, south, east, north];
}

/**
 * Generate a MapLibre tile URL template for a GeoJSON tile service
 * (e.g. Martin, Tileserver GL, or pg_tileserv).
 *
 * @example
 * ```ts
 * const url = geojsonTileUrl("http://localhost:3111", "public.parcels");
 * source.addSource("parcels", {
 *   type: "vector",
 *   tiles: [url]  // => "http://localhost:3111/public.parcels/{z}/{x}/{y}"
 * });
 * ```
 */
export function geojsonTileUrl(baseUrl: string, layer: string): string {
  const cleanBase = baseUrl.replace(/\/+$/, "");
  return `${cleanBase}/${layer}/{z}/{x}/{y}`;
}

/**
 * Get the parent tile for a given tile coordinate.
 *
 * @example
 * ```ts
 * const parent = parentTile({ z: 10, x: 843, y: 388 });
 * // => { z: 9, x: 421, y: 194 }
 * ```
 */
export function parentTile(tile: TileCoord): TileCoord | null {
  if (tile.z <= 0) return null;
  return {
    z: tile.z - 1,
    x: Math.floor(tile.x / 2),
    y: Math.floor(tile.y / 2),
  };
}

/**
 * Get child tiles for a given tile coordinate.
 *
 * @example
 * ```ts
 * const children = childTiles({ z: 9, x: 421, y: 194 });
 * // => 4 tiles at zoom 10 covering the same area
 * ```
 */
export function childTiles(tile: TileCoord): TileCoord[] {
  const z = tile.z + 1;
  const x2 = tile.x * 2;
  const y2 = tile.y * 2;
  return [
    { z, x: x2, y: y2 },
    { z, x: x2 + 1, y: y2 },
    { z, x: x2, y: y2 + 1 },
    { z, x: x2 + 1, y: y2 + 1 },
  ];
}

/**
 * Count the number of tiles at a given zoom level within a bounding box.
 * Useful for estimating tile cache size.
 *
 * @example
 * ```ts
 * const count = tilesInBBox([114, 22, 122, 32], 12);
 * console.log(`~${count} tiles needed for zoom 12 over HK-Pearl Delta`);
 * ```
 */
export function tilesInBBox(bbox: BBox, zoom: number): number {
  const tl = latLonToTile(bbox[3], bbox[0], zoom); // top-left
  const br = latLonToTile(bbox[1], bbox[2], zoom); // bottom-right
  return (br.x - tl.x + 1) * (br.y - tl.y + 1);
}

/**
 * Check if a lat/lon pair is inside a tile (at the tile's zoom level).
 */
export function isInTile(tile: TileCoord, lat: number, lon: number): boolean {
  const tileCoord = latLonToTile(lat, lon, tile.z);
  return tileCoord.x === tile.x && tileCoord.y === tile.y;
}
