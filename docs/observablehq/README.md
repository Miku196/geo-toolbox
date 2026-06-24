# geo-toolbox + ObservableHQ

在 ObservableHQ 中直接使用 geo-toolbox WASM 模块进行浏览器端地理空间分析。

## 快速开始

ObservableHQ 支持直接 `import` npm 包。将以下代码粘贴到任意 notebook 中：

```js
// Import geo-toolbox WASM
geo_wasm = require("@geo-toolbox/wasm")

// 也可以使用 ESM import（如果 notebook 支持）
// import * as geo_wasm from "@geo-toolbox/wasm"
```

## 可用模块

`@geo-toolbox/wasm` 导出的 WASM 绑定：

| 模块 | 功能 |
|------|------|
| CRS 变换 | 坐标系转换、GeoHash 编解码 |
| 碳核算 | 5 碳库模型计算 |
| 栅格运算 | NDVI、波段运算、重采样 |
| 矢量运算 | 缓冲区、相交、面积、质心 |
| 瓦片工具 | 经纬度 ↔ 瓦片坐标、MVT 编码 |
| 时空分析 | 趋势分析、突变检测 |
| 输入输出 | GeoJSON/NMEA/CamoFox 解析 |

## 示例 Notebook

- [**碳汇计算**](./carbon-sink-example.md) — 在浏览器中计算 IPCC 5 碳库碳汇
- [**NDVI 植被指数**](./ndvi-example.md) — 加载 COG 影像，浏览器端计算 NDVI
- [**CRS 坐标变换**](./crs-transform-example.md) — 坐标系自由转换

## 限制

- WASM 模块约 2-4 MB（首次加载需几秒）
- 栅格处理受浏览器内存限制（建议 < 100 MB 影像）
- 不支持多线程（wasm-bindgen 单线程模式）
- 不支持文件系统访问（数据需通过 fetch/upload 加载）

## 离线使用

```js
// 如果 @geo-toolbox/wasm 已在本地安装
import * as geo from "@geo-toolbox/wasm"

// 或从本地文件加载
const wasmModule = await WebAssembly.instantiateStreaming(
  fetch("./geo_wasm_bg.wasm")
)
```
