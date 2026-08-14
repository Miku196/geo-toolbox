# geo-toolbox

**Rust 地理空间工具链** — AI Agent 与地理空间重工具之间的高性能胶水层。

将 PostGIS、GEE、QGIS、GDAL 等重型 GIS 工具串联成自动化管线：
数据采集 → 入库存储 → 遥感分析 → 碳核算 → 成果输出。

单仓库（monorepo）承载「一个基座 + 三个方向」：

| 方向 | 载体 | 形态 | 说明 |
|------|------|------|------|
| 基座 | `core/` + `plugins/` + `adapters/` | Rust crates | 五层洋葱架构，240 个工具 |
| ① AI 边缘计算 | `crates/geo-cli` | CLI 二进制 | minimal feature 裁剪，仅 14MB |
| ② WASM 浏览器离线 | `crates/geo-wasm` + `bindings/maplibre` + `apps/field-pwa` | npm 包 / PWA | 数据不出浏览器 |
| ③ GeoAgent | `crates/geo-agent` | HTTP 网关 | 自然语言 → JSON 工具调用 |

[![Rust](https://img.shields.io/badge/rust-1.80+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Tools](https://img.shields.io/badge/tools-240-blue.svg)]()
[![MCP Tools](https://img.shields.io/badge/mcp-89%20tools-blue.svg)]()

---

## 架构

五层洋葱架构（Core → Facade → Plugin → Wiring → Adapter），依赖严格单向：

```
┌──────────────────────────────────────────────────────────┐
│                      geo-toolbox                         │
├──────────────────────────────────────────────────────────┤
│  Layer 1: Core (15 crates) — 纯 Rust，零外部依赖         │
│  几何/CRS · 抽象 trait · 栅格 · 矢量 · 瓦片 · 时序 ·     │
│  索引 · 统计 · IO · 报告 · 碳核算 · GeoParquet · OGC ·   │
│  排放因子 · Facade · Registry                            │
├──────────────────────────────────────────────────────────┤
│  Layer 2: Plugins (18 crates) — 专业领域                 │
│  碳核算 · 生态 · 测绘 · 城乡规划 · 水文 · 地灾 ·          │
│  农业 · 能源 · 林业 · 海岸带 · 遥感 · 气候 · 地貌 ·      │
│  地震 · 社会经济 · 大气 · 火山 · 地下水                  │
├──────────────────────────────────────────────────────────┤
│  Layer 3: Wiring (geo-wiring) — DI 依赖注入组合根         │
│  唯一允许同时依赖 Plugin 和 Adapter 的工厂                │
├──────────────────────────────────────────────────────────┤
│  Layer 4: Adapters (5 crates) — 外部桥接，Feature-gated   │
│  geo-adapters-geo (PostGIS/GDAL/PDAL/GEE) ·              │
│  geo-adapters-io (DuckDB/STAC/OSM/CAD) ·                 │
│  geo-adapters-sim (MODFLOW/DSSAT/IoT) · QGIS · PyO3      │
├──────────────────────────────────────────────────────────┤
│  Layer 5: 消费端 — CLI / WASM / Agent / Server / 绑定     │
└──────────────────────────────────────────────────────────┘
```

核心原则：依赖单向 · WASM 数据不出网 · Rust 做胶水 Python 做重活 · Feature flags 控制依赖
---

## 三个方向

### ① AI 边缘计算 — geo-cli

裁剪到 minimal feature 后仅 **14MB** 的离线 CLI，适合边缘设备部署：

```bash
cargo build --release --no-default-features --features minimal -p geo-cli

# 子命令模式
geo carbon assess input.geojson
geo hydro basin dem.tif

# Unix 管道模式
geo pipeline read input.geojson | geo buffer 500 | geo write output.geojson
```

### ② WASM 浏览器离线 — geo-wasm

Rust 核心编译到 WASM，数据全程不出浏览器：

```bash
wasm-pack build --target web crates/geo-wasm --out-dir ../../pkg --out-name geo_wasm
```

```typescript
import { CrsEngine, CarbonEngine } from 'geo-wasm';

const crs = new CrsEngine();
const [x, y] = await crs.transform(4326, 3857, 104.06, 30.57);

const carbon = new CarbonEngine();
const report = await carbon.calculate(geojson, factorsCsv, 2025);
```

配套：[MapLibre GL JS 插件](bindings/maplibre-gl-geo-toolbox/README.md) · [野外作业 PWA](apps/field-pwa/) · [ObservableHQ 示例](docs/observablehq/README.md)

### ③ GeoAgent — geo-agent

LLM 网关把自然语言请求转为 JSON 工具调用，离线时降级到关键词路由：

```bash
cargo run --release -p geo-agent
curl -X POST http://127.0.0.1:3000/agent \
  -H "Content-Type: application/json" \
  -d '{"query": "计算这个区域的 NDVI，红色波段是第3波段，近红波段是第4波段"}'
```

详见 [crates/geo-agent/README.md](crates/geo-agent/README.md)。

---

## 快速开始

```bash
# 轻量编译（无需外部依赖）
cargo build --release --no-default-features --features minimal

# 运行全部测试
cargo test --workspace
```

### 按需编译

```bash
cargo build --release                    # 全功能（需 GDAL/QGIS）
cargo build --features minimal,postgis   # + PostGIS
cargo build --features qgis              # + QGIS
```

### 编译要求

| 组件 | 版本 | 说明 |
|------|:--:|------|
| Rust | 1.80+ | rustup.rs |
| wasm-pack | 0.13+ | WASM 需要 |
| PostgreSQL | 15+ | PostGIS 适配器需要 |
| GDAL | 3.8+ | GDAL 适配器需要 |
| QGIS | 3.34+ | QGIS 适配器需要 |

---

## 工具（240 tools · 89 MCP tools）

geo-toolbox 内置 MCP Server + HTTP API + WMS，所有工具可直接被 AI Agent 调用。

**空间计算 (Core)**: `crs_list`, `crs_transform`, `tile_latlon_to_tile`, `tile_bounds`, `geohash_encode/decode/neighbors`, `rtree_query`, `quadtree_query`, `vector_buffer/simplify/kde/density/intersect/area/centroid`, `temporal_trend`, `zonal_stats`, `tpi_compute`, `tri_compute`, `hillshade`, `resample`

**碳核算**: `carbon_calculate_raw`, `carbon_calculate_geojson`, `carbon_uncertainty`, `report_carbon`, `report_render`

**专业插件**: `ecology_assess/rusle/musle`, `energy_solar/wind/geothermal/transmission`, `forestry_carbon_stock/site_classify`, `coastal_shoreline/bruun/bathtub`, `hydro_inundation/runoff/strahler/scs`, `geohazard_landslide/fs/newmark`, `agri_yield/soil`, `urban_far`

**遥感**: `remote_toa_radiance/full_pipeline/cloud_mask`, `remote_insar_coherence/full/displacement_class`

**数据接入**: `ingest_camofox/nmea`, `duckdb_query/ingest_geojson`, `stac_search`, `osm_query_bbox`, `store_query/migrate`

**外部桥接**: `qgis_buffer/reproject`, `cli_cog_convert/ogr2ogr`, `gee_classify/status`, `cad_export_geojson`, `dvc_snapshot/hash`, `tile_encode_mvt`

> 完整工具清单见 `crates/geo-agent/tools_schema.json`（240 个，由 `scripts/generate_agent_schemas.py` 生成）
> 启动 MCP Server: `geo-toolbox mcp-serve`
---

## 仓库布局

```
geo-toolbox/
├── core/          # 基座算法（15 crates，含 geo-facade / geo-registry）
├── plugins/       # 领域插件（18 crates）
├── adapters/      # 外部桥接（5 crates，feature-gated）
├── crates/        # 入口：geo-cli ① · geo-wasm ② · geo-agent ③ · geo-server · geo-wiring
├── bindings/      # Python (PyO3) / MapLibre GL JS / Jupyter / QGIS
├── apps/          # field-pwa 野外作业 PWA（方向二）
├── examples/      # chengdu-carbon / china-risk-assessment / dexing-copper / maplibre-carbon
├── contrib/       # 归档插件（8 个，已移出 workspace）
├── fuzz/          # Fuzz 测试目标（geo-fuzz）
├── scripts/       # CI / Git Hook / Schema 生成
└── docs/          # ObservableHQ 等补充文档
```

## 文档

- [使用指南 (WIKI)](WIKI.md) — API 参考、插件开发、适配器集成、FAQ、术语表
- [开发路线图 (ROADMAP)](ROADMAP.md) — 现状与计划
- [仓库边界 (BOUNDARY)](BOUNDARY.md) — 单仓决策与工程约定

## License

MIT

