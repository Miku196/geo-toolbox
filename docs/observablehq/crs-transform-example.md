# ObservableHQ Notebook: CRS 坐标变换

> 在浏览器中实现 WGS84 ↔ Beijing 1954 ↔ CGCS2000 ↔ Web Mercator ↔ UTM 等坐标系自由转换。

---

## Step 1: Import

```js
geo = require("@geo-toolbox/wasm")
```

## Step 2: 列出支持的坐标系

```js
supportedCRS = geo.crs_list()
```

## Step 3: 单点变换

```js
// 北京 1954 → WGS84
bj54Coords = [[116.4, 39.9]]  // 北京天安门 (BJ54)
wgs84Coords = geo.crs_transform(
  bj54Coords.map(([lng, lat]) => ({ x: lng, y: lat })),
  "EPSG:4214",  // Beijing 1954
  "EPSG:4326"   // WGS84
)
wgs84Coords  // => [{ x: 116.397, y: 39.908 }]
```

## Step 4: 批量变换

```js
// 批量转换城市坐标
cities = [
  { name: "北京", bj54: [116.4, 39.9] },
  { name: "上海", bj54: [121.47, 31.23] },
  { name: "广州", bj54: [113.26, 23.13] },
  { name: "成都", bj54: [104.07, 30.67] },
  { name: "乌鲁木齐", bj54: [87.62, 43.82] }
]

transformed = cities.map(city => {
  const [wgsLng, wgsLat] = geo.crs_transform(
    [city.bj54.map((v, i) => i === 0 ? { x: v, y: 0 } : { x: 0, y: v })[0]],
    "EPSG:4214",
    "EPSG:4326"
  );
  return { ...city, wgs84: wgsLng };
})
```

## Step 5: WGS84 → Web Mercator (瓦片地图)

```js
// WGS84 → EPSG:3857 (Web Mercator)
wgs84_bbox = [[113.7, 22.5], [114.3, 23.0]]  // 深圳城区

mercator = wgs84_bbox.map(([lng, lat]) => {
  const [result] = geo.crs_transform(
    [{ x: lng, y: lat }],
    "EPSG:4326",
    "EPSG:3857"
  );
  return [result.x, result.y];
})

// 显示坐标系差异
({ wgs84: wgs84_bbox, mercator })
```

## Step 6: 自动 UTM 分区

```js
// 按经度自动选择 UTM 分区
viewof autoUTM = {
  const zones = [];
  for (let lon = 73; lon <= 135; lon += 5) {
    const zone = Math.floor((lon + 180) / 6) + 1;
    const epsg = 32600 + zone;
    zones.push({ lon, zone, epsg: `EPSG:${epsg}` });
  }
  return Inputs.table(zones, {
    columns: ["lon", "zone", "epsg"],
    header: { lon: "经度 (°)", zone: "UTM 分区", epsg: "EPSG 代码" }
  });
}
```

## Step 7: 交互式坐标变换器

```js
viewof inputLng = Inputs.range([73, 135], { step: 0.1, value: 116.4, label: "经度" })
viewof inputLat = Inputs.range([18, 54], { step: 0.1, value: 39.9, label: "纬度" })
```

```js
viewof fromCRS = Inputs.select(supportedCRS, { value: "EPSG:4326", label: "来源坐标系" })
viewof toCRS = Inputs.select(supportedCRS, { value: "EPSG:3857", label: "目标坐标系" })
```

```js
transformedCoords = geo.crs_transform(
  [{ x: inputLng, y: inputLat }],
  fromCRS,
  toCRS
)
```

```js
md`**${fromCRS}** (${inputLng.toFixed(2)}, ${inputLat.toFixed(2)}) → **${toCRS}** (${transformedCoords[0].x.toFixed(2)}, ${transformedCoords[0].y.toFixed(2)})`
```

## 常用 EPSG 速查

| EPSG | 名称 | 用途 |
|------|------|------|
| 4326 | WGS84 | GPS/GIS 默认球坐标 |
| 3857 | Web Mercator | 网络瓦片地图 |
| 4214 | Beijing 1954 | 中国历史数据 |
| 4490 | CGCS2000 | 中国现行大地坐标系 |
| 4525-4548 | CGCS2000 3° GK | 中国大比例尺测绘 |
| 32601-32660 | WGS84 UTM (北纬) | 全球通用投影 |
| 32701-32760 | WGS84 UTM (南纬) | 全球通用投影 |

## 精度说明

- CGCS2000 ↔ WGS84：差异 < 1 cm（实际等同）
- Beijing 1954 → WGS84：偏差 50-120 m（取决于区域转换参数）
- Web Mercator → WGS84：无精度损失（投影变换，精确可逆）
