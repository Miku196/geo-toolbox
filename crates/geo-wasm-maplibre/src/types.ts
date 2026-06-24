/**
 * @geo-toolbox/maplibre — TypeScript types for MapLibre GL JS + geo-wasm integration.
 */

/** GeoJSON geometry types supported by geo-toolbox */
export type GeoJSONGeometry =
  | GeoJSONPoint
  | GeoJSONMultiPoint
  | GeoJSONLineString
  | GeoJSONMultiLineString
  | GeoJSONPolygon
  | GeoJSONMultiPolygon;

export interface GeoJSONPoint {
  type: "Point";
  coordinates: [number, number] | [number, number, number];
}

export interface GeoJSONMultiPoint {
  type: "MultiPoint";
  coordinates: Array<[number, number]>;
}

export interface GeoJSONLineString {
  type: "LineString";
  coordinates: Array<[number, number]>;
}

export interface GeoJSONMultiLineString {
  type: "MultiLineString";
  coordinates: Array<Array<[number, number]>>;
}

export interface GeoJSONPolygon {
  type: "Polygon";
  coordinates: Array<Array<[number, number]>>;
}

export interface GeoJSONMultiPolygon {
  type: "MultiPolygon";
  coordinates: Array<Array<Array<[number, number]>>>;
}

/** GeoJSON Feature */
export interface GeoJSONFeature {
  type: "Feature";
  geometry: GeoJSONGeometry;
  properties?: Record<string, unknown>;
}

/** GeoJSON FeatureCollection */
export interface GeoJSONFeatureCollection {
  type: "FeatureCollection";
  features: GeoJSONFeature[];
}

/** Bounding box [west, south, east, north] */
export type BBox = [number, number, number, number];

/** Coordinate pair */
export type LngLat = [number, number];

/** CRS identifier (EPSG code or authority string) */
export type CRS = string;

/** Geohash with bounding info */
export interface GeohashInfo {
  hash: string;
  lat: number;
  lon: number;
  precision: number;
  bbox: BBox;
}

/** Tile coordinate in XYZ scheme */
export interface TileCoord {
  z: number;
  x: number;
  y: number;
}

/** CRS transform result */
export interface TransformResult {
  coords: Array<[number, number]>;
  fromCRS: CRS;
  toCRS: CRS;
}

/** NDVI value range */
export interface NdviValue {
  min: number;
  max: number;
  mean: number;
  std: number;
}

/** Carbon pool result */
export interface CarbonPool {
  name: string;
  areaHa: number;
  carbonTonsPerHa: number;
  totalCarbon: number;
}
