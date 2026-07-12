# geo-toolbox

**Rust 地理空间工具链** — AI Agent 与地理空间重工具之间的高性能胶水层。

将 PostGIS、GEE、QGIS、GDAL 等重型 GIS 工具串联成自动化管线：
数据采集 → 入库存储 → 遥感分析 → 碳核算 → 成果输出。

采用 **Core → Plugin → Adapter 三层架构**：Rust 负责性能敏感路径，
遥感计算和空间分析委托 Python 生态。

[![Rust](https://img.shields.io/badge/rust-1.80+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-1000+-pass-green.svg)]()
[![MCP Tools](https://img.shields.io/badge/mcp-89%20tools-blue.svg)]()

---

## 架构

```
┌───────────────────────────────────────────────────┐
│                   geo-toolbox                     │
├───────────────────────────────────────────────────┤
│  Layer 1: Core (14 crates) — 纯 Rust，零外部依赖  │
│  几何/CRS · 栅格 · 矢量 · 瓦片 · 时序 · 索引 ·    │
│  统计 · IO · 报告 · 碳核算 · GeoParquet · OGC     │
├───────────────────────────────────────────────────┤
│  Layer 2: Plugins (26 crates) — 专业领域          │
│  碳核算 · 生态 · 测绘 · 城乡规划 · 水文 · 地灾 ·  │
│  农业 · 能源 · 林业 · 海岸带 · 遥感 · 气候 · 地貌 │
├───────────────────────────────────────────────────┤
│  Layer 3: Adapters (13 crates) — 外部桥接         │
│  PostGIS · GEE · QGIS · DuckDB · STAC · OSM ·     │
│  CAD · GDAL · IoT · DSSAT · MODFLOW · PDAL        │
└───────────────────────────────────────────────────┘
依赖方向（严格单向）：Adapter → Plugin → Core
```

核心原则：依赖单向、WASM 数据不出网、Rust 做胶水 Python 做重活、Feature flags 控制依赖。

---

## 快速开始

```bash
# 轻量编译（无需外部依赖）
cargo build --release --no-default-features --features minimal

# 列出所有坐标系
cargo run -- crs list

# 运行全部测试
cargo test --workspace
```

### 按需编译

```bash
cargo build --release                    # 全功能（需 GDAL/QGIS）
cargo build --features minimal,postgis   # + PostGIS
cargo build --features qgis              # + QGIS
```

---

## 编译指南

| 组件 | 版本 | 说明 |
|------|:--:|------|
| Rust | 1.80+ | rustup.rs |
| wasm-pack | 0.13+ | WASM 需要 |
| PostgreSQL | 15+ | PostGIS 适配器需要 |
| GDAL | 3.8+ | GDAL 适配器需要 |
| QGIS | 3.34+ | QGIS 适配器需要 |

```bash
# WASM 编译
wasm-pack build --target web crates/geo-wasm --out-dir ../../pkg --out-name geo_wasm
```

---

## MCP 工具一览（89 tools）

geo-toolbox 内置 MCP Server + HTTP API + WMS，所有工具可直接被 AI Agent 调用。

**空间计算 (Core)**: `crs_list`, `crs_transform`, `tile_latlon_to_tile`, `tile_bounds`, `geohash_encode/decode/neighbors`, `rtree_query`, `quadtree_query`, `vector_buffer/simplify/kde/density/intersect/area/centroid`, `temporal_trend`, `zonal_stats`, `tpi_compute`, `tri_compute`, `hillshade`, `resample`

**碳核算**: `carbon_calculate_raw`, `carbon_calculate_geojson`, `carbon_uncertainty`, `report_carbon`, `report_render`

**专业插件**: `ecology_assess/rusle/musle`, `energy_solar/wind/geothermal/transmission`, `forestry_carbon_stock/site_classify`, `coastal_shoreline/bruun/bathtub`, `hydro_inundation/runoff/strahler/scs`, `geohazard_landslide/fs/newmark`, `agri_yield/soil`, `urban_far`

**遥感**: `remote_toa_radiance/full_pipeline/cloud_mask`, `remote_insar_coherence/full/displacement_class`

**数据接入**: `ingest_camofox/nmea`, `duckdb_query/ingest_geojson`, `stac_search`, `osm_query_bbox`, `store_query/migrate`

**外部桥接**: `qgis_buffer/reproject`, `cli_cog_convert/ogr2ogr`, `gee_classify/status`, `cad_export_geojson`, `dvc_snapshot/hash`, `tile_encode_mvt`

> 启动 MCP Server: `geo-toolbox mcp-serve`

---

## CLI 使用

```bash
# 子命令模式
geo carbon assess input.geojson
geo hydro basin dem.tif

# Unix 管道模式
geo pipeline read input.geojson | geo buffer 500 | geo write output.geojson
geo pipeline read city.geojson | geo filter key=class value=park | geo area
```

---

## 浏览器端 (WASM)

```bash
npm install geo-wasm
```

```typescript
import { CrsEngine, CarbonEngine } from 'geo-wasm';

const crs = new CrsEngine();
const [x, y] = await crs.transform(4326, 3857, 104.06, 30.57);

const carbon = new CarbonEngine();
const report = await carbon.calculate(geojson, factorsCsv, 2025);
```

---

## 项目结构

```
geo-toolbox/
├── core/                  # 核心引擎 (14 crates)
├── plugins/               # 专业插件 (26 crates)
├── adapters/              # 外部适配器 (13 crates)
├── crates/                # 入口 (CLI / Server / WASM / Wiring)
├── bindings/              # Python / Jupyter / QGIS / MapLibre
├── examples/              # 成都碳收支 / 中国风险评估 / 德兴铜矿
├── fuzz/                  # Fuzz 测试目标
└── docs/                  # 补充文档
```

---

## 三层架构详解

### Layer 1: Core — 纯 Rust 核心引擎

| Crate | 职责 |
|-------|------|
| `geo-core` | 几何基类、CRS、`Plugin` trait、`GeoError` |
| `geo-raster` | 栅格运算、NDVI、地形分析 |
| `geo-vector` | 矢量空间运算 |
| `geo-tile` | MVT/PMTiles 瓦片 |
| `geo-temporal` | 时空序列分析 |
| `geo-index` | GeoHash/R-tree/四叉树 |
| `geo-stats` | 空间统计 |
| `geo-io` | GeoJSON/CSV/NMEA 解析 |
| `geo-carbon-math` | IPCC 碳核算公式 |
| `geo-report` | Tera 报告模板引擎 |
| `geo-parquet` | GeoParquet 云原生格式 |
| `geo-ogc` | WMS/WFS/WPS 标准 |
| `geo-registry` | 插件注册调度中心 |

### Layer 2: Plugins — 专业领域插件

| 插件 | 核心能力 |
|------|---------|
| `geo-plugin-carbon` | 碳核算、LCA、VCS/CCB、CCER报告 |
| `geo-plugin-ecology` | NDVI变化、RUSLE/MUSLE土壤侵蚀、随机森林LULC |
| `geo-plugin-hydro` | SCS-CN径流、InVEST碳+水、流域提取、单位线、地下水 |
| `geo-plugin-energy` | Weibull风能、Jensen尾流、地热、PVWatts、输电走廊 |
| `geo-plugin-geohazard` | 滑坡敏感性、Newmark位移、安全系数FS、降雨ID阈值 |
| `geo-plugin-forestry` | 树高生长曲线、立地等级、碳汇潜力 |
| `geo-plugin-coastal` | Bruun侵蚀、Holland风暴潮、蓝碳、波浪爬高 |
| `geo-plugin-survey` | Gauss-Kruger投影、七参数转换、土方量 |
| `geo-plugin-urban` | 容积率、建筑密度、城市洪水 |
| `geo-plugin-agri` | 作物估产、土壤评级、DSSAT适配 |
| `geo-plugin-remote-sensing` | 辐射校正、InSAR形变 |
| `geo-plugin-climate` | GCM降尺度、IDF曲线、干旱指数、Kriging |
| `geo-plugin-geomorph` | D8流向累积、Strahler河网 |

### Layer 3: Adapters — 外部系统桥接

| 适配器 | 外部系统 | 方式 |
|--------|---------|------|
| `geo-adapter-postgis` | PostgreSQL+PostGIS | sqlx |
| `geo-adapter-gee` | Google Earth Engine | NATS→Python |
| `geo-adapter-qgis` | QGIS | Subprocess/REST |
| `geo-adapter-duckdb` | DuckDB/SQLite | 嵌入式 |
| `geo-adapter-stac` | STAC API | HTTP |
| `geo-adapter-osm` | OpenStreetMap | Overpass API |
| `geo-adapter-cad` | CAD/DXF | 格式导出 |
| `geo-adapter-cli` | GDAL/DVC | 子进程 |
| `geo-adapter-iot` | MQTT传感器 | MQTT |
| `geo-adapter-dssat` | DSSAT作物模型 | 文件生成 |
| `geo-adapter-modflow` | MODFLOW地下水 | 文件生成 |
| `geo-adapter-pdal` | PDAL LiDAR | 管线处理 |
| `geo-adapter-pygeoapi` | PyO3 FFI | WKB↔Shapely |

---

## 开发

```bash
# 全量测试
cargo test --workspace

# benchmark
cargo bench --workspace

# 单 crate
cargo test -p geo-plugin-carbon
```

---

## 示例

- `examples/dexing-copper/` — 德兴铜矿生态修复评估
- `examples/chengdu-carbon/` — 成都碳收支分析
- `examples/china-risk-assessment/` — 中国地质灾害风险评估

---

## 文档

- [使用指南 (WIKI)](WIKI.md) — API 参考、插件开发、适配器集成、FAQ
- [开发路线图 (ROADMAP)](ROADMAP.md) — 进度与计划
- [领域词汇表 (context.md)](context.md) — 架构概念与术语

## License

MIT
