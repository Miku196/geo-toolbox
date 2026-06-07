# geo-toolbox

**Rust 地理空间工具链** — AI Agent 与地理空间重工具之间的高性能胶水层。

将 PostGIS、GEE、QGIS、GDAL 等重型 GIS 工具串联成一条自动化管线：
数据采集 → 入库存储 → 遥感分析 → 碳核算 → 成果输出。

不替代任何现有工具。Rust 负责性能敏感路径（批写、格式转换、消息分发、碳核算），
遥感计算和空间分析仍委托 Python 生态（GEE SDK、PyQGIS、GDAL CLI、brightway2）。

[![Rust](https://img.shields.io/badge/rust-1.80+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-61%20pass-green.svg)]()

---

## 目录

- [是什么](#是什么)
- [快速开始](#快速开始)
- [架构](#架构)
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

一个 Rust workspace，9 个 crate，约 8,600 行代码，覆盖地理空间数据管线的全生命周期：

```
数据采集          入库存储          处理分析          成果输出
   │                │                │                │
 CamoFox 网页抓取  PostGIS 矢量     GEE 遥感分类     DXF CAD 图纸
 NMEA GPS 解析     TimescaleDB 时序  GDAL 栅格转换    Excel 数据面板
 MQTT 传感器流     MinIO/S3 栅格    QGIS 空间分析     GeoJSON 导出
                   DVC 版本控制      LCA 碳足迹       Markdown 报告
```

### 核心设计原则

- **Rust 做胶水，Python 做重活** — GEE 的 Random Forest 分类、QGIS 的 buffer/overlay 仍用 Python，Rust 只负责任务分发、结果搬运、格式转换
- **每个 crate 独立可测** — 不依赖数据库也能跑大多数测试；集成测试用 Docker Compose
- **Feature flags 控制依赖** — 不需要 GDAL/QGIS 就别编译进去，最小化二进制体积
- **MCP 原生集成** — 内置 MCP Server，AI Agent 通过 JSON-RPC 直接调用所有功能
- **性能量化** — 批写用 PostgreSQL COPY 协议（比逐条 INSERT 快 10-50x），CRS 变换有 benchmark

---

## 快速开始

```bash
git clone https://github.com/Miku196/geo-toolbox.git
cd geo-toolbox

# 编译（需要 Rust 1.80+ 和 libproj）
cargo build --release

# 列出坐标系
cargo run -- crs list

# 坐标变换
cargo run -- crs transform --from 4326 --to 3857 104.06 30.57

# 运行测试
cargo test
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

## Crate 详解

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
| `crs.rs` | CRS 注册表（5 个内置坐标系）、坐标变换 |
| `types.rs` | 几何类型别名、坐标校验 |

内置坐标系：`EPSG:4326` (WGS84)、`EPSG:3857` (Web Mercator)、`EPSG:32649` (UTM 49N)、`EPSG:3405` (等积投影，碳核算必需)、`EPSG:4547` (CGCS2000)。

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
| `geojson_export.rs` | 空间查询 → GeoJSON 文件 | PostGIS `ST_AsGeoJSON` |
| `report.rs` | 碳核算结果 → Markdown 报告 | `tera` 模板引擎 |

---

## 部署

### 前置条件

| 组件 | 必需？ | Linux (apt) | macOS (brew) | Windows |
|------|-------|-------------|-------------|---------|
| Rust 1.80+ | ✅ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` | 同左 | [rustup.rs](https://rustup.rs) |
| libproj | ✅ | `apt install libproj-dev` | `brew install proj` | `vcpkg install proj` |
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
# 列出所有内置坐标系
geo-toolbox crs list
# EPSG:4326  Storage   WGS84
# EPSG:3857  Storage   Web Mercator
# EPSG:32649 Storage   WGS84 / UTM zone 49N
# EPSG:3405  Carbon    World Equal Area
# EPSG:4547  Storage   CGCS2000

# 查看某个坐标系详情
geo-toolbox crs show 4326

# 坐标变换（成都: WGS84 → Web Mercator）
geo-toolbox crs transform --from 4326 --to 3857 104.06 30.57
# → x=11584385.2 y=3575028.3

# 深圳: WGS84 → UTM 49N
geo-toolbox crs transform --from 4326 --to 32649 113.9 22.5

# 注册自定义坐标系
geo-toolbox crs register 4528 "CGCS2000 / 3-degree Gauss-Kruger zone 40" "+proj=tmerc +lat_0=0 +lon_0=120 +k=1 +x_0=40500000 +y_0=0 +ellps=GRS80 +units=m +no_defs"
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

# GeoJSON 导出
geo-toolbox output geojson \
    "SELECT ST_AsGeoJSON(geom) FROM spatial_assets WHERE aoi_id = 'cd-gaoxin'" \
    --output gaoxin.geojson

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
│   └── chengdu-carbon/        # 成都开发区碳收支评估完整示例
│       ├── chengdu-zones.geojson
│       ├── landcover-transition.csv
│       ├── emission-factors.csv
│       ├── calc_carbon.py
│       ├── gen_html_pdf.py
│       └── chengdu-carbon-zones.qgs
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
    ├── geo-carbon/            # 碳核算
    │   └── src/{lib,emission_factor,lca,carbon_sink,audit}.rs
    └── geo-output/            # 成果输出
        └── src/{lib,dxf_export,excel,geojson_export,report}.rs
```

---

## 示例

`examples/chengdu-carbon/` 是一个完整的碳收支评估案例，从零数据到 PDF 报告：

| 步骤 | 数据/工具 | 说明 |
|------|----------|------|
| 1. 构建 AOI | `chengdu-zones.geojson` | 成都 4 个开发区边界 |
| 2. 土地覆被 | `landcover-transition.csv` | 24 条覆被转移记录（基于公开统计） |
| 3. 排放因子 | `emission-factors.csv` | 17 条 IPCC 2019 因子 |
| 4. 计算 | `calc_carbon.py` | 纯 Python IPCC Tier 1 计算 |
| 5. 可视化 | `chengdu-carbon-zones.qgs` | QGIS 项目文件 |
| 6. 报告 | `gen_html_pdf.py` | HTML + Chrome headless → PDF |

运行方法见 `examples/chengdu-carbon/` 目录下的脚本注释。

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
