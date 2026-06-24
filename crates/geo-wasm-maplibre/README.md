# @geo-toolbox/maplibre

MapLibre GL JS plugin for [geo-toolbox](https://github.com/geo-toolbox/geo-toolbox) — WASM-powered spatial operations, CRS transformations, and tile utilities running natively in the browser.

## Installation

```bash
npm install @geo-toolbox/maplibre @geo-toolbox/wasm maplibre-gl
```

## Quick Start

```ts
import { buffer, centroid, transformCoords, autoDetectUTMZone } from "@geo-toolbox/maplibre";

// Buffer a clicked point by 500m
map.on("click", async (e) => {
  const geom = { type: "Point", coordinates: [e.lngLat.lng, e.lngLat.lat] };
  const buffered = await buffer(geom, 500);
  map.getSource("buffer").setData(buffered);
});

// Auto-detect UTM zone and transform
const zone = autoDetectUTMZone(116.4, 39.9); // => "EPSG:32650"
const projected = await transformCoords([[116.4, 39.9]], "EPSG:4326", zone);
```

## API

### Spatial Operations (`spatial-operations.js`)

| Function | Description |
|----------|-------------|
| `buffer(geometry, distance)` | Buffer geometry by distance (meters for geographic CRS) |
| `intersect(geomA, geomB)` | Intersect two geometries, returning the overlap |
| `area(geometry)` | Calculate area in m² (geographic) or unit² (projected) |
| `centroid(geometry)` | Compute centroid as `[lon, lat]` |
| `decodeGeohash(hash)` | Decode geohash → `{ lat, lon, precision, bbox }` |
| `encodeGeohash(lat, lon, precision)` | Encode lat/lon → geohash string (1-12) |

### Coordinate Transform (`coordinate-transform.js`)

| Function | Description |
|----------|-------------|
| `transformCoords(coords, fromCRS, toCRS)` | Transform coordinate array between CRS |
| `autoDetectUTMZone(lon, lat?)` | Auto-detect EPSG code for UTM zone |
| `autoDetectGKZone(lon)` | Auto-detect China Gauss-Krüger 3° zone |
| `listSupportedCRS()` | List all supported CRS by geo-toolbox WASM |

### Tile Utilities (`tile-utils.js`)

| Function | Description |
|----------|-------------|
| `latLonToTile(lat, lon, zoom)` | Convert lat/lon → `{ z, x, y }` tile coordinate |
| `tileToBBox(tile)` | Convert tile → `[west, south, east, north]` bounding box |
| `geojsonTileUrl(baseUrl, layer)` | Generate MapLibre tile URL template |
| `parentTile(tile)` | Get parent tile one zoom level up |
| `childTiles(tile)` | Get 4 child tiles one zoom level down |
| `tilesInBBox(bbox, zoom)` | Count tiles at zoom within bounding box |
| `isInTile(tile, lat, lon)` | Check if lat/lon falls inside tile |

### Types (`types.js`)

TypeScript types for GeoJSON, BBox, CRS, TileCoord, GeohashInfo, CarbonPool, and NdviValue.

## Example: Complete MapLibre Integration

```ts
import maplibregl from "maplibre-gl";
import { buffer, centroid, transformCoords, autoDetectUTMZone } from "@geo-toolbox/maplibre";

const map = new maplibregl.Map({
  container: "map",
  style: "https://demotiles.maplibre.org/style.json",
  center: [116.4, 39.9],
  zoom: 10,
});

map.on("load", async () => {
  // Add a buffer source
  map.addSource("buffer-layer", {
    type: "geojson",
    data: { type: "FeatureCollection", features: [] },
  });
  map.addLayer({
    id: "buffer-fill",
    type: "fill",
    source: "buffer-layer",
    paint: { "fill-color": "#088", "fill-opacity": 0.3 },
  });

  // On click: buffer and show
  map.on("click", "parcels", async (e) => {
    if (!e.features?.[0]) return;
    const buffered = await buffer(e.features[0], 1000);
    map.getSource("buffer-layer").setData(buffered);
  });
});
```

## License

MIT
