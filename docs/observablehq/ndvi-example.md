# ObservableHQ Notebook: NDVI 植被指数计算

> 在浏览器中加载 COG (Cloud-Optimized GeoTIFF)，用 WASM 计算 NDVI 植被指数。

---

## Step 1: 加载 COG 栅格

```js
// 从 STAC API 或本地加载 COG URL
sentinel2Url = "https://sentinel-cogs.s3.us-west-2.amazonaws.com/sentinel-s2-l2a-cogs/50/S/PC/2024/S2A_50SPC_20240815_0_L2A/B04.tif"
// B04 = Red, B08 = NIR（需要两块）

// 也可以从本地文件选择
// viewof fileInput = Inputs.file({ accept: ".tif,.tiff", multiple: true })
```

```js
geo = require("@geo-toolbox/wasm")

// Fetch COG into WASM memory
redBand = await fetch(sentinel2Url)
  .then(r => r.arrayBuffer())
  .then(buf => new Uint8Array(buf))

nirUrl = sentinel2Url.replace("B04", "B08")
nirBand = await fetch(nirUrl)
  .then(r => r.arrayBuffer())
  .then(buf => new Uint8Array(buf))
```

## Step 2: 计算 NDVI

```js
ndviResult = geo.raster_ndvi({
  red: redBand,
  nir: nirBand,
  method: "standard"  // "standard" | "savi" | "evi"
})
```

```js
// NDVI 统计
ndviStats = {
  mean: ndviResult.mean,
  std: ndviResult.std,
  min: ndviResult.min,
  max: ndviResult.max,
  // 分级面积 (ha)
  healthy: ndviResult.healthy_area_ha,    // NDVI > 0.5
  moderate: ndviResult.moderate_area_ha,   // 0.2 < NDVI ≤ 0.5
  degraded: ndviResult.degraded_area_ha    // NDVI ≤ 0.2
}
```

## Step 3: NDVI 直方图

```js
Plot.plot({
  marks: [
    Plot.rectY(ndviResult.histogram, {
      x1: d => d.bin_start,
      x2: d => d.bin_end,
      y: "count",
      fill: d => {
        const v = (d.bin_start + d.bin_end) / 2;
        if (v < 0.2) return "#d73027";
        if (v < 0.5) return "#fdae61";
        return "#1a9641";
      },
      tip: true
    })
  ],
  x: { label: "NDVI 值", domain: [-1, 1] },
  y: { label: "像元数", grid: true }
})
```

## Step 4: 分类结果表

```js
viewof ndviTable = Inputs.table([
  { class: "健康植被 (NDVI > 0.5)", area: ndviStats.healthy, pct: ndviStats.healthy / (ndviStats.healthy + ndviStats.moderate + ndviStats.degraded) * 100 },
  { class: "中等植被 (0.2 < NDVI ≤ 0.5)", area: ndviStats.moderate, pct: ndviStats.moderate / (ndviStats.healthy + ndviStats.moderate + ndviStats.degraded) * 100 },
  { class: "退化植被 (NDVI ≤ 0.2)", area: ndviStats.degraded, pct: ndviStats.degraded / (ndviStats.healthy + ndviStats.moderate + ndviStats.degraded) * 100 },
], {
  columns: ["class", "area", "pct"],
  header: { class: "植被等级", area: "面积 (ha)", pct: "占比 (%)" },
  format: { area: d3.format(".1f"), pct: d3.format(".1f") }
})
```

## 进阶：多时相 NDVI 趋势

```js
// 加载 5 年同一季节的 NDVI 序列
years = [2020, 2021, 2022, 2023, 2024]
ndviSeries = await Promise.all(years.map(async year => {
  const url = `https://sentinel-cogs.s3.us-west-2.amazonaws.com/sentinel-s2-l2a-cogs/50/S/PC/${year}/S2A_50SPC_${year}0815_0_L2A/`
  const red = await fetch(url + "B04.tif").then(r => r.arrayBuffer()).then(b => new Uint8Array(b))
  const nir = await fetch(url + "B08.tif").then(r => r.arrayBuffer()).then(b => new Uint8Array(b))
  const ndvi = geo.raster_ndvi({ red, nir, method: "standard" })
  return { year, mean: ndvi.mean, std: ndvi.std }
}))
```

```js
// 趋势图
Plot.plot({
  marks: [
    Plot.line(ndviSeries, { x: "year", y: "mean", stroke: "#1a9641", strokeWidth: 2 }),
    Plot.dot(ndviSeries, { x: "year", y: "mean", fill: "#1a9641" }),
    Plot.areaY(ndviSeries, {
      x: "year",
      y1: d => d.mean - d.std,
      y2: d => d.mean + d.std,
      fillOpacity: 0.15,
      fill: "#1a9641"
    })
  ],
  x: { label: "年份", tickFormat: d3.format("d") },
  y: { label: "平均 NDVI", grid: true }
})
```

## 与 MapLibre GL JS 联动

```js
// 将 NDVI 结果渲染到地图上
import maplibregl from "maplibre-gl"

map = {
  const div = DOM.element("div");
  div.style.height = "400px";
  const map = new maplibregl.Map({
    container: div,
    style: "https://demotiles.maplibre.org/style.json",
    center: [117.2, 42.4],
    zoom: 10
  });
  map.on("load", () => {
    map.addSource("ndvi", {
      type: "image",
      url: URL.createObjectURL(new Blob([ndviResult.colored_tiff], { type: "image/tiff" })),
      coordinates: ndviResult.bounds  // [west, north, east, south]
    });
    map.addLayer({
      id: "ndvi-overlay",
      type: "raster",
      source: "ndvi",
      paint: { "raster-opacity": 0.7 }
    });
  });
  return div;
}
```
