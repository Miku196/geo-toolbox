# ObservableHQ Notebook: 碳汇计算 (Carbon Sink)

> 在浏览器中计算 IPCC 5 碳库（地上生物量 / 地下生物量 / 枯死木 / 枯落物 / 土壤有机碳）的碳汇量。

---

## Step 1: Import geo-toolbox

```js
geo = require("@geo-toolbox/wasm")
```

## Step 2: 定义 AOI 与参数

```js
aoi = {
  type: "Feature",
  properties: { name: "河北塞罕坝林场" },
  geometry: {
    type: "Polygon",
    coordinates: [[
      [117.2, 42.4],
      [117.5, 42.4],
      [117.5, 42.6],
      [117.2, 42.6],
      [117.2, 42.4]
    ]]
  }
}
```

```js
// 碳密度参数（中国温带落叶松人工林，IPCC 默认值）
carbonParams = {
  agb: 92.3,    // 地上生物量 (t/ha)
  bgb: 23.1,    // 地下生物量 (t/ha) — R:S = 0.25
  deadwood: 8.5, // 枯死木 (t/ha)
  litter: 12.2,  // 枯落物 (t/ha)
  soc: 85.0,     // 土壤有机碳 (t/ha)
  carbonFraction: 0.47
}
```

## Step 3: 计算各碳库

```js
fivePoolCarbon = geo.carbon_five_pool({
  area_ha: geo.vector_area(JSON.stringify(aoi)) / 10000,
  agb_t_per_ha: carbonParams.agb,
  bgb_t_per_ha: carbonParams.bgb,
  deadwood_t_per_ha: carbonParams.deadwood,
  litter_t_per_ha: carbonParams.litter,
  soc_t_per_ha: carbonParams.soc,
  carbon_fraction: carbonParams.carbonFraction
})
```

## Step 4: 可视化结果

```js
// 表格展示
viewof poolTable = {
  const pools = [
    { name: "地上生物量 (AGB)", carbon: fivePoolCarbon.agb_tco2 },
    { name: "地下生物量 (BGB)", carbon: fivePoolCarbon.bgb_tco2 },
    { name: "枯死木 (Deadwood)", carbon: fivePoolCarbon.deadwood_tco2 },
    { name: "枯落物 (Litter)", carbon: fivePoolCarbon.litter_tco2 },
    { name: "土壤有机碳 (SOC)", carbon: fivePoolCarbon.soc_tco2 },
    { name: "合计", carbon: fivePoolCarbon.total_tco2 }
  ];
  return Inputs.table(pools, {
    columns: ["name", "carbon"],
    header: { name: "碳库", carbon: "tCO₂" },
    format: { carbon: d3.format(".2f") }
  });
}
```

```js
// 柱状图
Plot.plot({
  marks: [
    Plot.barY(poolTable.slice(0, 5), {
      x: "name",
      y: "carbon",
      fill: d3.scaleOrdinal(["#2c7bb6", "#abd9e9", "#fdae61", "#f46d43", "#d73027"]),
      sort: { x: "y", reverse: true },
      tip: true
    })
  ],
  x: { label: "碳库" },
  y: { label: "tCO₂", grid: true }
})
```

## 结果解读

- **合计碳储量 > 1000 tCO₂** → 中大型碳汇项目规模
- **AGB 占比 < 60%** → 需检查地下生物量比例（通常 R:S 0.2-0.3）
- **SOC 在总量中 > 40%** → 森林土壤是最大碳库，不可忽视

## 进阶：不确定性分析

```js
// 蒙特卡洛 1000 次
import { normal } from "@jstat/normal"

n = 1000
samples = Array.from({ length: n }, () => {
  return geo.carbon_five_pool({
    area_ha: 500,
    agb_t_per_ha: normal.sample(92.3, 9.2),  // CV=10%
    bgb_t_per_ha: normal.sample(23.1, 3.5),
    deadwood_t_per_ha: normal.sample(8.5, 1.7),
    litter_t_per_ha: normal.sample(12.2, 1.8),
    soc_t_per_ha: normal.sample(85.0, 12.8),
    carbon_fraction: normal.sample(0.47, 0.02)
  }).total_tco2
})

// 95% 置信区间
samples.sort((a, b) => a - b)
ci95 = {
  lower: samples[Math.floor(n * 0.025)],
  median: samples[Math.floor(n * 0.5)],
  upper: samples[Math.floor(n * 0.975)]
}
```
