# geo-toolbox

**Rust 地理空间工具链** — AI Agent 与地理空间重工具之间的高性能胶水层。

将 PostGIS、GEE、QGIS、GDAL 等重型 GIS 工具串联成一条自动化管线：
数据采集 → 入库存储 → 遥感分析 → 碳核算 → 成果输出。

不替代任何现有工具。Rust 负责性能敏感路径（批写、格式转换、消息分发、碳核算），
遥感计算和空间分析仍委托 Python 生态（GEE SDK、PyQGIS、GDAL CLI、brightway2）。

[![Rust](https://img.shields.io/badge/rust-1.80+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-51%20pass-green.svg)]()
[![NPM](https://img.shields.io/badge/npm-geo--wasm-red)](https://www.npmjs.com/package/geo-wasm)

---

## 目录

- [是什么](#是什么)
- [快速开始](#快速开始)
- [架构](#架构)
- [🌐 浏览器端 (WASM)](#浏览器端-wasm)
- [📦 NPM 包](#npm-包)
- [Crate 详解](#crate-详解)
- [部署](#部署)
- [使用手册](#使用手册)
- [Feature Flags](#feature-flags)
- [消息队列](#消息队列)
- [开发](#开发)
- [示例](#示例)
- [文档](#文档)

---

## 是什么

一个 Rust workspace，13 个 crate，约 15,000 行代码，覆盖地理空间数据管线的全生命周期 — 从浏览器端到服务端：

```
┌── 浏览器端 (WASM) ── 数据不出网 ──────────────────────────┐
│  CRS 变换  NMEA 解析  碳核算(IPCC)  空间运算              │
│  GeoJSON/Excel/DXF 导出  IndexedDB 本地存储               │
└──────────────────────────────────────────────────────────┘
                              │
┌── 服务端管线 ────────────────────────────────────────────┐
│                                                         │
│ 数据采集          入库存储          处理分析          成果输出 │
│    │                │                │                │   │
│  CamoFox 网页抓取  PostGIS 矢量     GEE 遥感分类     DXF CAD │
│  NMEA GPS 解析     TimescaleDB 时序  GDAL 栅格转换    Excel  │
│  MQTT 传感器流     MinIO/S3 栅格    QGIS 空间分析     GeoJSON │
│                    GeoParquet 云原生  WMS/WFS/WPS OGC  Markdown │
│                    DVC 版本控制      LCA 碳足迹              │
└──────────────────────────────────────────────────────────┘
```

### 核心设计原则

- **浏览器端 WASM — 敏感数据不出网** — 所有计算在浏览器内完成，GeoJSON 拖入 → 碳核算 → 报告下载，零数据外传
- **Rust 做胶水，Python 做重活** — GEE 的 Random Forest 分类、QGIS 的 buffer/overlay 仍用 Python，Rust 只负责任务分发、结果搬运、格式转换
- **每个 crate 独立可测** — 不依赖数据库也能跑大多数测试；集成测试用 Docker Compose
- **Feature flags 控制依赖** — 不需要 GDAL/QGIS 就别编译进去，最小化二进制体积
- **MCP 原生集成** — 内置 MCP Server，AI Agent 通过 JSON-RPC 直接调用所有功能
- **OGC 标准互操作** — WMS/WFS/WPS 纯 Rust 实现，任何 GIS 客户端可直接接入
- **云原生格式** — GeoParquet 读写 + 空间谓词下推，直达 Arrow/DataFusion 管道
- **纯 Rust CRS 变换 — 零 C 依赖** — 4326↔3857 (WGS84↔Web Mercator)、4326↔9000 (WGS84↔GCJ-02 火星)、9000↔9001 (GCJ-02↔BD-09 百度)、4326→3405 (等积)，无需 cmake/proj

---

## 快速开始

```bash
git clone https://github.com/Miku196/geo-toolbox.git
cd geo-toolbox

# 编译（Rust 1.80+，无需 cmake/proj）
cargo build --release

# 列出坐标系
cargo run -- crs list

# 坐标变换 — 纯 Rust，零 C 依赖
cargo run -- crs transform --from 4326 --to 3857 104.06 30.57
cargo run -- crs transform --from 4326 --to 9000 116.40 39.90   # WGS84 → GCJ-02 火星坐标

# 批量变换（stdin）
echo "104.06,30.57" | cargo run -- crs transform --from 4326 --to 3857 --batch

# 本地 GeoJSON 压缩/重投影
cargo run -- output geojson --from-file input.geojson --output compact.geojson
cargo run -- output geojson --from-file input.geojson --output merc.geojson --to-epsg 3857

# 运行测试（20 tests, 0 deps）
cargo test -p geo-core
```

### 自包含 HTML 报告

```bash
cd examples/china-risk-assessment
python self_contained_report.py
# 生成 output/*.html — 双击即开，地图 + 统计 + 交互式 CRS 变换，零服务器
```

### 中国自然灾害风险评估

```bash
cd examples/china-risk-assessment
python flood_risk_pipeline.py    # 洪水高风险区 GIS 评估
python earthquake_pipeline.py    # 地震活动 GIS 评估
```
详见 [examples/china-risk-assessment/README.md](examples/china-risk-assessment/README.md)。

### WASM 浏览器端

```bash
cd geo-toolbox
python -m http.server 8899
# 打开 http://127.0.0.1:8899/demo.html
```

---

## 架构

```
                        ┌── 纯 Rust 路径 ───────────────────┐
AI Agent ──MCP──→ geo-toolbox ──┬──→ PostGIS (COPY 批写)
                                 ├──→ MinIO (COG 读写)
                                 ├──→ TimescaleDB (超表)
                                 ├──→ CRS 转换 (PROJ FFI)
                                 ├──→ 碳核算 (SQL 一条查询)
                                 └──→ DXF/Excel/GeoJSON
                        │                                    │
                        └── 委托 Python 路径 ────────────────┘
                                 ├──→ GEE (NATS → gee-worker Python)
                                 ├──→ QGIS (REST → PyQGIS 长驻服务)
                                 ├──→ GDAL (子进程 gdal_translate/ogr2ogr)
                                 └──→ LCA (子进程 → brightway2)
```

**数据流规则**：AI Agent 永远不持有数据，只传递数据路径、SQL 参数、作业配置 JSON。所有读写统一经过 `geo-store`（PostGIS + MinIO）。

**双通道写入**：矢量/栅格元数据走事务通道（geo-store COPY 批写），GPS/IoT 高频时序流直写 TimescaleDB 超表。

---

## 浏览器端 (WASM)

geo-toolbox 核心功能已编译为 WebAssembly，直接在浏览器中运行。
**所有计算在本地完成，GeoJSON 数据不会上传到任何服务器。**

### 环境准备

```bash
# 1. 安装 WASM 编译目标
rustup target add wasm32-unknown-unknown

# 2. 安装 wasm-pack (如果网络不通，换国内源)
# 方式 A: 直接安装
cargo install wasm-pack

# 方式 B: 换源安装 (ustc/tuna/rsproxy)
# 在 ~/.cargo/config.toml 添加:
# [source.crates-io]
# replace-with = 'ustc'
# [source.ustc]
# registry = 'sparse+https://mirrors.ustc.edu.cn/crates.io-index/'

# 3. 编译 WASM
wasm-pack build --target web crates/geo-wasm

# 编译产物在 crates/geo-wasm/pkg/
# - geo_wasm_bg.wasm  (二进制)
# - geo_wasm.js        (JS 胶水代码)
# - geo_wasm.d.ts      (TypeScript 类型)
```

### 启动 Demo

```bash
# 用任意 HTTP Server 启动（WASM 必须通过 HTTP 加载）
cd crates/geo-wasm
python -m http.server 8080
# 或: npx serve .
# 或: npx http-server -p 8080 -c-1

# 浏览器打开 http://localhost:8080/demo.html
```

### Demo 页面功能

| 面板 | 功能 | 操作 |
|------|------|------|
| CRS 变换 | WGS84 ↔ Web Mercator ↔ GCJ-02 ↔ BD-09 | 输入坐标，实时变换 |
| NMEA 解析 | $GPGGA / $GPRMC 语句 | 粘贴日志，批量解析 |
| 碳核算 | IPCC Tier 1 (子类匹配 + 表头CSV) | 成都开发区真实数据 / 拖入 GeoJSON + 排放因子 CSV |
| 空间运算 | Area / BBox / Centroid / Simplify | 粘贴 GeoJSON 几何，查看结果 |
| 本地存储 | IndexedDB | 持久化 GeoJSON 要素 |

### 使用 NPM 包

```bash
npm install geo-wasm
```

```typescript
import { CrsEngine, CarbonEngine, GeoStore, parseNmea } from 'geo-wasm';

// CRS 变换
const crs = new CrsEngine();
const [x, y] = await crs.transform(4326, 3857, 104.06, 30.57);

// 碳核算 (GeoJSON + CSV → 报告 JSON)
// 支持 header-based CSV (列顺序无关) + subcategory 精细匹配
const engine = new CarbonEngine();
const report = await engine.calculate(
  landcoverGeojsonFC,
  'source,category,subcategory,factor_value,unit,region\n' +
  'IPCC_2019,forest,evergreen_broadleaf,-380.0,tCO2e/ha,CN-51\n' +
  'IPCC_2019,settlement,industrial,480.0,tCO2e/ha/yr,CN-51',
  2025
);
// → { total_area_ha, total_emission_tco2e, classes: [
//     { landcover_class: "forest:evergreen_broadleaf", ... }, ... ] }

// GPS 解析
const fix = parseNmea('$GPGGA,123519,4807.038,N,01131.000,E,1,08,1.2,545.4,M,,,*47');
// → { type: 'GGA', lat: 48.1173, lng: 11.5166, quality: 1, satellites: 8 }

// IndexedDB 本地存储
const store = new GeoStore();
await store.init();
await store.putFeature('aoi-1', aoiFeature);
const all = await store.getAllFeatures();
```

---

## NPM 包

### API 总览

| 类/函数 | 说明 |
|---------|------|
| `CrsEngine` | CRS 坐标变换 (6 个内置 EPSG + 自定义) |
| `CarbonEngine` | IPCC 碳核算引擎 (GeoJSON → 报告) |
| `GeoStore` | IndexedDB 本地要素存储 |
| `parseNmea()` | NMEA 0183 GPS 语句解析 |
| `validateGpsFix()` | GPS 质量校验 (HDOP + 卫星数) |
| `validateCoord()` | 坐标合法性检查 (lon∈[-180,180], lat∈[-90,90]) |
| `computeArea()` | GeoJSON 面积计算 (返回 m² + ha) |
| `computeBbox()` | 几何 Bounding Box |
| `computeCentroid()` | 几何质心 |
| `simplifyGeometry()` | Douglas-Peucker 简化 |
| `convexHull()` | 凸包计算 |
| `exportExcel()` | 数据表 → XLSX 文件 (Uint8Array) |
| `exportGeoJson()` | 要素数组 → FeatureCollection |
| `exportCarbonReport()` | 碳核算结果 → Markdown 报告 |
| `csvToJson()` | CSV 文本 → JSON 数组 |

### geo-core — 共享基础设施（461 行）

整个工作区的基石。所有 crate 都依赖它。

```rust
// 统一错误类型，12 个变体
pub enum GeoError {
    CrsNotFound(u16, u16),
    CrsTransform(String),
    Validation(String),
    Database(String),
    Io(#[from] std::io::Error),
    Serde(#[from] serde_json::Error),
    ObjectStore(String),
    MessageQueue(String),
    GcsBridge(String),
    Csv(String),
    ExternalProcess { command: String, message: String },
    Other(String),
}
pub type GeoResult<T> = Result<T, GeoError>;
```

| 模块 | 功能 |
|------|------|
| `errors.rs` | 统一错误类型，所有 crate 共用 |
| `crs.rs` | CRS 注册表（7 个内置坐标系）、坐标变换、`builtin` 纯 Rust 变换模块 |
| `types.rs` | 几何类型别名、坐标校验 |

内置坐标系：WGS84 (4326)、Web Mercator (3857)、GCJ-02 火星 (9000)、BD-09 百度 (9001)、UTM 49N (32649)、UTM 50N (32650)、等积投影 (3405)。

`builtin` 模块提供 8 个纯 Rust 坐标变换函数（零 C 依赖）：
`wgs84_to_mercator`、`mercator_to_wgs84`、`wgs84_to_gcj02`、`gcj02_to_wgs84`、
`gcj02_to_bd09`、`bd09_to_gcj02`、`wgs84_to_equal_area`。CLI 和 WASM 共享同一份实现。

---

### geo-cli — CLI 入口 + MCP Server（1,417 行）

用户交互的唯一入口。既可以直接命令行调用，也可以作为 MCP Server 供 AI Agent 通过 JSON-RPC 调用。

```bash
geo-toolbox crs list                          # CRS 管理
geo-toolbox ingest camofox data.json           # 数据入库
geo-toolbox store migrate                      # 数据库迁移
geo-toolbox process gee classify --aoi ...     # GEE 分发
geo-toolbox process gdal cog in.tif out.tif    # GDAL 操作
geo-toolbox process qgis buffer --input ...    # QGIS 处理
geo-toolbox carbon emission-factor calculate .. # 碳核算
geo-toolbox output excel "SELECT ..." --output . # 成果导出
geo-toolbox mcp-serve --port 9378              # MCP Server
```

MCP Server 暴露的 tools：`crs_list`、`crs_transform`、`ingest_camofox`、`carbon_calculate` 等，可扩展。

---

### geo-store — 数据脊骨（1,317 行）

所有读写操作的核心。区分双通道写入：

| 通道 | 用途 | 特征 |
|------|------|------|
| **事务通道** | 矢量/栅格索引/元数据 | 低频、大 payload、需 CRS 校验、COPY 批写 |
| **流式直通** | GPS/IoT 高频时序 | 高频、小包、无 CRS 操作、直写 TimescaleDB 超表 |

| 模块 | 功能 |
|------|------|
| `postgis.rs` | 连接池、迁移、空间查询 |
| `batch_writer.rs` | PostgreSQL COPY 协议批写（比 INSERT 快 10-50x） |
| `timescale.rs` | 超表创建、chunk 管理（1 小时间隔） |
| `minio.rs` | S3/MinIO/GCS 对象存储（基于 `object_store` crate） |
| `dvc.rs` | DVC 版本控制（子进程调用 dvc CLI） |
| `migrations/` | 3 个完整 SQL 迁移文件（含索引、约束、注释） |

---

### geo-ingest — 数据接入（660 行）

多种数据源的解析和校验：

| 模块 | 功能 | 输入格式 |
|------|------|---------|
| `camofox.rs` | 网页采集 JSON 解析 | CamoFox/CDP JSON |
| `nmea.rs` | GPS NMEA 0183 解析 | `$GPGGA` / `$GPRMC` 语句 |
| `mqtt.rs` | MQTT 传感器流（feature `mqtt`） | JSON payload → TimescaleDB |
| `validator.rs` | 数据质量网关 | 坐标范围、HDOP、值域检查 |

NMEA 解析示例：
```rust
let msg = parse_nmea_line("$GPGGA,123519,2232.1234,N,11355.5678,E,1,12,0.8,100.0,M,,,*7F")?;
// → GgaFix { lat: 22.53539, lng: 113.92613, quality: 1, satellites: 12, altitude: 100.0 }
```

---

### geo-gee — GEE 任务分发（750 行）

**不直接调用 GEE API**。Rust 端只做三件事：

1. 将任务序列化为 JSON 写入消息队列
2. Python `gee-worker` 消费任务、执行 GEE SDK 操作
3. Rust 端读取回调、追踪任务状态

支持的任务类型：

| 任务 | 说明 | 输出 |
|------|------|------|
| `landcover_classification` | 随机森林土地覆盖分类（默认 50 棵树，10m 分辨率） | GeoTIFF → GCS |
| `ndvi_timeseries` | NDVI 时间序列合成（Sentinel-2） | COG → GCS |
| `change_detection` | 双年变化检测 | 变化图 → GCS |

```bash
# 分发土地覆盖分类任务
geo-toolbox process gee classify \
    --aoi s3://geo-data/vector/sites.gpkg \
    --year 2025 \
    --output-gcs gs://gee-exports/lc_2025.tif

# 查看状态
geo-toolbox process gee status --cid <uuid>
```

---

### geo-gdal — GDAL 栅格/矢量操作（750 行）

封装 GDAL CLI 工具。默认通过子进程调用 `gdal_translate`/`gdalwarp`/`ogr2ogr`，
可选开启 `gdal-bindings` feature 使用 Rust 原生绑定（更快，但需编译 libgdal）。

| 模块 | 功能 | 对应 CLI |
|------|------|---------|
| `raster.rs` | COG 转换、重投影、波段提取、重采样、合并、裁剪 | gdal_translate / gdalwarp |
| `vector.rs` | 格式互转（GPKG↔GeoJSON↔CSV↔Shapefile↔KML↔DXF） | ogr2ogr |
| `gcs_bridge.rs` | GCS → MinIO 搬运 + COG 转换 | gsutil cp + gdal_translate |

支持矢量格式：GPKG、GeoJSON、CSV、ESRI Shapefile、FlatGeobuf、GeoJSONSeq、DXF、KML。

---

### geo-qgis — QGIS 处理委托（600 行）

两种工作模式：

| 模式 | 实现 | 适用场景 |
|------|------|---------|
| **REST 客户端** | HTTP → PyQGIS 长驻服务 | 高频交互，无冷启动 |
| **子进程** | `qgis_process run` CLI | 低频大批量批处理 |

支持的处理算法：`buffer`、`reproject`、`clip`、`intersect`、`union`、`zonal_stats`。

```bash
# REST 模式（需 PyQGIS 服务运行在 localhost:9100）
geo-toolbox process qgis submit \
    --algorithm native:buffer \
    --input sites.gpkg --output buffered.gpkg \
    --params '{"INPUT":"input_layer","DISTANCE":2000}' \
    --server http://localhost:9100

# 子进程模式（需安装 QGIS）
geo-toolbox process qgis batch \
    --algorithm native:buffer \
    --input sites.gpkg --output buffered.gpkg \
    --extra '[["DISTANCE","2000"]]'

# 健康检查
geo-toolbox process qgis health --server http://localhost:9100
```

---

### geo-carbon — 碳核算引擎（642 行）

三条核算管线，一条 SQL 完成全部计算：

| 管线 | 方法 | 输入 |
|------|------|------|
| **A: 排放因子法** | IPCC Tier 1 | 土地覆被矢量 + `factor_registry` 表 |
| **B: LCA** | brightway2 子进程 | 活动数据 + ecoinvent 数据库 |
| **C: 碳汇遥感** | NPP/NEP 模型 | MODIS NPP + 森林清查 |

```bash
# 导入排放因子
geo-toolbox carbon emission-factor register emission-factors.csv

# 计算
geo-toolbox carbon emission-factor calculate \
    --aoi <uuid> --year 2025 --source IPCC_2019
```

审计链：每个计算结果可追溯到具体的遥感影像版本（`lc_dvc_hash`）、排放因子行（`factor_set_id` UUID）、审核人（`auditor_id`）。

---

### geo-output — 成果输出（762 行）

| 模块 | 功能 | 依赖 |
|------|------|------|
| `dxf_export.rs` | PostGIS 矢量 → CAD DXF 图纸 | `dxf` crate |
| `excel.rs` | SQL 查询结果 → 格式化 Excel | `rust_xlsxwriter` |
| `geojson_export.rs` | 空间查询 → GeoJSON / 本地文件验证压缩 | PostGIS `ST_AsGeoJSON` |
| `report.rs` | 碳核算结果 → Markdown 报告 | `tera` 模板引擎 |

`output geojson --from-file` 支持本地文件：读取 → 去空白压缩 → 可选 `--to-epsg` 重投影。

---

---

### geo-carbon-core — 纯 Rust 碳核算（零 DB 依赖）

从 `geo-carbon` 解耦出的纯计算引擎，可嵌入 WASM/PyO3/napi-rs 等任意环境。
支持 IPCC Tier 1 排放因子法：面积计算（纬度缩放 ±3%）、因子 CSV 表头解析（列顺序无关）、
子类精细匹配（`forest:evergreen_broadleaf`）及 category-only 回退。

```rust
use geo_carbon_core::{CarbonEngine, EmissionFactor, GeoFeature};
let engine = CarbonEngine::new();
let factors = vec![EmissionFactor::new("forest", 5.0, "IPCC_2019")];
let features = vec![GeoFeature::new("forest", geojson_polygon)?];
let report = engine.calculate(&features, &factors, 2025)?;
```

---

### geo-wasm — WASM 入口 + NPM 包（~2,700 行）

geo-toolbox 编译为 WebAssembly，浏览器直接调用。**数据不出网。**

| 模块 | 功能 |
|------|------|
| `crs.rs` | CRS 引擎 (wasm-bindgen) |
| `ingest.rs` | NMEA 解析 + 校验 |
| `carbon.rs` | 碳核算 (调用 geo-carbon-core) |
| `spatial.rs` | Area/BBox/Centroid/Simplify |
| `output.rs` | Excel/GeoJSON/Markdown 导出 |
| `storage.rs` | IndexedDB 本地存储 (rexie) |

---

### geo-parquet — 云原生矢量格式（~1,200 行）

GeoParquet 1.1 读写 + 空间谓词下推。

| 模块 | 功能 |
|------|------|
| `metadata.rs` | GeoParquet 元数据 + PROJJSON CRS |
| `schema.rs` | Arrow Schema 映射 |
| `reader.rs` | 读取 + Bbox 谓词下推 |
| `writer.rs` | 写入 (WKB + 累积 bbox) |
| `predicate.rs` | SpatialFilter (Bbox/Radius) |

---

### geo-ogc — OGC 标准服务（~2,200 行）

纯 Rust 实现 WMS 1.3 / WFS 2.0 / WPS 2.0。

| 模块 | 功能 |
|------|------|
| `common.rs` | OgcError/ServiceType |
| `wms.rs` | GetCapabilities + GetMap + GetFeatureInfo |
| `wfs.rs` | GetCapabilities + DescribeFeatureType + GetFeature |
| `wps.rs` | Execute(carbon/crs) + GetStatus + GetResult |

内置 WPS 流程：`carbon:emission-factor`、`crs:transform`。

## 部署

### 前置条件

| 组件 | 必需？ | Linux (apt) | macOS (brew) | Windows |
|------|-------|-------------|-------------|---------|
| Rust 1.80+ | ✅ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` | 同左 | [rustup.rs](https://rustup.rs) |
| libproj | 可选 | `apt install libproj-dev` | `brew install proj` | UTM 变换需要，基础变换无需 |
| PostgreSQL | 可选 | `apt install postgresql-16-postgis-3` | `brew install postgis` | 用 Docker |
| GDAL | 可选 | `apt install libgdal-dev` | `brew install gdal` | OSGeo4W 安装包 |
| QGIS | 可选 | `apt install qgis` | `brew install qgis` | OSGeo4W 安装包 |
| NATS | 可选 | `go install github.com/nats-io/nats-server/v2@latest` | 同左 | [nats.io](https://nats.io/download/) |

### 编译

```bash
git clone https://github.com/Miku196/geo-toolbox.git
cd geo-toolbox

# 全功能编译（需要 GDAL/QGIS 的 C 库）
cargo build --release

# 轻量编译 — 仅 CLI + 碳核算 + PostGIS，无 GIS 重依赖
cargo build --release --no-default-features --features minimal

# Windows (MSVC)
cargo build --release

# 编译产物
# Linux/macOS: target/release/geo-toolbox
# Windows:     target/release/geo-toolbox.exe
```

### 数据库（可选）

```bash
# 启动 PostGIS + TimescaleDB（需 Docker）
docker compose -f docker-compose.test.yml up -d

# 创建所有表结构
geo-toolbox store migrate

# 验证
geo-toolbox store read "SELECT PostGIS_Version()"
geo-toolbox store read "SELECT count(*) FROM spatial_assets"
```

### GEE 任务分发（可选）

```bash
# 方式 1: NATS（推荐，生产环境）
export GEO_NATS_URL=nats://localhost:4222
nats-server -js &

# 方式 2: 文件队列（默认，开发环境）
# 任务自动写入 ./queue/gee-tasks.jsonl
# Python gee-worker 只需 tail -f 该文件

# 查看队列
ls -la queue/
tail -f queue/gee-tasks.jsonl
```

---

## 使用手册

### CRS 坐标管理

```bash
# 列出所有内置坐标系 (5 个)
geo-toolbox crs list
# EPSG:4326  Storage   WGS 84
# EPSG:3857  Display   WGS 84 / Pseudo-Mercator
# EPSG:9000  Display   GCJ-02 (Mars Coordinate)
# EPSG:9001  Display   BD-09 (Baidu Coordinate)
# EPSG:32649 Carbon    WGS 84 / UTM zone 49N
# EPSG:32650 Carbon    WGS 84 / UTM zone 50N
# EPSG:3405  Carbon    World Equal Area

# 坐标变换 — 纯 Rust，零 C 依赖
geo-toolbox crs transform --from 4326 --to 3857 104.06 30.57
# → (11583906.21, 3577030.47)

# WGS84 → GCJ-02 火星坐标 (高德/腾讯)
geo-toolbox crs transform --from 4326 --to 9000 116.40 39.90
# → (116.4013, 39.9007)

# GCJ-02 → BD-09 百度坐标
geo-toolbox crs transform --from 9000 --to 9001 116.4013 39.9007

# 批量变换 (stdin, 一行一个 "x,y")
echo "104.06,30.57
121.47,31.23" | geo-toolbox crs transform --from 4326 --to 3857 --batch

# UTM 变换需要 proj feature (安装 libproj 后编译)
cargo build --features proj
geo-toolbox crs transform --from 4326 --to 32649 104.06 30.57
```

### 数据入库

```bash
# CamoFox 网页采集 JSON → PostGIS
geo-toolbox ingest camofox data.json

# NMEA GPS 日志解析
geo-toolbox ingest nmea gps_log.txt

# 通用 GeoJSON 写入指定表
geo-toolbox store write spatial_assets chengdu-zones.geojson

# 空间查询（返回 JSON）
geo-toolbox store read "SELECT ST_AsGeoJSON(geom), properties FROM spatial_assets WHERE aoi_id = 'cd-gaoxin'"

# DVC 版本快照
geo-toolbox store dvc-snapshot data/chengdu-zones.geojson
geo-toolbox store dvc-hash data/chengdu-zones.geojson
```

### GEE 遥感任务分发

```bash
# 土地覆盖分类
geo-toolbox process gee classify \
    --aoi s3://geo-data/vector/sites.gpkg \
    --year 2025 \
    --output-gcs gs://gee-exports/lc_2025.tif

# NDVI 时间序列
geo-toolbox process gee ndvi \
    --aoi s3://geo-data/vector/aoi.gpkg \
    --year 2024 \
    --output-gcs gs://gee-exports/ndvi_2024.tif

# 变化检测（双年对比）
geo-toolbox process gee change \
    --aoi s3://geo-data/vector/aoi.gpkg \
    --from 2020 --to 2025 \
    --output-gcs gs://gee-exports/change.tif

# 任务追踪
geo-toolbox process gee status --cid <uuid>
geo-toolbox process gee summary
```

### GDAL 栅格/矢量操作

```bash
# 转 Cloud-Optimized GeoTIFF
geo-toolbox process gdal cog input.tif output.cog.tif

# 指定压缩
geo-toolbox process gdal cog input.tif output.cog.tif --compression LZW

# 栅格重投影
geo-toolbox process gdal reproject input.tif output.tif --epsg 3857

# 矢量格式互转
geo-toolbox process gdal ogr2ogr input.geojson output.gpkg --overwrite

# 带过滤条件
geo-toolbox process gdal ogr2ogr input.geojson output.gpkg \
    --epsg 32649 --where "year=2020" --overwrite

# GCS → MinIO 搬运（GEE 结果回流）
geo-toolbox process gdal gcs-bridge \
    gs://gee-exports/lc_2025.tif \
    --prefix chengdu/landcover --cog

# 仅下载到本地
geo-toolbox process gdal gcs-bridge \
    gs://gee-exports/lc_2025.tif \
    --prefix chengdu/landcover --local
```

### QGIS 空间处理

```bash
# 缓冲区分析
geo-toolbox process qgis batch \
    --algorithm native:buffer \
    --input sites.gpkg --output buffered.gpkg \
    --extra '[["DISTANCE","2000"],["DISSOLVE","1"]]'

# 重投影
geo-toolbox process qgis batch \
    --algorithm native:reprojectlayer \
    --input sites.geojson --output sites_utm.gpkg \
    --extra '[["TARGET_CRS","EPSG:32649"]]'

# 相交分析（开发区 × 土地覆被）
geo-toolbox process qgis batch \
    --algorithm native:intersection \
    --input zones.gpkg --output zones_lc.gpkg \
    --extra '[["OVERLAY","landcover.gpkg"]]'

# 裁剪
geo-toolbox process qgis batch \
    --algorithm native:clip \
    --input sites.gpkg --output sites_clipped.gpkg \
    --extra '[["OVERLAY","boundary.gpkg"]]'

# 合并
geo-toolbox process qgis batch \
    --algorithm native:union \
    --input layer1.gpkg --output merged.gpkg \
    --extra '[["OVERLAY","layer2.gpkg"]]'

# 分区统计（需要栅格数据）
geo-toolbox process qgis batch \
    --algorithm native:zonalstatisticsfb \
    --input zones.gpkg --output zones_stats.gpkg \
    --extra '[["INPUT_RASTER","carbon_density.tif"],["COLUMN_PREFIX","carbon_"],["STATISTICS","mean,sum"]]'

# PyQGIS 服务检查
geo-toolbox process qgis health --server http://localhost:9100
```

### 碳核算

```bash
# 从 CSV 导入排放因子
geo-toolbox carbon emission-factor register emission-factors.csv

# 计算碳排放（一条 SQL 完成空间统计 + 因子关联）
geo-toolbox carbon emission-factor calculate \
    --aoi cd-gaoxin \
    --year 2025 \
    --source IPCC_2019

# LCA 生命周期评估（需 Python brightway2）
geo-toolbox carbon lca inventory.yaml
```

### 成果导出

```bash
# Excel 数据面板
geo-toolbox output excel \
    "SELECT landcover_class, SUM(area_ha) AS area, SUM(emission_tco2e) AS emission FROM carbon_accounting_results WHERE aoi_id = 'cd-gaoxin' GROUP BY landcover_class" \
    --output carbon_report.xlsx --sheet "碳核算"

# DXF CAD 图纸（自动 CRS 转换）
geo-toolbox output dxf \
    "SELECT ST_AsGeoJSON(geom) AS geom_json, 'buildings' AS layer FROM spatial_assets WHERE aoi_id = 'cd-gaoxin'" \
    --output cad_gaoxin.dxf --to-epsg 32649

# GeoJSON 导出 (PostGIS)
geo-toolbox output geojson \
    "SELECT ST_AsGeoJSON(geom) FROM spatial_assets WHERE aoi_id = 'cd-gaoxin'" \
    --output gaoxin.geojson

# GeoJSON 本地文件验证+压缩
geo-toolbox output geojson --from-file input.geojson --output compact.geojson

# 本地文件 + 重投影到 Web Mercator
geo-toolbox output geojson --from-file input.geojson --output merc.geojson --to-epsg 3857

# 碳核算报告
geo-toolbox output report \
    --aoi cd-gaoxin --year 2025 \
    --name "成都高新区" --source IPCC_2019 \
    --output chengdu_gaoxin_carbon.md
```

### MCP Server

```bash
# 启动（默认端口 9378，使用 stdio 传输）
geo-toolbox mcp-serve --port 9378

# AI Agent 通过 JSON-RPC 调用
# → tools/list → tools/call
```

暴露的 MCP tools：
- `crs_list` — 列出所有坐标系
- `crs_transform` — 坐标变换
- `ingest_camofox` — 解析 CamoFox JSON 并入库
- `carbon_calculate` — 执行碳核算

---

## Feature Flags

```toml
# 完整编译（默认）
cargo build --release

# 轻量：仅 CLI + PostGIS + 碳核算，无 GEE/QGIS/GDAL
cargo build --release --no-default-features --features minimal

# 按需组合
cargo build --release --no-default-features --features gee,qgis
cargo build --release --no-default-features --features gdal,gee,minimal
```

| Flag | 所在 crate | 新增依赖 | 功能 |
|------|-----------|---------|------|
| `gee` | geo-cli | `async-nats` | GEE 任务分发 |
| `qgis` | geo-cli | — | QGIS 处理委托 |
| `gdal` | geo-cli | `gdal` (libgdal) | GDAL 栅格/矢量 |
| `mqtt` | geo-ingest | `rumqttc` (libssl) | MQTT 流摄入 |
| `nats` | geo-gee | `async-nats` | NATS 消息队列 |
| `kafka` | geo-gee | `rdkafka` (librdkafka) | Kafka 消息队列 |
| `minimal` | geo-cli | 仅 libproj | 无任何 GIS 重依赖 |

**编译依赖最小化示例**：

```bash
# 只需要碳核算功能的生产部署
cargo build --release --no-default-features --features minimal

# Windows 上避免编译 GDAL（链接 libgdal 经常出问题）
cargo build --release --no-default-features --features gee,qgis
```

---

## 消息队列

geo-gee 支持三种消息队列后端，自动选择：

| 后端 | 何时使用 | 配置方式 |
|------|---------|---------|
| **NATS** | 生产环境 | `export GEO_NATS_URL=nats://localhost:4222` |
| **文件队列** | 开发/测试（默认） | 自动写入 `./queue/gee-tasks.jsonl` |
| **Kafka** | 已有 Kafka 集群 | 编译 `--features kafka` |

Python gee-worker 端：
```bash
# NATS 模式
nats sub gee.tasks

# 文件模式
tail -f queue/gee-tasks.jsonl

# 回调查看
tail -f queue/gee-callbacks.jsonl
nats sub gee.callbacks
```

---

## 开发

### 运行测试

```bash
# 全部单元测试（不需要任何外部服务，直接跑）
cargo test

# 单个 crate
cargo test -p geo-gee
cargo test -p geo-ingest
cargo test -p geo-carbon

# 集成测试（需要 Docker 启动 PostGIS）
docker compose -f docker-compose.test.yml up -d
cargo test -- --include-ignored

# 性能基准
cargo bench -p geo-store
cargo bench -p geo-core
```

### 代码质量

```bash
# Clippy（0 warnings）
cargo clippy

# 格式化检查
cargo fmt --all -- --check

# 文档
cargo doc --no-deps --open
```

### 项目结构

```
geo-toolbox/
├── Cargo.toml                 # workspace root + 共享依赖
├── README.md
├── docker-compose.test.yml    # 集成测试环境
├── .gitignore
├── build.bat                  # Windows 编译脚本（设置 PROJ 路径）
├── benches/                   # 性能基准
│   ├── crs_transform.rs
│   └── batch_write.rs
├── examples/
│   ├── chengdu-carbon/        # 成都开发区碳收支评估完整示例
│   │   ├── chengdu-zones.geojson
│   │   ├── landcover-transition.csv
│   │   ├── emission-factors.csv
│   │   ├── calc_carbon.py
│   │   ├── gen_html_pdf.py
│   │   └── chengdu-carbon-zones.qgs
│   └── china-risk-assessment/ # 中国洪水+地震风险评估
│       └── README.md           # geo-toolbox 实战管线文档
└── crates/
    ├── geo-core/              # 共享类型 + CRS + 错误
    │   └── src/{lib,errors,crs,types}.rs
    ├── geo-cli/               # CLI + MCP Server
    │   └── src/{main,mcp}.rs + commands/{crs,ingest,store,process,carbon,output}.rs
    ├── geo-store/             # PostGIS + TimescaleDB + MinIO + DVC
    │   └── src/{lib,postgis,batch_writer,timescale,minio,dvc}.rs + migrations/*.sql
    ├── geo-ingest/            # 数据接入
    │   └── src/{lib,camofox,nmea,mqtt,validator}.rs
    ├── geo-gee/               # GEE MQ 分发
    │   └── src/{lib,dispatcher,mq,tracker}.rs
    ├── geo-gdal/              # GDAL 操作
    │   └── src/{lib,raster,vector,gcs_bridge}.rs
    ├── geo-qgis/              # QGIS 委托
    │   └── src/{lib,grpc_client,process_runner}.rs
    ├── geo-carbon/            # 碳核算 (PostgreSQL)
    │   └── src/{lib,emission_factor,lca,carbon_sink,audit}.rs
    ├── geo-carbon-core/       # 纯 Rust 碳核算 (零 DB)
    │   └── src/{lib,engine,factor,feature,report}.rs
    ├── geo-output/            # 成果输出
    │   └── src/{lib,dxf_export,excel,geojson_export,report}.rs
    ├── geo-wasm/              # WASM 入口 + NPM 包
    │   └── src/{lib,crs,ingest,carbon,spatial,output,storage,utils}.rs
    ├── geo-parquet/           # GeoParquet 云原生格式
    │   └── src/{lib,metadata,schema,reader,writer,predicate}.rs
    ├── geo-ogc/               # OGC WMS/WFS/WPS 服务
    │   └── src/{lib,common,wms,wfs,wps}.rs
    │
    ├── package.json           # NPM 包配置
    ├── tsconfig.json          # TypeScript 配置
    └── src/
        └── index.ts           # TypeScript SDK
```

---

## 示例

### 成都碳核算

`examples/chengdu-carbon/` 是一个完整的碳收支评估案例，从零数据到 PDF 报告：

| 步骤 | 数据/工具 | 说明 |
|------|----------|------|
| 1. 构建 AOI | `chengdu-zones.geojson` | 成都 4 个开发区边界 |
| 2. 土地覆被 | `landcover-transition.csv` | 24 条覆被转移记录（基于公开统计） |
| 3. 排放因子 | `emission-factors.csv` | 17 条 IPCC 2019 因子（含子类） |
| 4. 计算 | `calc_carbon.py` | 纯 Python IPCC Tier 1 计算 |
| 5. 可视化 | `chengdu-carbon-zones.qgs` | QGIS 项目文件 |
| 6. 报告 | `gen_html_pdf.py` | HTML + Chrome headless → PDF |

WASM Demo 默认数据即取自本示例（高新区 + 天府新区）。

运行方法见 `examples/chengdu-carbon/` 目录下的脚本注释。

### 中国自然灾害风险评估

完整实战案例见 `examples/china-risk-assessment/`，展示如何用 geo-toolbox + Camoufox + Python GIS
构建洪水高风险区和地震活动两条评估管线。

---

## 技术选型

| 决策 | 选择 | 理由 |
|------|------|------|
| 数据库驱动 | `sqlx`（编译时 SQL 检查） | 不要运行时 ORM，空间查询写裸 SQL 最可靠 |
| 几何库 | `geo` + `geo-types` | Rust 生态标准，与 PostGIS WKB 互转成熟 |
| 异步运行时 | `tokio` | MCP Server + MQ + 批写都需要 async |
| 对象存储 | `object_store` | 统一 S3/GCS/MinIO 接口 |
| 消息队列 | NATS（主）+ 文件队列（回退） | 轻量、高性能、无 ZooKeeper 依赖 |
| 错误处理 | `thiserror` + 单一 `GeoError` enum | 所有 crate 共享错误类型 |
| 测试策略 | 单元测试离线跑 + Docker 集成测试 | 不要 mock 数据库 |

## 参与贡献

欢迎提交 Issue 和 Pull Request。

### 报告问题

遇到 bug、功能建议、文档改进 → [提交 Issue](https://github.com/Miku196/geo-toolbox/issues)

请在 Issue 中附上：
- 操作系统和 Rust 版本（`rustc --version`）
- 复现步骤
- 预期行为 vs 实际行为
- 相关日志或错误信息

### 提交代码

```bash
# Fork 并 clone
git clone https://github.com/Miku196/geo-toolbox.git
cd geo-toolbox

# 创建功能分支
git checkout -b feature/my-feature

# 确保通过所有检查
cargo fmt --all -- --check
cargo clippy
cargo test

# 提交（推荐 Conventional Commits）
git commit -m "feat: add something useful"

# 推送并发起 PR
git push origin feature/my-feature
```

PR 合并前需要：
- [ ] 新增/修改的公开 API 有文档注释（`#![warn(missing_docs)]`）
- [ ] 集成测试（需 Docker）标记 `#[ignore]`
