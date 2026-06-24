# maplibre-gl-geo-toolbox

MapLibre GL JS plugin — browser-side geospatial analysis powered by
[geo-toolbox](https://github.com/user/geo-toolbox) WebAssembly.

Zero server dependency: CRS transforms, carbon math, tile encoding, and spatial
analysis all run in the user's browser.

## Install

```bash
npm install maplibre-gl-geo-toolbox
```

## Quick start

```js
import maplibregl from 'maplibre-gl';
import { GeoMaplibrePlugin } from 'maplibre-gl-geo-toolbox';

const map = new maplibregl.Map({ /* ... */ });

// Attach geo-toolbox WASM engine
const geo = new GeoMaplibrePlugin(map);

// Transform coordinates on the fly
geo.transformLayer('source-id', {
  from: 'EPSG:4326',
  to: 'EPSG:3857'
});

// Add MVT tiles from GeoJSON
geo.addLocalTileLayer({
  id: 'analysis-layer',
  source: data,                        // GeoJSON FeatureCollection
  maxZoom: 14
});

// Compute NDVI from source raster
geo.computeNdvi('sentinel-source', {
  redBand: 3,
  nirBand: 4,
  output: 'ndvi-layer'
});
```

## API

### `new GeoMaplibrePlugin(map, options?)`

Create a geo-toolbox plugin bound to a MapLibre map instance.

Options:
- `wasmPath` — path to geo-wasm .wasm file (default: `'./pkg/geo_wasm_bg.wasm'`)
- `worker` — use Web Worker for heavy computation (default: `true`)

### Methods

| Method | Description |
|--------|-------------|
| `.transformLayer(sourceId, { from, to })` | Reproject vector source CRS |
| `.addLocalTileLayer({ id, source, maxZoom })` | Create MVT layer from GeoJSON |
| `.computeNdvi(sourceId, { redBand, nirBand, output })` | Compute NDVI raster |
| `.computeCarbonSink(aoi, params)` | IPCC carbon sink estimation |
| `.getCrsEngine()` | Raw CRS engine access |
| `.getCarbonEngine()` | Raw carbon engine access |
| `.getTileEngine()` | Raw tile engine access |

## Web Worker

Heavy computations (raster ops, carbon math) run in a Web Worker by default.
The main thread stays responsive for map interaction.

## ObservableHQ

```js
// Observable notebook:
geo = import("maplibre-gl-geo-toolbox")
```

See `examples/observablehq-example.md` for a full notebook.
