# ObservableHQ Notebook: geo-toolbox WASM

[ObservableHQ](https://observablehq.com/) notebook that demonstrates browser-side
geospatial analysis with geo-toolbox WebAssembly.

## Quick Start

Open a new Observable notebook and import:

```js
// Import geo-toolbox WASM module
geo = import("https://cdn.example.com/geo-wasm/pkg/geo_wasm.js")

// Initialize
geo.then(async (module) => {
  await module.default(); // init WASM
})
```

## Examples

### 1. CRS Transform

```js
viewof lonInput = Inputs.range([73, 135], { value: 116.4, step: 0.1, label: "Longitude" })
viewof latInput = Inputs.range([18, 54], { value: 39.9, step: 0.1, label: "Latitude" })

{
  const engine = new geo.CrsEngine();
  const [x, y] = engine.transformPoint(lonInput, latInput, "EPSG:4326", "EPSG:3857");
  return md`**Web Mercator**: ${x.toFixed(0)}, ${y.toFixed(0)}`;
}
```

### 2. Carbon Sink Estimation

```js
viewof aoiText = Inputs.textarea({
  label: "AOI (GeoJSON polygon)",
  value: `{"type":"Polygon","coordinates":[[[100,30],[100,31],[101,31],[101,30],[100,30]]]}`
})

{
  const aoi = JSON.parse(aoiText);
  const engine = new geo.CarbonEngine();
  const result = engine.calculate_sink(aoi, { year: 2026 });
  return Plot.barY(result.breakdown, {
    x: "type",
    y: "sink_tco2_yr",
    fill: "steelblue"
  }).plot();
}
```

### 3. GeoJSON → MVT Tile

```js
{
  const engine = new geo.TileEngine();
  const mvt = engine.geojson_to_mvt(sampleGeoJSON, { z: 10, x: 853, y: 372 });

  // mvt is a Uint8Array that can be served as application/vnd.mapbox-vector-tile
  return md`MVT byte size: **${mvt.length}** bytes`;
}
```

### 4. NDVI from Sentinel-2

```js
viewof ndviRed = Inputs.range([0, 1], { value: 0.12, step: 0.01, label: "Red band value" })
viewof ndviNir = Inputs.range([0, 1], { value: 0.45, step: 0.01, label: "NIR band value" })

{
  const ndvi = (ndviNir - ndviRed) / (ndviNir + ndviRed);
  return md`**NDVI** = ${ndvi.toFixed(3)} ${ndvi > 0.5 ? '🟢 Healthy' : ndvi > 0.2 ? '🟡 Moderate' : '🔴 Degraded'}`;
}
```

## Full Module API

```js
geo = import("geo-wasm")

// CRS Engine
const crs = new geo.CrsEngine();
crs.transformPoint(lon, lat, fromEpsg, toEpsg);  // → [x, y]
crs.listCrses();                                  // → string[]
crs.validateCoord(lon, lat);

// Carbon Engine
const carbon = new geo.CarbonEngine();
carbon.calculate_sink(aoi, params);               // → { totalSink_tco2_yr, breakdown }
carbon.ipcc_tier1(area_ha, factor);               // → f64

// Tile Engine
const tile = new geo.TileEngine();
tile.geojson_to_mvt(geojson, zxy);                // → Uint8Array
tile.latlon_to_tile(lat, lon, z);                 // → { x, y }
tile.tile_bounds(z, x, y);                        // → [minLon, minLat, maxLon, maxLat]

// Spatial Engine (future)
const spatial = new geo.SpatialEngine();
spatial.buffer(geojson, distance_m);              // → GeoJSON
spatial.intersect(geojsonA, geojsonB);             // → GeoJSON
```
