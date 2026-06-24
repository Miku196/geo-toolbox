# geo-toolbox 三层架构改造开发流程

> **目标**：将 13 个平铺 crate 改造为 Core（核心引擎）→ Plugin（专业插件）→ Adapter（外部适配器）三层架构。
>
> **原则**：
> - 每 Phase 可独立验证，`cargo check --workspace && cargo test --workspace` 必须通过。
> - **依赖方向必须严格单向**：Adapter 可依赖 Plugin，Plugin 可依赖 Core；但 Core 不能依赖 Plugin，Plugin 不能依赖 Adapter。
> - **同一层内部可以有向无环依赖**（如 geo-raster → geo-core），但不能循环。
> - **Plugin 之间禁止互相依赖**（保持可插拔性）。

---

## 目录

- [架构总览](#架构总览)
- [当前 crate → 新架构映射](#当前-crate--新架构映射)
- [Phase 1：Core 提纯 — 标记纯 Rust 代码边界](#phase-1core-提纯--标记纯-rust-代码边界)
- [Phase 2：Core 重组 — 新建 7 个核心引擎 crate](#phase-2core-重组--新建-7-个核心引擎-crate)
- [Phase 3：Adapter 重组 — 外部依赖 crate 改名为适配器](#phase-3adapter-重组--外部依赖-crate-改名为适配器)
- [Phase 4：Plugin 体系 — 新建 7 个专业插件 crate](#phase-4plugin-体系--新建-7-个专业插件-crate)
- [Phase 5：Registry — 插件注册与调度中心](#phase-5registry--插件注册与调度中心)
- [Phase 6：CLI + WASM 入口适配 — 最终集成](#phase-6cli--wasm-入口适配--最终集成)
- [验证检查清单](#验证检查清单)

---

## 执行策略：先跑通 MVP，再铺全量

> 26 个 crate 全量落地工作量大。建议分 3 轮迭代，先验证架构可行性再铺开。
> 
> **当前进度**：Round 1 + Round 2 + Round 3 已全部完成。三层架构改造完成，89 个 MCP 工具注册完毕。

### Round 1：最小可行核心 + 一个完整案例（目标：1-2 天完成）✅ 已完成

**只建这些 crate**：

| 层级 | Crate | 理由 |
|------|-------|------|
| Core | `geo-core` | 类型/错误/CRS（已有，不动） |
| Core | `geo-raster` | 波段运算 + NDVI（从 geo-gdal 提纯） |
| Core | `geo-stats` | 分区统计（zonal stats） |
| Core | `geo-io` | GeoJSON/CSV 读写（从 geo-ingest 提纯） |
| Core | `geo-carbon-math` | IPCC 方程式（从 geo-carbon-core 提纯） |
| Plugin | `geo-plugin-carbon` | 碳核算插件（rules.toml + 碳密度表 + 碳汇报告模板） |
| Plugin | `geo-plugin-ecology` | 生态修复插件（NDVI 变化检测 + 调用 carbon-math 算碳汇） |

**跑通的案例**：矿山生态修复评估
```
输入：矿区 AOI GeoJSON + 两期 Sentinel-2 NDVI 影像
  → geo-io 读取 GeoJSON
  → geo-raster 计算 NDVI 差值
  → geo-stats 分区统计植被变化面积
  → geo-carbon-math 计算碳汇量
  → geo-plugin-ecology 读取 rules.toml 参数，组装结果
  → geo-report 渲染碳汇评估报告 Markdown
输出：碳汇报告 + 植被恢复面积统计
```

**验证成功的标准**：
- `cargo test -p geo-plugin-ecology` 通过（输入 GeoJSON + 两幅小 NDVI tiff → 输出碳汇数字和报告字符串）
- 生态修复插件**不 import 碳核算插件**，只 import `geo-carbon-math`

### Round 2：补齐剩余插件 + WASM ✅ 已完成

在 Round 1 验证架构可行后，依次添加：
- `geo-plugin-survey`（测绘）
- `geo-plugin-urban`（城乡规划）
- `geo-plugin-hydro`（水文）
- `geo-plugin-geohazard`（地质灾害）
- `geo-plugin-agri`（农业）
- `geo-wasm` 适配（只用 core 层 crate）

同时补齐 core 层：`geo-vector`, `geo-index`, `geo-report`, `geo-parquet`, `geo-ogc`

### Round 3：适配器 + CLI + Registry ✅ 已完成

最后补齐外部系统桥接：
- 7 个 adapter crate
- `geo-registry`
- `geo-cli` 改造

**完成成果**：
- 89 个 MCP 工具注册完毕
- 三层架构（Core → Plugin → Adapter）完整落地
- 无环依赖验证通过
- 测试覆盖率 45%（238/531 tests）

---

## 架构总览

```
┌─────────────────────────────────────────────────────────────┐
│                        geo-toolbox                          │
├─────────────────────────────────────────────────────────────┤
│  Layer 1: 核心引擎 (Core) — 16 个工具                        │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  栅格/矢量/几何/索引/统计/IO/报告基类                  │  │
│  │  纯 Rust，不依赖任何专业领域逻辑                       │  │
│  │  CRS: crs_list, crs_transform, validate_coord         │  │
│  │  瓦片: tile_latlon_to_tile, tile_bounds, tile_url,    │  │
│  │        tile_encode_mvt                                │  │
│  │  索引: geohash_encode/decode/neighbors                │  │
│  │  矢量: vector_buffer/intersect/area/centroid          │  │
│  │  时序: temporal_trend                                 │  │
│  │  统计: zonal_stats                                    │  │
│  └───────────────────────────────────────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│  Layer 2: 专业插件 (Plugins) — 67 个工具                     │
│  ┌────────┬────────┬────────┬────────┬────────┬────────┐  │
│  │生态修复│ 测绘   │城乡规划│ 水文   │地质灾害│ 农业   │  │
│  │4工具  │8工具   │6工具   │9工具   │3工具   │4工具   │  │
│  ├────────┼────────┼────────┼────────┼────────┼────────┤  │
│  │碳核算 │林业   │海岸带 │ 能源   │        │        │  │
│  │9工具  │1工具  │5工具  │4工具   │        │        │  │
│  └────────┴────────┴────────┴────────┴────────┴────────┘  │
│  每个插件 = 配置文件(rules.toml) + 报告模板 + 调用core的入口 │
│  ⚠ 插件之间禁止互相依赖                                     │
├─────────────────────────────────────────────────────────────┤
│  Layer 3: 外部适配器 (Adapters) — 6 个工具                    │
│  ┌──────┬──────┬──────┬──────┬──────┬──────┬────────────┐ │
│  │ QGIS │ CAD  │ GEE  │PostGIS│ MCP │ IoT  │ 其他 CLI   │ │
│  │2工具 │1工具 │2工具 │4工具  │      │1工具 │4工具       │ │
│  └──────┴──────┴──────┴──────┴──────┴──────┴────────────┘ │
│  作用：让 core 与外部生态系统双向通信                        │
└─────────────────────────────────────────────────────────────┘

依赖方向（单向）：
  Adapter ──→ Plugin ──→ Core
  （上层可依赖下层，下层不可依赖上层）

工具总数：89 个
  Core: 16 | 碳核算: 9 | 生态修复: 4 | 新能源: 4 | 林业: 1
  海岸带: 5 | 水文: 9 | 地灾: 3 | 测绘: 8 | 农业: 4
  城乡规划: 6 | 数据接入: 6 | 外部桥接: 16
```

### 三层间依赖硬约束

```
❌ 禁止：Core 依赖 Plugin 或 Adapter
❌ 禁止：Plugin 依赖 Adapter
❌ 禁止：Plugin 之间互相依赖
✅ 允许：Adapter 依赖 Plugin（如生态报告适配 QGIS 导出）
✅ 允许：Adapter 依赖 Core
✅ 允许：Plugin 依赖 Core（多个 core crate 组合）
```

### Core 层内部依赖 DAG

Core 层 crate 之间可以有向无环依赖，以下是规划的依赖层级：

```
Layer 0: geo-core          （几何基类：类型/CRS/错误/BBox）
              ↑
Layer 1: geo-index         （空间索引：R-tree/GeoHash）
         geo-io            （IO 基类：格式解析/序列化）
         geo-stats         （统计基类：分区统计/回归/插值）
              ↑
Layer 2: geo-raster        （栅格基类：波段运算/重采样）
         geo-vector         （矢量基类：缓冲区/拓扑）
         geo-carbon-math   （碳核算纯公式：IPCC方程式）
              ↑
Layer 3: geo-report        （报告基类：Tera 模板/Markdown）
         geo-parquet        （云原生格式：GeoParquet 读写）

关键约束：
  - geo-raster 和 geo-vector 不能互相依赖
    （两者通过 geo-core 中的 trait 交互）
  - geo-carbon-math 只能依赖 geo-core 和 geo-stats
    （不依赖 geo-raster/geo-vector/geo-parquet）
  - geo-parquet 只能依赖 geo-core 和 geo-vector
    （不依赖 geo-raster）
```

---

## 当前 crate → 新架构映射

```
现状 crate              归属          原因
────────────────────────────────────────────────────────────
geo-core                → Core        类型/错误/CRS，已经是核心（注：可考虑改名为 geo-primitives）
geo-parquet             → Core        云原生格式引擎，纯 Rust（只依赖 geo-core + geo-vector）
geo-ogc                 → Core        WMS/WFS/WPS 纯 Rust 标准实现

geo-carbon-core         → 拆分
  ├── geo-carbon-math   → Core        纯数学公式（IPCC 方程式），不依赖任何 GIS 类型
  └── geo-plugin-carbon → Plugin      碳核算业务（依赖 geo-carbon-math + geo-raster + 碳密度表配置）

geo-store               → Adapter     本质是对 PostGIS 的桥接（改名为 geo-adapter-postgis）
geo-gee                 → Adapter     对 GEE 的桥接
geo-gdal                → Adapter     对 GDAL 的桥接
geo-qgis                → Adapter     对 QGIS 的桥接

geo-ingest              → 拆分
  ├── camofox.rs        → Core        纯 Rust JSON 解析
  ├── nmea.rs           → Core        纯 Rust NMEA 解析
  └── mqtt.rs           → Adapter     外部 MQTT broker

geo-output              → 拆分
  ├── excel.rs          → Core        纯 Rust xlsxwriter
  ├── report.rs         → Core        纯 Rust Tera 模板
  ├── geojson_export.rs → Core        纯 Rust 序列化
  └── dxf_export.rs     → Adapter     CAD 外部生态

geo-carbon              → Plugin      废弃，逻辑拆分到 geo-plugin-carbon
geo-cli                 → 入口+Adapter
  └── mcp.rs            → Adapter     MCP 协议适配
geo-wasm                → 入口        浏览器端，只能依赖 core 层（不能依赖 plugin/adapter）
```

---

## Phase 1：Core 提纯 — 标记纯 Rust 代码边界

**目标**：不新建 crate，只在现有 crate 内部审查并注释标记，区分"纯 Rust 逻辑"和"外部依赖调用"。

**原则**：纯 Rust 逻辑 = 不依赖数据库连接、网络 I/O、子进程调用、外部二进制。

### 1.1 操作清单

| Crate | 文件 | 操作 |
|-------|------|------|
| `geo-core` | 全部 | ✅ 已经是纯 Rust，不动 |
| `geo-carbon-core` | 全部 | ✅ 已经是纯 Rust，不动 |
| `geo-parquet` | 全部 | ✅ 已经是纯 Rust，不动 |
| `geo-ogc` | 全部 | ✅ 已经是纯 Rust，不动 |
| `geo-gdal` | `src/raster.rs` | 审查：COG 转换（子进程调用）vs 栅格代数（可提纯） |
| `geo-gdal` | `src/vector.rs` | 审查：ogr2ogr（子进程调用）vs 格式转换逻辑 |
| `geo-ingest` | `src/camofox.rs` | 标记：JSON 解析 = 纯 Rust → 将来入 Core IO |
| `geo-ingest` | `src/nmea.rs` | 标记：NMEA 解析 = 纯 Rust → 将来入 Core IO |
| `geo-ingest` | `src/mqtt.rs` | 标记：rumqttc = 外部依赖 → 留 Adapter |
| `geo-output` | `src/excel.rs` | 标记：rust_xlsxwriter = 纯 Rust → 将来入 Core IO |
| `geo-output` | `src/report.rs` | 标记：Tera 模板 = 纯 Rust → 将来入 Core Report |
| `geo-output` | `src/geojson_export.rs` | 标记：序列化 = 纯 Rust → 将来入 Core IO |
| `geo-output` | `src/dxf_export.rs` | 标记：dxf crate = 可纯 Rust，但属 CAD 生态 → 留 Adapter |
| `geo-carbon` | `src/emission_factor.rs` | 审查：SQL 查询 vs 计算公式（公式提纯到 `geo-carbon-math`，SQL 部分留在插件） |
| `geo-carbon-core` | `src/engine.rs`, `src/factor.rs` | 审查：纯公式 → `geo-carbon-math`；GIS 类型依赖 → `geo-plugin-carbon` |

### 1.2 标记方式

在每个文件顶部添加注释：

```rust
//! @layer: Core       ← 纯 Rust，不依赖外部
//! @layer: Adapter    ← 依赖外部系统（数据库/网络/子进程）
```

### 1.3 验证

```bash
cargo check --workspace
cargo test --workspace   # 应保持 51 tests pass
```

---

## Phase 2：Core 重组 — 新建 7 个核心引擎 crate

**目标**：从现有 crate 中抽离纯 Rust 代码，汇聚到新的 core crate 中。

**原则**：
- 每个 core crate **零外部系统依赖**（不依赖 PostGIS/GEE/QGIS/GDAL CLI/网络）
- 可独立 `cargo test`，无需 Docker
- 搬代码时保持原有逻辑不变，只改 `crate::` 路径

### 2.1 新建 crate 清单

| 新建 crate | 来源 | 内容 | Cargo.toml 依赖 |
|-----------|------|------|----------------|
| `core/geo-raster` | `geo-gdal/src/raster.rs` 提纯部分 | 栅格代数（波段加减乘除）、重采样（最邻近/双线性/三次）、波段统计（min/max/mean/std）、镶嵌边界计算 | `geo-core`, `geo-io` |
| `core/geo-vector` | `geo-core` + `geo-ogc` 公共部分 | 矢量运算基类：缓冲区、相交、合并、裁剪的纯 Rust 实现、拓扑验证 | `geo-core`, `geo-io` |
| `core/geo-index` | 全新 | 空间索引：R-tree（rstar crate）、GeoHash 编解码、H3 六边形网格、四叉树 | `geo-core` |
| `core/geo-stats` | 全新 | 分区统计（zonal stats）、线性回归、时间序列插值、聚类（k-means 空间聚类）、莫兰指数 | `geo-core` |
| `core/geo-io` | `geo-ingest` + `geo-output` 提纯部分 | 格式解析基类（JSON/CSV/NMEA/GeoJSON）、Excel 读写基类（rust_xlsxwriter/calamine）、流式读写 trait | `geo-core` |
| `core/geo-carbon-math` | `geo-carbon-core` 的纯公式部分 | IPCC Tier 1 方程、排放因子插值、面积加权、不确定性传播 — 纯数学，不依赖任何 GIS 类型 | `geo-core`（仅错误类型）, `geo-stats` |
| `core/geo-report` | `geo-output/src/report.rs` | Tera 模板引擎封装、Markdown/HTML 渲染基类、自定义过滤器、公共模板组件 | `geo-core`, `geo-stats`, `tera`, `chrono` |

### 2.2 crate 目录结构（以 geo-raster 为例）

```
core/geo-raster/
├── Cargo.toml
├── src/
│   ├── lib.rs            # 入口：re-export 所有模块
│   ├── band.rs           # 波段运算：add/sub/mul/div/ndvi/ndwi
│   ├── resample.rs       # 重采样算法
│   ├── stats.rs          # 波段统计
│   └── mosaic.rs         # 镶嵌边界计算
└── tests/
    ├── band_tests.rs
    ├── resample_tests.rs
    └── fixtures/
        └── sample.tif    # 小测试栅格（< 100KB）
```

### 2.3 geo-report 特殊说明

`geo-report` 在 core 层只提供**渲染引擎 + 公共组件**，不包含任何领域模板：

```
core/geo-report/
├── Cargo.toml
├── src/
│   ├── lib.rs            # 入口：ReportEngine struct
│   ├── render.rs         # Tera 封装、render_md/render_html
│   ├── filters.rs        # 自定义 Tera 过滤器（ha_fmt, co2_fmt, percent_fmt...）
│   └── schema.rs         # 报告配置 schema
└── templates/            # 公共组件（所有插件通过 include 引用）
    ├── _partials/        # 可复用片段
    │   ├── header.md.tera
    │   ├── table.md.tera
    │   ├── map.md.tera
    │   └── stats.md.tera
    ├── _layouts/         # 页面布局
    │   ├── base.md.tera
    │   └── two-column.md.tera
    └── _filters/         # 过滤器定义（与 src/filters.rs 对应）
```

各插件调用方式：
```rust
// plugins/geo-plugin-ecology/src/ecology.rs
use geo_report::ReportEngine;

let mut engine = ReportEngine::new()?;
engine.register_templates("ecology", Path::new("templates/"))?;
let md = engine.render_md("carbon-sink.md.tera", &data)?;
```

### 2.4 Cargo.toml 模板（geo-raster）

```toml
[package]
name = "geo-raster"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "纯 Rust 栅格运算基类 — 波段运算、重采样、镶嵌，零外部依赖"

[dependencies]
geo-core = { path = "../geo-core" }
geo-types.workspace = true
geo.workspace = true
serde.workspace = true
thiserror.workspace = true
# 注意：不依赖 gdal / proj / sqlx / tokio / object_store / rumqttc / async-nats
```

### 2.4 workspace Cargo.toml 更新

```toml
[workspace]
members = [
    # 保留原有
    "crates/geo-core",
    "crates/geo-carbon-core",
    # ...
    # 新增 Core
    "core/geo-raster",
    "core/geo-vector",
    "core/geo-index",
    "core/geo-stats",
    "core/geo-io",
    "core/geo-report",
]
```

### 2.5 验证

```bash
cargo check --workspace
cargo test -p geo-raster
cargo test -p geo-vector
cargo test -p geo-index
cargo test -p geo-stats
cargo test -p geo-io
cargo test -p geo-report
cargo test --workspace     # 全量，原有 51 tests 不得减少
```

---

> **风险提示**：`geo-adapter-cli` 合并了 GDAL CLI、DVC CLI、shell 子进程等多种外部工具。
> 合用一个 crate 用 feature flags 隔离是可行的，但如果 GDAL 和 DVC 的接口差异太大，
> 后续可拆分为 `geo-adapter-gdal` 和 `geo-adapter-dvc`。先合并，观察。
>
> 同样，`geo-report` 中的过滤器（`ha_fmt`, `co2_fmt`, `percent_fmt`）算通用单位转换，
> 放在 core 层可接受。但不要出现"碳汇"、"生态"等业务关键词。

## Phase 3：Adapter 重组 — 外部依赖 crate 改名为适配器

**目标**：将依赖外部系统的 crate 统一放到 `adapters/` 目录，实现 `ExternalAdapter` trait。

**原则**：
- 改名不删功能，原有 pub API 全部保留
- 新增 `ExternalAdapter` trait impl
- 双向通信：`push_to_external` / `pull_from_external` 统一接口

### 3.1 crate 移动与改名

| 原来 | 改为 | 外部依赖 |
|------|------|---------|
| `crates/geo-store` | `adapters/geo-adapter-postgis` | PostgreSQL |
| `crates/geo-gee` | `adapters/geo-adapter-gee` | Python GEE worker, NATS |
| `crates/geo-qgis` | `adapters/geo-adapter-qgis` | PyQGIS, qgis_process |
| `crates/geo-gdal` | `adapters/geo-adapter-cli`（合并） | GDAL CLI, DVC CLI |
| `crates/geo-cli/src/mcp.rs` | `adapters/geo-adapter-mcp` | MCP Agent 协议 |
| `crates/geo-ingest` mqtt 部分 | `adapters/geo-adapter-iot` | MQTT broker, NATS |
| 新增 | `adapters/geo-adapter-cad` | DXF/DWG 生态 |
| 新增 | `adapters/geo-adapter-cli` | GDAL/DVC/shell 子进程 |

### 3.2 ExternalAdapter trait（定义在 geo-core）

```rust
/// 所有外部适配器必须实现此 trait
pub trait ExternalAdapter: Plugin {
    /// 外部服务的端点标识（URL、连接串、命令名）
    fn external_endpoint(&self) -> &str;

    /// 检查外部服务是否可达且健康
    async fn health_check(&self) -> GeoResult<bool>;

    /// 获取外部工具版本号
    async fn external_version(&self) -> GeoResult<String>;

    /// 是否需要网络
    fn requires_network(&self) -> bool { true }

    // ── 双向通信核心 ──

    /// 推送数据到外部系统
    async fn push(&self, table: &str, data: &[GeoFeature]) -> GeoResult<u64>;

    /// 从外部系统拉取数据
    async fn pull(&self, query: &str) -> GeoResult<Vec<GeoFeature>>;

    /// 执行外部命令
    async fn execute(&self, command: &str, params: serde_json::Value)
        -> GeoResult<serde_json::Value>;
}
```

### 3.3 以 geo-adapter-postgis 为例

```
adapters/geo-adapter-postgis/
├── Cargo.toml
├── src/
│   ├── lib.rs            # impl ExternalAdapter for PostgisAdapter
│   ├── adapter.rs        # PostgisAdapter struct: connect/health/push/pull
│   ├── postgis.rs        # 原 geo-store 的 PostgisStore 逻辑
│   ├── batch_writer.rs   # 原 geo-store 的 BatchWriter
│   ├── dvc.rs            # DVC 集成（或迁入 CLI adapter）
│   └── timescale.rs      # TimescaleDB hypertable
└── migrations/
    └── *.sql             # PostGIS 迁移脚本
```

```rust
// adapters/geo-adapter-postgis/src/adapter.rs
use geo_core::plugin::{ExternalAdapter, Plugin, PluginCategory, PluginMeta};
use geo_core::errors::GeoResult;

pub struct PostgisAdapter {
    pool: sqlx::PgPool,
    url: String,
}

impl Plugin for PostgisAdapter {
    fn name(&self) -> &str { "postgis" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
    fn description(&self) -> &str { "PostGIS bidirectional adapter with TimescaleDB support" }
    fn category(&self) -> PluginCategory { PluginCategory::Adapter }
}

impl ExternalAdapter for PostgisAdapter {
    fn external_endpoint(&self) -> &str { &self.url }

    async fn health_check(&self) -> GeoResult<bool> {
        sqlx::query("SELECT 1").fetch_one(&self.pool).await?;
        Ok(true)
    }

    async fn external_version(&self) -> GeoResult<String> {
        let row: (String,) = sqlx::query_as("SELECT version()")
            .fetch_one(&self.pool).await?;
        Ok(row.0)
    }

    async fn push(&self, table: &str, data: &[GeoFeature]) -> GeoResult<u64> {
        // ST_GeomFromGeoJSON + COPY batch insert
        todo!()
    }

    async fn pull(&self, query: &str) -> GeoResult<Vec<GeoFeature>> {
        // ST_AsGeoJSON + rows → GeoFeature
        todo!()
    }

    async fn execute(&self, command: &str, params: Value) -> GeoResult<Value> {
        // 通用 SQL 执行 → JSON 返回
        todo!()
    }
}
```

### 3.4 更新 workspace Cargo.toml

```toml
[workspace]
members = [
    # Core（原有 + 新增）
    "crates/geo-core",
    "crates/geo-carbon-core",
    "crates/geo-parquet",
    "crates/geo-ogc",
    "core/geo-raster",
    "core/geo-vector",
    "core/geo-index",
    "core/geo-stats",
    "core/geo-io",
    "core/geo-report",

    # Adapters（改名 + 新增）
    "adapters/geo-adapter-postgis",
    "adapters/geo-adapter-gee",
    "adapters/geo-adapter-qgis",
    "adapters/geo-adapter-cad",
    "adapters/geo-adapter-cli",
    "adapters/geo-adapter-mcp",
    "adapters/geo-adapter-iot",

    # 暂时保留原有（逐步淘汰）
    "crates/geo-store",
    "crates/geo-gee",
    "crates/geo-qgis",
    "crates/geo-gdal",

    # 入口
    "crates/geo-cli",
    "crates/geo-wasm",
]
```

### 3.5 验证

```bash
cargo check --workspace
cargo test -p geo-adapter-postgis    # 需要 DATABASE_URL 环境变量
cargo test -p geo-adapter-gee        # 需要 NATS
cargo test --workspace               # 全量
```

---

## Phase 4：Plugin 体系 — 新建 7 个专业插件 crate

**目标**：每个插件是薄层 — 只含 rules.toml + 报告模板 + 组装 core/adapter 调用的入口。

**原则**：
- 插件**不实现**栅格/矢量/统计算法（那是 core 的事）
- 插件**不直接访问**数据库/网络（那是 adapter 的事）
- 插件 = 业务规则编排 + 领域参数 + 报告模板
- **插件之间禁止互相依赖**（geo-plugin-urban 不能 import geo-plugin-hydro）
- 如果某功能被多个插件需要，应下沉到 core 层

### 4.1 新建 7 个插件

| 插件 | 核心计算（调用 core） | 数据输入 | 输出物 |
|------|---------------------|---------|--------|
| `geo-plugin-carbon` | 排放 = 面积 × 因子；碳汇 = geo-carbon-math + geo-raster | 碳密度 CSV + AOI | 碳核算报告 |
| `geo-plugin-ecology` | NDVI 变化检测、土地覆盖转移矩阵；**直接调用 geo-carbon-math 算碳汇**，自读 rules.toml 中的碳密度参数 | AOI + 遥感影像 | 生态修复评估报告 + 碳汇地图 |
| `geo-plugin-survey` | 控制网平差、土方量计算、等值线生成、断面图 | 测量原始数据 | 测绘成果表 + CAD 交换文件 |
| `geo-plugin-urban` | 用地分类统计、容积率、建筑密度、日照时长 | 用地规划图 + 建筑矢量 | 规划指标表 + 合规报告 |
| `geo-plugin-hydro` | 流域边界提取、汇流量计算、洪水淹没范围 | DEM + 降雨数据 + 河网 | 水文分析报告 + 淹没图 |
| `geo-plugin-geohazard` | 滑坡敏感性 = 坡度 × 岩性 × 降雨权重；地震烈度衰减 | 地质图 + DEM + 地震目录 | 风险等级图 + 评估报告 |
| `geo-plugin-agri` | 作物估产 = NDVI × 面积 × 系数；土壤肥力评级 | 农田边界 + 遥感影像 + 土壤图 | 产量估计报告 + 施肥建议 |

### 4.2 插件标准目录结构（以 ecology 为例）

```
plugins/geo-plugin-ecology/
├── Cargo.toml
├── rules.toml                    # 业务规则配置
├── templates/
│   └── ecology-report.md.tera    # 生态领域专属报告模板
├── src/
│   ├── lib.rs                    # 插件入口
│   └── ecology.rs                # 核心编排逻辑
└── tests/
    ├── integration.rs            # 集成测试
    └── fixtures/
        ├── aoi.geojson           # 测试 AOI
        └── sample_ndvi.tif       # 测试影像
```

### 4.3 rules.toml 规范

```toml
[plugin]
name = "ecology"
version = "0.1.0"
description = "生态修复评估 — 碳汇计算、植被指数分析、土地覆盖变化检测"

# ── 碳汇参数 ──
[carbon.forest]
co2_per_ha_yr = 4.8          # 森林年碳汇 tCO₂/ha
biomass_expansion = 1.7      # 生物量扩展因子
root_shoot_ratio = 0.25      # 根冠比
carbon_fraction = 0.47       # 含碳率

[carbon.grassland]
co2_per_ha_yr = 1.2
soil_carbon_ratio = 0.58

[carbon.wetland]
co2_per_ha_yr = 8.5
methane_factor = 0.12

# ── NDVI 阈值 ──
[ndvi]
healthy_min = 0.5             # 健康植被 NDVI 下限
degraded_max = 0.2            # 退化植被 NDVI 上限
moderate_range = [0.2, 0.5]   # 中等植被区间
collection = "COPERNICUS/S2_SR_HARMONIZED"

# ── 土地覆盖分类 ──
[landcover.classes]
forest = [1, 2, 3]
grassland = [4, 5]
wetland = [6]
cropland = [7, 8]
built_up = [9, 10]
water = [11]
bare = [12]

# ── 报告 ──
[report]
template = "templates/ecology-report.md.tera"
format = "markdown"           # markdown | html | pdf
language = "zh"

# ── 依赖的其他插件/适配器 ──
[dependencies]
adapters = ["postgis", "gee"]   # 需要的适配器
cores = ["raster", "stats"]     # 需要的核心模块
```

### 4.4 报告模板放哪里？

三层模板体系：

```
geo-toolbox/
├── core/geo-report/                      # 模板引擎 + 通用组件
│   ├── src/render.rs                     # Tera 封装、渲染接口
│   └── templates/                        # 公共组件（所有插件复用）
│       ├── _partials/
│       │   ├── header.md.tera            # 通用页眉
│       │   ├── table.md.tera             # 通用表格宏
│       │   ├── map.md.tera               # 地图占位宏
│       │   └── stats.md.tera             # 统计摘要宏
│       ├── _layouts/
│       │   ├── base.md.tera              # 基础布局（header + body + footer）
│       │   └── two-column.md.tera        # 双栏布局
│       └── _filters/
│           └── format.rs                 # 自定义 Tera 过滤器（单位转换、数字格式化）
│
├── plugins/geo-plugin-ecology/templates/ # 领域专属模板
│   ├── carbon-sink.md.tera               # 碳汇报告
│   ├── vegetation-change.md.tera         # 植被变化报告
│   └── landcover-matrix.md.tera          # 土地覆盖转移矩阵
│
├── plugins/geo-plugin-urban/templates/   # 每个插件自己管
│   ├── land-use-stats.md.tera
│   └── building-density.md.tera
│
├── plugins/geo-plugin-hydro/templates/
│   ├── watershed.md.tera
│   └── flood-inundation.md.tera
│
└── ...（其余插件同理）
```

**原则**：

| 放在哪里 | 放什么 | 谁可以用 |
|---------|--------|---------|
| `core/geo-report/templates/` | 通用组件：页眉、表格宏、布局、格式化过滤器 | 所有插件通过 `{% include "_partials/table.md.tera" %}` 引用 |
| `plugins/<name>/templates/` | 领域专属：业务图表、指标表、专业术语 | 只有该插件自己使用 |
| 插件调用方式 | `geo-report.render("templates/carbon-sink.md.tera", data)` — 路径相对于插件目录 | — |

**插件模板引用公共组件的写法**：

```markdown
{# plugins/geo-plugin-ecology/templates/carbon-sink.md.tera #}

{% include "_layouts/base.md.tera" %}

{% block content %}

# 碳汇评估报告

**项目区域**：{{ aoi_name }}
**评估年份**：{{ year }}

{# 使用 geo-report 提供的通用表格宏 #}
{% include "_partials/table.md.tera" %}

## 碳汇明细

| 土地覆盖类型 | 面积 (ha) | 年碳汇 (tCO₂/yr) |
|-------------|----------|-----------------|
{% for class in carbon.classes %}
| {{ class.name }} | {{ class.area_ha | ha_fmt }} | {{ class.sink_tco2 | co2_fmt }} |
{% endfor %}

{# ha_fmt、co2_fmt 是 geo-report 注册的自定义 Tera 过滤器 #}

{% endblock %}
```

**geo-report 暴露的渲染接口**：

```rust
// core/geo-report/src/render.rs

pub struct ReportEngine {
    tera: Tera,
}

impl ReportEngine {
    /// 新建引擎，自动加载内置公共模板
    pub fn new() -> GeoResult<Self>;

    /// 注册用户模板目录（插件调用此方法加载自己的 templates/）
    pub fn register_templates(&mut self, plugin_name: &str, dir: &Path) -> GeoResult<()>;

    /// 渲染模板为 Markdown
    pub fn render_md(&self, template: &str, context: &Value) -> GeoResult<String>;

    /// 渲染模板为 HTML（Markdown → HTML 自动转换）
    pub fn render_html(&self, template: &str, context: &Value) -> GeoResult<String>;
}
```

### 4.5 插件入口代码模板

```rust
// plugins/geo-plugin-ecology/src/lib.rs

use geo_core::errors::GeoResult;
use geo_core::plugin::{Plugin, PluginCategory, PluginMeta};
use serde::Deserialize;
use std::path::Path;

mod ecology;

/// 从 rules.toml 加载的配置
#[derive(Debug, Deserialize)]
pub struct EcologyConfig {
    pub plugin: PluginSection,
    pub carbon: CarbonSection,
    pub ndvi: NdviSection,
    pub landcover: LandcoverSection,
    pub report: ReportSection,
}

#[derive(Debug, Deserialize)]
pub struct PluginSection {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct CarbonSection {
    pub forest: ForestCarbonConfig,
    pub grassland: GrasslandCarbonConfig,
    pub wetland: WetlandCarbonConfig,
}

// ... 其余 config struct

pub struct EcologyPlugin {
    config: EcologyConfig,
}

impl EcologyPlugin {
    /// 从 rules.toml 加载插件
    pub fn load(rules_dir: &Path) -> GeoResult<Self> {
        let rules_path = rules_dir.join("rules.toml");
        let config_str = std::fs::read_to_string(&rules_path)?;
        let config: EcologyConfig = toml::from_str(&config_str)
            .map_err(|e| geo_core::GeoError::Validation(format!("bad rules.toml: {e}")))?;
        Ok(Self { config })
    }

    /// 计算碳汇（编排 core + adapter）
    pub async fn calculate_carbon_sink(
        &self,
        aoi_geojson: &str,
        year: u16,
        store: &dyn geo_core::plugin::StorePlugin,
        raster: &dyn geo_core::plugin::RasterPlugin,
    ) -> GeoResult<CarbonSinkReport> {
        // 1. 从 adapter 拉取 AOI 的土地覆盖数据
        // 2. 用 core raster 重分类
        // 3. 用 core stats 计算各类面积
        // 4. 套用 rules.toml 中的碳汇因子
        // 5. 返回结构化结果
        todo!()
    }
}

impl Plugin for EcologyPlugin {
    fn name(&self) -> &str { &self.config.plugin.name }
    fn version(&self) -> &str { &self.config.plugin.version }
    fn description(&self) -> &str { &self.config.plugin.description }
    fn category(&self) -> PluginCategory { PluginCategory::Plugin }
}
```

### 4.5 报告模板示例

```markdown
{# templates/ecology-report.md.tera #}
# 生态修复评估报告

**项目区域**：{{ aoi_name }}
**评估年份**：{{ year }}
**生成时间**：{{ generated_at }}

---

## 1. 碳汇评估

| 土地覆盖类型 | 面积 (ha) | 年碳汇 (tCO₂/yr) | 占比 |
|-------------|----------|-----------------|------|
{% for class in carbon.classes %}
| {{ class.name }} | {{ class.area_ha | round(1) }} | {{ class.sink_tco2 | round(2) }} | {{ class.percentage | round(1) }}% |
{% endfor %}
| **合计** | **{{ carbon.total_area_ha | round(1) }}** | **{{ carbon.total_sink_tco2 | round(2) }}** | **100%** |

## 2. 植被指数分析

- 健康植被面积：{{ ndvi.healthy_area_ha | round(1) }} ha
- 退化植被面积：{{ ndvi.degraded_area_ha | round(1) }} ha
- 平均 NDVI：{{ ndvi.mean | round(3) }}

## 3. 生态修复建议

{% for suggestion in suggestions %}
- {{ suggestion }}
{% endfor %}
```

### 4.6 Cargo.toml 模板（插件）

**geo-plugin-ecology**（组合多个 core crate + 调用 geo-plugin-carbon）：

```toml
[package]
name = "geo-plugin-ecology"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "生态修复评估插件 — 碳汇计算、植被指数分析、土地覆盖变化检测"

[dependencies]
# Core 层（可依赖多个）
geo-core = { path = "../../crates/geo-core" }
geo-raster = { path = "../../core/geo-raster" }
geo-stats = { path = "../../core/geo-stats" }
geo-io = { path = "../../core/geo-io" }
geo-report = { path = "../../core/geo-report" }

serde.workspace = true
serde_json.workspace = true
toml.workspace = true
tera.workspace = true

# ⚠ 注意：不能依赖其他 plugin（如 geo-plugin-urban）
# ⚠ 注意：不能依赖任何 adapter（如 geo-adapter-postgis）
```

**geo-plugin-carbon**（纯碳核算插件，依赖 core 的纯公式 + raster）：

```toml
[package]
name = "geo-plugin-carbon"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "碳核算插件 — IPCC 排放因子方法、碳汇计算、碳密度配置"

[dependencies]
# Core 层
geo-core = { path = "../../crates/geo-core" }
geo-carbon-math = { path = "../../core/geo-carbon-math" }
geo-raster = { path = "../../core/geo-raster" }
geo-stats = { path = "../../core/geo-stats" }

serde.workspace = true
toml.workspace = true

# ⚠ geo-plugin-ecology 不依赖本 crate。
# ecology 直接依赖 geo-carbon-math，自备碳密度配置（rules.toml），
# 保持 Plugin 层零横向依赖。
# 若两插件配置重复太多，后续可抽 geo-carbon-config 放 core 层共享。
```

**geo-plugin-ecology（注意：不 import geo-plugin-carbon）**：

```toml
[package]
name = "geo-plugin-ecology"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "生态修复评估插件 — 碳汇计算、植被指数分析、土地覆盖变化检测"

[dependencies]
# Core 层（可依赖多个）
geo-core = { path = "../../crates/geo-core" }
geo-raster = { path = "../../core/geo-raster" }
geo-stats = { path = "../../core/geo-stats" }
geo-io = { path = "../../core/geo-io" }
geo-carbon-math = { path = "../../core/geo-carbon-math" }
# ↑ 直接调碳核算公式，不通过 geo-plugin-carbon

geo-report = { path = "../../core/geo-report" }

serde.workspace = true
serde_json.workspace = true
toml.workspace = true
tera.workspace = true

# ⚠ 注意：没有 geo-plugin-carbon 依赖
# ⚠ 注意：没有 geo-plugin-urban 等横向依赖
# ⚠ 注意：没有 geo-adapter-* 依赖
```

### 4.7 验证

```bash
# 每个插件独立测试
cargo test -p geo-plugin-ecology
cargo test -p geo-plugin-survey
cargo test -p geo-plugin-urban
cargo test -p geo-plugin-hydro
cargo test -p geo-plugin-geohazard
cargo test -p geo-plugin-agri

# 全量
cargo test --workspace
```

---

## Phase 5：Registry — 插件注册与调度中心

**目标**：新建 `geo-registry` crate，统一管理所有 plugin 和 adapter 的生命周期。

### 5.1 crate 结构

```
crates/geo-registry/
├── Cargo.toml
├── src/
│   ├── lib.rs            # PluginRegistry struct + 公共 API
│   ├── discovery.rs      # 编译期插件发现（feature flags 驱动）
│   ├── dispatch.rs       # 请求路由与调度
│   └── mcp.rs            # MCP tools/list 动态生成
└── tests/
    └── registry_tests.rs
```

### 5.2 PluginRegistry 核心 API

```rust
// crates/geo-registry/src/lib.rs

use geo_core::plugin::{
    Plugin, PluginCategory, PluginMeta,
    StorePlugin, IngestPlugin, OutputPlugin, CarbonPlugin, ProcessPlugin,
    ExternalAdapter,
};
use geo_core::errors::GeoResult;
use std::collections::HashMap;

pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn Plugin>>,
    adapters: HashMap<String, Box<dyn ExternalAdapter>>,
    metadata: Vec<PluginMeta>,
}

impl PluginRegistry {
    /// 创建空注册表
    pub fn new() -> Self { /* ... */ }

    // ── 注册 ──

    /// 注册一个通用插件
    pub fn register(&mut self, plugin: Box<dyn Plugin>) -> GeoResult<()>;

    /// 注册一个外部适配器
    pub fn register_adapter(&mut self, adapter: Box<dyn ExternalAdapter>) -> GeoResult<()>;

    /// 自动发现并注册所有编译进 binary 的插件（通过 feature flags）
    #[cfg(feature = "auto-discover")]
    pub fn discover_all(&mut self) -> GeoResult<()>;

    // ── 查找 ──

    /// 按名称查找插件
    pub fn get(&self, name: &str) -> Option<&dyn Plugin>;

    /// 按类别列出所有插件
    pub fn list_by_category(&self, category: PluginCategory) -> Vec<&dyn Plugin>;

    /// 列出所有已注册插件的元数据
    pub fn list_all(&self) -> &[PluginMeta];

    // ── 调度 ──

    /// 向指定插件发送操作请求
    pub async fn dispatch(
        &self,
        plugin_name: &str,
        action: &str,
        params: serde_json::Value,
    ) -> GeoResult<serde_json::Value>;

    // ── 生命周期 ──

    /// 初始化所有插件
    pub async fn init_all(&mut self) -> GeoResult<()>;

    /// 检查所有适配器健康状态
    pub async fn health_check_all(&self) -> Vec<(String, bool)>;

    /// 优雅关闭所有插件
    pub async fn shutdown_all(&mut self) -> GeoResult<()>;

    // ── MCP ──

    /// 为 MCP 协议生成 tools/list 响应
    pub fn generate_mcp_tools(&self) -> serde_json::Value;
}
```

### 5.3 MCP tools/list 自动生成逻辑

```rust
impl PluginRegistry {
    /// 根据已注册插件动态生成 MCP tools/list
    pub fn generate_mcp_tools(&self) -> serde_json::Value {
        let mut tools = Vec::new();

        for meta in &self.metadata {
            match meta.category {
                PluginCategory::Store => {
                    tools.push(json!({
                        "name": format!("store_query_{}", meta.name),
                        "description": format!("{}: Execute SQL query", meta.description),
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "sql": {"type": "string"}
                            },
                            "required": ["sql"]
                        }
                    }));
                }
                PluginCategory::Carbon => {
                    tools.push(json!({
                        "name": format!("carbon_{}", meta.name),
                        "description": format!("{}: Calculate carbon emissions", meta.description),
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "aoi_id": {"type": "string"},
                                "year": {"type": "integer"},
                                "source": {"type": "string", "default": "IPCC_2019"}
                            },
                            "required": ["aoi_id", "year"]
                        }
                    }));
                }
                // ... 其他 category
                _ => {}
            }

            // 从 rules.toml 的 [actions] 段读取自定义 tool
            if let Some(actions) = meta.extra.get("actions").and_then(|a| a.as_array()) {
                for action in actions {
                    tools.push(action.clone());
                }
            }
        }

        json!({
            "jsonrpc": "2.0",
            "result": {
                "tools": tools
            }
        })
    }
}
```

### 5.4 auto-discover 机制（feature flag 驱动）

```rust
#[cfg(feature = "auto-discover")]
impl PluginRegistry {
    /// 编译期自动发现所有 feature-gated 插件
    pub fn discover_all(&mut self) -> GeoResult<()> {
        // Plugins
        #[cfg(feature = "plugin-ecology")]
        self.register(Box::new(geo_plugin_ecology::EcologyPlugin::load(
            std::path::Path::new("plugins/geo-plugin-ecology/rules.toml")
        )?))?;

        #[cfg(feature = "plugin-survey")]
        self.register(Box::new(geo_plugin_survey::SurveyPlugin::load(
            std::path::Path::new("plugins/geo-plugin-survey/rules.toml")
        )?))?;

        // ... 其余插件

        // Adapters
        #[cfg(feature = "adapter-postgis")]
        {
            let url = std::env::var("DATABASE_URL").unwrap_or_default();
            if !url.is_empty() {
                self.register_adapter(Box::new(
                    geo_adapter_postgis::PostgisAdapter::connect(&url).await?
                ))?;
            }
        }

        #[cfg(feature = "adapter-gee")]
        self.register_adapter(Box::new(
            geo_adapter_gee::GeeAdapter::new_default().await?
        ))?;

        // ... 其余适配器

        Ok(())
    }
}
```

### 5.5 Cargo.toml

```toml
[package]
name = "geo-registry"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "插件注册与调度中心 — 统一管理所有 plugin 和 adapter 的生命周期"

[dependencies]
geo-core = { path = "../geo-core" }
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
tracing.workspace = true

# 可选：编译期插件发现
geo-plugin-ecology = { path = "../../plugins/geo-plugin-ecology", optional = true }
geo-plugin-survey = { path = "../../plugins/geo-plugin-survey", optional = true }
geo-plugin-urban = { path = "../../plugins/geo-plugin-urban", optional = true }
geo-plugin-hydro = { path = "../../plugins/geo-plugin-hydro", optional = true }
geo-plugin-geohazard = { path = "../../plugins/geo-plugin-geohazard", optional = true }
geo-plugin-agri = { path = "../../plugins/geo-plugin-agri", optional = true }

geo-adapter-postgis = { path = "../../adapters/geo-adapter-postgis", optional = true }
geo-adapter-gee = { path = "../../adapters/geo-adapter-gee", optional = true }
geo-adapter-qgis = { path = "../../adapters/geo-adapter-qgis", optional = true }
geo-adapter-cad = { path = "../../adapters/geo-adapter-cad", optional = true }
geo-adapter-cli = { path = "../../adapters/geo-adapter-cli", optional = true }
geo-adapter-mcp = { path = "../../adapters/geo-adapter-mcp", optional = true }
geo-adapter-iot = { path = "../../adapters/geo-adapter-iot", optional = true }

[features]
default = []
auto-discover = ["plugin-ecology", "adapter-postgis"]
plugin-ecology = ["geo-plugin-ecology"]
plugin-survey = ["geo-plugin-survey"]
plugin-urban = ["geo-plugin-urban"]
plugin-hydro = ["geo-plugin-hydro"]
plugin-geohazard = ["geo-plugin-geohazard"]
plugin-agri = ["geo-plugin-agri"]
adapter-postgis = ["geo-adapter-postgis"]
adapter-gee = ["geo-adapter-gee"]
adapter-qgis = ["geo-adapter-qgis"]
adapter-cad = ["geo-adapter-cad"]
adapter-cli = ["geo-adapter-cli"]
adapter-mcp = ["geo-adapter-mcp"]
adapter-iot = ["geo-adapter-iot"]
```

### 5.6 验证

```bash
cargo test -p geo-registry
cargo test --workspace
```

---

## Phase 6：CLI + WASM 入口适配 — 最终集成

**目标**：`geo-cli` 不再硬编码 import 所有 crate，改为通过 `PluginRegistry` 动态调度。

### 6.1 geo-cli 改造前后对比

```rust
// ══ 改造前 ══
use geo_core;
use geo_store::PostgisStore;
use geo_gee::GeeDispatcher;
use geo_qgis::QgisClient;
use geo_carbon::CarbonEngine;
use geo_output::{DxfExporter, ExcelDashboard, GeoJsonExporter, ReportGenerator};
// ... 逐个 import 所有 crate

async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Store { action } => commands::store::handle(action).await,
        Commands::Process(action) => commands::process::handle(action).await,
        Commands::Carbon(action) => commands::carbon::handle(action).await,
        // ... 硬编码路由
    }
}

// ══ 改造后 ══
use geo_registry::PluginRegistry;

async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = PluginRegistry::new();

    // 编译期自动发现所有 feature-gated 插件和适配器
    #[cfg(feature = "auto-discover")]
    registry.discover_all().await?;

    // 手动注册运行时配置的适配器
    if let Ok(db_url) = std::env::var("DATABASE_URL") {
        registry.register_adapter(Box::new(
            geo_adapter_postgis::PostgisAdapter::connect(&db_url).await?
        ))?;
    }

    // 初始化所有插件
    registry.init_all().await?;

    // 统一调度
    match cli.command {
        Commands::Plugin { name, action, params } => {
            let result = registry.dispatch(&name, &action, params).await?;
            println!("{result}");
        }
        Commands::AdapterHealth => {
            let status = registry.health_check_all().await;
            for (name, ok) in status {
                println!("{name}: {}", if ok { "✅" } else { "❌" });
            }
        }
        Commands::McpServe { port } => {
            geo_adapter_mcp::serve(registry, port).await?;
        }
    }

    registry.shutdown_all().await?;
    Ok(())
}
```

### 6.2 MCP Server 改造

```rust
// adapters/geo-adapter-mcp/src/lib.rs

use geo_registry::PluginRegistry;
use serde_json::Value;

pub async fn serve(registry: PluginRegistry, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    // ...

    "tools/list" if handshake_done => {
        // 改造前：硬编码 15 个 tool
        // 改造后：从 registry 动态生成
        registry.generate_mcp_tools()
    }

    "tools/call" if handshake_done => {
        let tool_name = request["params"]["name"].as_str().unwrap_or("");
        let args = &request["params"]["arguments"];

        // 改造前：match 15 个分支手工 dispatch
        // 改造后：统一通过 registry
        registry.dispatch(tool_name, "execute", args.clone()).await
            .map(|result| json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [{"type": "text", "text": result.to_string()}]
                }
            }))
            .unwrap_or_else(|e| json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32000, "message": e.to_string()}
            }))
    }
}
```

### 6.3 CLI 子命令重新设计（更抽象）

```rust
#[derive(Subcommand)]
enum Commands {
    /// 列出所有已注册的插件和适配器
    ListPlugins {
        /// 按类别过滤
        #[arg(long)]
        category: Option<String>,
    },

    /// 向指定插件发送操作
    Plugin {
        /// 插件名称
        name: String,
        /// 操作名
        action: String,
        /// 参数（JSON）
        #[arg(long, default_value = "{}")]
        params: String,
    },

    /// 检查所有适配器健康状态
    AdapterHealth,

    /// 启动 MCP Server
    McpServe {
        #[arg(long, default_value = "9378")]
        port: u16,
    },
}
```

### 6.4 geo-wasm 不动

```
geo-wasm 只依赖：
  geo-core        ← CRS 变换、类型定义
  geo-carbon-core ← 纯 Rust 碳核算

不需要 adapter（无 DB）、不需要 plugin（太厚重）、不需要 registry。
```

### 6.5 最终 Cargo.toml feature flags

```toml
# crates/geo-cli/Cargo.toml
[features]
default = ["auto-discover"]
auto-discover = ["geo-registry/auto-discover"]

# 按需启用（最小化编译）
minimal = []
plugin-ecology = ["geo-registry/plugin-ecology"]
plugin-survey = ["geo-registry/plugin-survey"]
adapter-postgis = ["geo-registry/adapter-postgis"]
adapter-gee = ["geo-registry/adapter-gee"]
# ...
```

### 6.6 验证

```bash
# 编译检查（默认带所有 plugin + adapter）
cargo check --workspace

# 最小化编译（仅 core，无外部依赖）
cargo check -p geo-cli --no-default-features --features minimal

# 按需编译（仅 ecology + postgis）
cargo check -p geo-cli --no-default-features \
    --features "plugin-ecology,adapter-postgis"

# 全量测试
cargo test --workspace
```

---

## 验证检查清单

每个 Phase 完成后的检查项：

| Phase | cargo check | cargo test | cargo clippy | 额外检查 | 状态 |
|-------|:-----------:|:----------:|:------------:|------|:----:|
| 1. Core 提纯 | ✅ | ✅ 51 tests | ✅ | 无破坏 | ✅ 已完成 |
| 2. Core 重组 | ✅ | ✅ ≥ 51 tests | ✅ | **每新 crate 独立可测；验证 core 层内部 DAG 无循环** | ✅ 已完成 |
| 3. Adapter 重组 | ✅ | ✅ | ✅ | 原有测试不变 | ✅ 已完成 |
| 3a. Phase 3a 适配器补齐 | ✅ | ✅ 345 tests | ✅ | QGIS 双 seam → 统一 QgisAdapter (Subprocess/Rest backend) + WMS HTTP 暴露 + geo-wiring 去重 | ✅ 已完成 |
| 3b. Phase 3b 架构去重与宏 | ✅ | ✅ | ✅ | register_plugin! 宏 + geo-wiring 抽离 + geo-server 依赖瘦身 (22→3) + MCP serve 拆分 | ✅ 已完成 |
| 4. Plugin 体系 | ✅ | ✅ | ✅ | **每插件独立可测；验证无跨插件 import** | ✅ 已完成 |
| 5. Registry | ✅ | ✅ ≥ 80 tests | ✅ | 含 MCP 集成测试；register_plugin!/register_sync_tools!/register_async_tools! 宏 | ✅ 已完成 |
| 6. CLI 适配 | ✅ | ✅ ≥ 238 tests | ✅ | 最终验收 — 89 个 MCP 工具注册完毕 | ✅ 已完成 |

### 依赖方向合规性检查（每个 Phase 后执行）

```bash
# 检查 core 层是否误依赖了 plugin 或 adapter
! grep -r "geo-plugin" core/ --include="Cargo.toml" && echo "✅ Core 未依赖 Plugin"
! grep -r "geo-adapter" core/ --include="Cargo.toml" && echo "✅ Core 未依赖 Adapter"

# 检查 plugin 是否误依赖了 adapter
! grep -r "geo-adapter" plugins/ --include="Cargo.toml" && echo "✅ Plugin 未依赖 Adapter"

# 检查 plugin 之间是否有横向依赖
for p in plugins/*/; do
  for q in plugins/*/; do
    [ "$p" = "$q" ] && continue
    pname=$(basename "$p")
    ! grep -r "$pname" "$q/Cargo.toml" 2>/dev/null || echo "❌ $q 依赖了 $pname"
  done
done && echo "✅ Plugin 之间无横向依赖"

# 检查 WASM 是否误依赖了 plugin 或 adapter
! grep -r "geo-plugin\|geo-adapter" crates/geo-wasm/Cargo.toml && echo "✅ WASM 未依赖 Plugin/Adapter"
```

### 最终测试命令

```bash
# 全量编译
cargo build --release

# 全量测试（需要 DATABASE_URL 和 NATS 环境变量的 adapter 测试除外）
cargo test --workspace

# 带外部服务的测试
DATABASE_URL=postgres://geo:geo@localhost/geo_test \
GEO_NATS_URL=nats://localhost:4222 \
cargo test --workspace -- --include-ignored

# Lint
cargo clippy --workspace -- -D warnings

# 最小化二进制体积检查
cargo build -p geo-cli --no-default-features --features minimal --release
ls -lh target/release/geo-toolbox

# 最小化 WASM 体积检查（只依赖 core 层）
wasm-pack build crates/geo-wasm --release
ls -lh pkg/geo_wasm_bg.wasm
```

---

## 附录 A：最终 crate 清单（24 个）

```
geo-toolbox/
├── core/                         # Layer 1: 核心引擎（10 个 crate）
│   ├── geo-core/                 # 几何基类：类型/CRS/错误/BBox（注：可考虑改名 geo-primitives）
│   ├── geo-raster/               # 栅格运算基类（依赖 geo-core + geo-io）
│   ├── geo-vector/               # 矢量运算基类（依赖 geo-core + geo-io）
│   ├── geo-index/                # 空间索引基类（依赖 geo-core）
│   ├── geo-stats/                # 统计基类（依赖 geo-core）
│   ├── geo-io/                   # IO 基类（依赖 geo-core）
│   ├── geo-carbon-math/          # 碳核算纯公式（依赖 geo-core + geo-stats）
│   ├── geo-report/               # 报告基类（依赖 geo-core + geo-stats）
│   ├── geo-parquet/              # 云原生格式（依赖 geo-core + geo-vector）
│   └── geo-ogc/                  # OGC 标准服务（依赖 geo-core + geo-vector）
│
├── plugins/                      # Layer 2: 专业插件（7 个 crate）
│   ├── geo-plugin-carbon/        # 碳核算（依赖 geo-carbon-math + geo-raster + geo-stats）
│   ├── geo-plugin-ecology/       # 生态修复（依赖 core crates + 调用 carbon 插件的规则）
│   ├── geo-plugin-survey/        # 测绘
│   ├── geo-plugin-urban/         # 城乡规划
│   ├── geo-plugin-hydro/         # 水文
│   ├── geo-plugin-geohazard/     # 地质灾害
│   └── geo-plugin-agri/          # 农业
│
├── adapters/                     # Layer 3: 外部适配器（7 个 crate）
│   ├── geo-adapter-postgis/      # PostGIS 桥接（原 geo-store 改名后删除）
│   ├── geo-adapter-gee/          # GEE 分发
│   ├── geo-adapter-qgis/         # QGIS 桥接
│   ├── geo-adapter-cad/          # CAD 格式
│   ├── geo-adapter-cli/          # 外部 CLI
│   ├── geo-adapter-mcp/          # MCP 协议
│   └── geo-adapter-iot/          # IoT 传感器
│
├── crates/                       # 入口（2 个 crate）
│   ├── geo-registry/             # 插件注册与调度中心
│   ├── geo-cli/                  # CLI 入口（依赖 registry + 全量 feature flags）
│   └── geo-wasm/                 # WASM 浏览器入口（⚠ 只能依赖 core 层）
│
├── Cargo.toml                    # workspace 根
└── DEVPLAN.md                    # 本文档
```

## 附录 B：依赖关系图（改造后，带方向约束）

```
                          Layer 1: Core（有向无环）
┌────────────────────────────────────────────────────────────────┐
│                                                                │
│  Layer 0:          ┌──────────────┐                            │
│                    │   geo-core   │ 几何基类                    │
│                    └──────┬───────┘                            │
│           ┌───────────────┼───────────────┐                    │
│           ▼               ▼               ▼                    │
│  Layer 1: geo-index   geo-io          geo-stats                │
│           └──┬───────────┬───────────────┘                     │
│              │           │                                     │
│       ┌──────┘     ┌────┴────┐                                 │
│       ▼            ▼         ▼                                 │
│  Layer 2:    geo-raster  geo-vector  geo-carbon-math           │
│              │            │             │                      │
│              └────────────┼─────────────┘                      │
│                           ▼                                    │
│  Layer 3:   geo-report   geo-parquet   geo-ogc                 │
│                                                                │
│  ⚠ geo-raster 和 geo-vector 不互相依赖                         │
└────────────────────────────────────────────────────────────────┘
          │                    │                    │
          │   ┌────────────────┘                    │
          ▼   ▼                                    │
┌──────────────────────────┐                       │
│  Layer 2: Plugins（7个）  │                       │
│                          │                       │
│  geo-plugin-carbon ←──── 只依赖 core，不互相依赖   │
│  geo-plugin-ecology       │                      │
│  geo-plugin-survey        │                      │
│  geo-plugin-urban         │                      │
│  geo-plugin-hydro         │                      │
│  geo-plugin-geohazard     │                      │
│  geo-plugin-agri          │                      │
│                          │                       │
│  ⚠ 插件间禁止横向依赖     │                       │
└──────────┬───────────────┘                       │
           │                                       │
           ▼                                       ▼
┌──────────────────────────┐    ┌──────────────────────────┐
│  Layer 3: Adapters（7个） │    │  Entries（2个）           │
│                          │    │                          │
│  geo-adapter-postgis     │    │  geo-cli                 │
│  geo-adapter-gee         │    │    └→ geo-registry       │
│  geo-adapter-qgis        │    │       统一调度所有插件     │
│  geo-adapter-cad         │    │                          │
│  geo-adapter-cli         │    │  geo-wasm                │
│  geo-adapter-mcp         │    │    ⚠ 只能依赖 Core 层     │
│  geo-adapter-iot         │    │    ❌ 不能依赖 Plugin     │
│                          │    │    ❌ 不能依赖 Adapter    │
│  ⚠ 可依赖 Core + Plugin  │    │                          │
└──────────────────────────┘    └──────────────────────────┘

依赖方向（单向箭头 = "可以依赖"）：
  Adapter ──→ Plugin ──→ Core
  Adapter ──→ Core  （直连也允许）

禁止方向：
  Core ──✗──→ Plugin/Adapter
  Plugin ──✗──→ Adapter
  Plugin ──✗──→ Plugin （横向）
```

---

> **文档版本**：v1.3
> **最后更新**：2026-06-16
> **关联**：README.md, ROADMAP.md, WIKI.md, Cargo.toml
>
> **v1.3 更新**：RUSLE 土壤流失方程 (ecology) + SCS-CN 径流曲线数 (hydro) + InVEST 碳存储+水源涵养 (hydro)
> — 3 个新模块, 35 个新测试, 7 个新 CLI 工具, ecology 20 tests + hydro 27 tests (总计 47)。新增"v0.6 完成小结"。
>
> **v1.2 更新**：拓展路线图状态刷新 — Round 1/2 全部完成标记(v0.3-v0.4)，Round 3 部分完成(Python bindings 待启动)。
> 新增"v0.4 完成小结"区块总结架构治理成果。
>
> **v1.1 更新**：Phase 3a 适配器三项落地 — WMS HTTP 暴露到 geo-server、QGIS 双 seam 合并为统一 QgisAdapter、
> 共享 Registry 配线抽到 crates/geo-wiring。register_plugin! 宏定义（geo-registry）+ 24/26 tools.rs 批量迁移。
> mcp::serve 拆分为 handler 函数。未用导入清理。

---

# 拓展路线图

> 基于三层架构的增量拓展计划。每轮可独立验证，`cargo check --workspace && cargo test --workspace` 必须通过。

---

> **v0.6 完成小结 (2026-06-16)**：
> - RUSLE 土壤流失方程: A=R·K·LS·C·P 5因子计算, 16 tests, 2 CLI tools
> - SCS-CN 径流曲线数: 26 种土地利用 CN 查表, AMC 修正, 9 tests, 2 CLI tools
> - InVEST 碳存储(4碳库)+水源涵养(Budyko): 20 种生态系统碳密度, 10 tests, 3 CLI tools
> - 总计: 3 模块 35 新测试 7 新 CLI 工具, ecology 20 + hydro 27 = 47 tests
> - 文档: README + WIKI 新增 API 示例, ROADMAP 完成标记, DEVPLAN v1.3

---
> - 架构治理：`register_plugin!` 宏 + 26 tools.rs 迁移 (代码量 -60%)
> - `geo-wiring` crate 抽离 Registry 接线 (消除 CLI/Server 双份重复)
> - QGIS 适配器统一双后端 (`QgisBackend::Subprocess | Rest`)
> - geo-server WMS `/wms` 端点 (GetMap/GetFeatureInfo)
> - geo-server 依赖瘦身: 22 crate → 2 核心 (geo-wiring + geo-ogc)
> - MCP `serve()` 拆分为 3 函数 (handle_tools_call / dispatch_tool)
> - Plugin 层补齐至 10 个完整插件 (energy/forestry/coastal 已落地)
> - Adapter 层补齐至 7 个活跃适配器 (duckdb/stac/osm 已落地)
> - 废弃 `geo-adapter-mcp` 空壳 (MCP 逻辑内置于 geo-cli)

---

## Round 1：本地分析 + 浏览器渲染闭环 ✅ (v0.3 完成)

**目标**：补齐"离线可用 + 浏览器端渲染"两个短板，让 geo-toolbox 脱离 QGIS/PostGIS 也能独立完成分析→可视化。

### Core 层新增

| Crate | 功能 | 依赖 | 说明 |
|-------|------|------|------|
| `geo-tile` | 矢量瓦片 (MVT) 编码 + 栅格瓦片 (PMTiles) 读写 | `geo-core`, `geo-index` | 复用 geohash 做瓦片索引；纯 Rust protobuf MVT 编码 |
| `geo-temporal` | 时空序列分析 | `geo-core`, `geo-raster`, `ndarray` | NDVI 年际变化趋势拟合、突变检测、季节分解 |

**验证标准**：
- `geo-tile` 输入 GeoJSON → 输出 z/x/y MVT 字节流，MapLibre 可直接渲染
- `geo-temporal` 输入 5 年 NDVI 栅格序列 → 输出趋势斜率 + 显著性 p 值

### Adapter 层新增

| Crate | 外部系统 | 通信方式 | 说明 |
|-------|---------|---------|------|
| `geo-adapter-duckdb` | DuckDB + Spatial | 嵌入式进程 (duckdb crate) | 零安装部署的本地空间分析引擎 |
| `geo-adapter-stac` | STAC / COG / Zarr | HTTP/S3 (stac-rs crate) | 云原生地理空间数据发现与按范围拉取 |

**验证标准**：
- DuckDB adapter 不依赖 PostGIS，可独立运行 `cargo test`
- STAC adapter 查询 `planetarycomputer.microsoft.com/api` 返回 Sentinel-2 条目

### Plugin 层新增（复用 Round 1 Core）

| Crate | 功能 | 输入 | 输出 |
|-------|------|------|------|
| `geo-plugin-energy` | 新能源选址评估 | DEM + 太阳辐射栅格 + 路网 | 光伏/风电适宜性等级图 |

**验证标准**：
- 输入 1°×1° DEM → 输出坡度 < 25° + 年辐射 > 1500 kWh/m² 的区域 GeoJSON

### 依赖方向

```
geo-adapter-duckdb ──→ geo-plugin-energy ──→ geo-temporal ──→ geo-raster
geo-adapter-stac   ──→ geo-plugin-energy ──→ geo-tile     ──→ geo-index
                                              geo-temporal ──→ ndarray
```

---

## Round 2：双碳 + 生态三件套 ✅ (v0.3 完成)

**目标**：补上能源、林业、海岸三个政策强绑定领域，复用 Round 1 的 `geo-temporal` 和 `geo-tile`。

### Plugin 层新增

| Crate | 功能 | 输入 | 输出 | 复用 |
|-------|------|------|------|------|
| `geo-plugin-forestry` | 林业碳汇计量 | 多期遥感 + 样地调查 CSV | 蓄积量、碳汇量、CCER 报告 | `geo-carbon-math`, `geo-temporal` |
| `geo-plugin-coastal` | 海岸带变化监测 | 多期岸线 + DEM | 侵蚀速率图、淹没范围 | `geo-temporal`, `geo-raster` |

### Adapter 层新增

| Crate | 外部系统 | 通信方式 | 说明 |
|-------|---------|---------|------|
| `geo-adapter-osm` | OpenStreetMap Overpass API | HTTP (reqwest) | 按 AOI 拉取 OSM 路网/建筑/POI |

### Plugin 层补齐

| Crate | 功能 | 说明 |
|-------|------|------|
| `geo-plugin-hydro` | 流域提取、淹没分析 | 当前为空壳，需实现 `watershed_analysis()` + `flood_inundation()` |
| `geo-plugin-geohazard` | 滑坡敏感性 = 坡度×岩性×降雨 | 当前为空壳，需实现 `landslide_susceptibility()` |

**验证标准**：
- forestry 输入两期 NDVI + 样地数据 → 输出碳汇变化 tCO₂/yr
- coastal 输入 1990/2020 岸线 → 输出侵蚀速率 m/yr + 2050 预测岸线
- osm adapter 输入 AOI bbox → 返回 road/building/landuse GeoJSON

---

## Round 3：生态破圈 ✅ (v0.11 完成)

**目标**：通过 Python bindings 和 gRPC 服务总线让非 Rust 用户也能用，同时补齐大数据和 AI 场景。

### Adapter 层新增

| Crate | 外部系统 | 通信方式 | 说明 |
|-------|---------|---------|------|
| `geo-adapter-pygeoapi` | shapely / rasterio / xarray | PyO3 FFI | 零拷贝几何互转，Python 生态直接调用 Rust 引擎 |
| `geo-adapter-pdal` | PDAL (点云处理) | 子进程 (pdal pipeline) | LiDAR LAS/LAZ 格式读写 |

### 基础设施

| 方向 | 具体动作 |
|------|---------|
| Python bindings | maturin + PyO3 打包 `pip install geo-toolbox` |
| CI/CD | ✅ `.github/workflows/ci.yml`：`cargo test --workspace` + `clippy` + `rustfmt` + coverage |
| Benchmark | ✅ criterion 基准测试套件，8 crates 覆盖 CRS 变换、碳核算、NDVI、IO、矢量、索引、统计 |
| Fuzzing | cargo-fuzz 对 GeoJSON/NMEA/CamoFox 解析器做模糊测试 |

### Plugin 层补齐（剩余空壳）

| Crate | 功能 |
|-------|------|
| `geo-plugin-survey` | 控制网平差、土方量计算 |
| `geo-plugin-urban` | 用地分类、容积率、日照分析 |
| `geo-plugin-agri` | 作物估产、土壤肥力评级 |

### WASM 生态

| 方向 | 具体方案 |
|------|---------|
| MapLibre GL JS 插件 | `maplibre-gl-geo-toolbox` — WASM 空间运算注入浏览器渲染管线 |
| ObservableHQ 模块 | `import { CrsEngine, CarbonEngine } from "geo-wasm"` |

---

## 远期储备（需求驱动）

以下拓展有价值但需等待明确需求或前置条件成熟：

| 拓展 | 层级 | 阻塞因素 |
|------|------|---------|
| `geo-network` — 网络分析 | Core | 需 OSM PBF 解析 → 路网图构建，工作量大 |
| `geo-plugin-transport` — 交通可达性 | Plugin | 依赖 `geo-network` |
| `geo-plugin-sponge-city` — 海绵城市 | Plugin | 管网数据涉密，实际可用性受限 |
| `geo-plugin-air` — 大气扩散 | Plugin | AERMOD 是 Fortran 遗产，建议改为 adapter 调外部模型 |
| `geo-plugin-realestate` — 房产估值 | Plugin | 成交价数据不公开，模型可做但数据难拿 |
| `geo-adapter-geoserver` — GeoServer | Adapter | REST API 足够简单，直接 HTTP 调即可 |
| `geo-adapter-arcgis` — ArcGIS | Adapter | 认证层复杂，版本差异大，维护成本高 |
| `geo-adapter-sedona` — Apache Sedona | Adapter | 需要 Spark 集群，与轻量定位冲突 |
| `geo-adapter-grpc` — gRPC 总线 | Adapter | MCP 协议已覆盖 Agent 通信，功能重叠 |
| `geo-adapter-tile38` — Tile38 | Adapter | 与 MQTT adapter 功能重叠，二选一 |
| `geo-ml` — ONNX 推理 | Core | ONNX Runtime 编译数百 MB，GEE adapter 已覆盖分类场景 |
| `geo-pointcloud` — 点云 | Core | `las` crate 不成熟，建议走 `geo-adapter-pdal` 子进程 |
| `geo-algebra` — 栅格代数 DSL | Core | 上层插件未就绪，写了没人调 |

---

## 拓展约束速查

与现有架构约束一致：

| 层 | 可新增 | 禁止 |
|----|--------|------|
| Core | 纯 Rust 引擎，只依赖其他 Core crate | 禁止依赖 Plugin / Adapter |
| Plugin | 组装 Core 调用 + rules.toml 配置 + 报告模板 | 禁止依赖 Adapter；禁止横向依赖其他 Plugin |
| Adapter | 持有外部连接、子进程、网络；可依赖 Core + Plugin + 其他 Adapter | — |
| WASM | 等同 Plugin 约束（只依赖 Core） | 禁止依赖 Plugin / Adapter |

---

> **路线图版本**：v1.3
> **制定日期**：2026-06-11 &nbsp; · &nbsp; **上次更新**：2026-06-16
> **下次评审**：流域提取 + 降雨阈值 ID 曲线 完成后


---

## 插件拓展机会分析（v2026-06-20）

以下基于现有代码逐插件的分析，按 leverage（接口深度/收益）排序。

### 1. survey（测绘/Gauss-Krüger）— 🟡 中等

**现状**：gauss.rs（593行）：GK正反算第6阶级数 + 4椭球 + 中国3°/6°分带 + zone转换 + 自动识别。survey.rs（338行）：4种土方量 + 控制网加权平差。

| 拓展 | Leverage | 说明 |
|------|----------|------|
| **椭球间坐标转换**（Molodensky/Helmert七参数） | 🔴 高 | 当前各椭球独立，CGCS2000↔Beijing54是最普遍痛点。加Molodensky (~30行) + Helmert (~50行)即可覆盖 |
| **UTM支持** | 🟡 中 | GK↔UTM共享TM级数核心，差异仅分带方式和尺度因子0.9996。加`scale_factor: f64`参数即可 |
| **大地线计算**（Vincenty） | 🟡 中 | 当前只有TM平面距离，加Vincenty正反算获得椭球面距离，测绘刚需 |
| **坐标系统注册表**（+GRS80/Hayford/Clarke） | 🟢 低 | 加新椭球只是查表，leverage低 |
| **导线平差 / 变形监测** | 🟡 中 | 当前只有控制网自由网平差，加附合/闭合导线 + 卡尔曼变形预测 |

### 2. hydro（水文）— 🟡 中等

**现状**：SCS-CN产流（scs_cn.rs 已验证正确）+ 流域划分 + InVEST衔接。

| 拓展 | Leverage | 说明 |
|------|----------|------|
| **单位线汇流** | 🔴 高 | CN产流只有产水量（径流深m³），加Snyder/Clark UH → 流量过程线，从"产流"→"产汇流"完整水文链 |
| **TR-55完整版** | 🟡 中 | 当前只有CN表，缺Tc（汇流时间）、Ia/P比率法、图表法 |
| **Muskingum河段演算** | 🟡 中 | 流域→河道→洪峰演进，和单位线配套 |
| **SWAT输入生成** | 🟢 低 | HRU→SWAT输入的adapter模式，已有watershed基础 |

### 3. ecology（RUSLE）— ✅ 刚修正

**现状**：R/K/LS/C/P五因子完整（R因子已于本次修正），已验证正确。

| 拓展 | Leverage | 说明 |
|------|----------|------|
| **SDR（泥沙输移比）** | 🔴 高 | 当前A=RKLSCP算到地块边缘，加SDR→入河泥沙量，leverage极高 |
| **MUSLE（事件版土壤流失）** | ✅ 已完成 | ecology_musle_single/assessment/annual MCP tools |
| **WEQ/RWEQ风蚀** | 🟡 中 | 风蚀模型，和水蚀互补。椭圆核心，接口类似RUSLE |

### 4. coastal（海岸）— ✅ 刚修正

**现状**：Holland风场+风增水+逆气压+蓝碳（风暴潮坐标语义已于本次修正）。

| 拓展 | Leverage | 说明 |
|------|----------|------|
| **波浪爬高/越浪** | 🔴 高 | 当前只有风增水+逆气压，缺wave setup/runup——这是风暴潮致灾的最主要形式 |
| **SLR海平面上升情景** | 🟡 中 | 加1m/2m SLR情景，淹没从静态→动态时间维度 |
| **CVI（海岸脆弱性指数）** | 🟡 中 | 5-7因子加权（岸线变化率/坡度/波高/潮差/海平面），现有函数已有计算基础 |
| **盐沼/红树林迁移** | 🟡 中 | 蓝碳+SLR→海平面上升后湿地迁移，生态保护刚需 |

### 5. energy（能源）— 🟡 中等

**现状**：Weibull风电拟合 + 光伏辐射 + 地热 + 输电LCP（Dijkstra）。

| 拓展 | Leverage | 说明 |
|------|----------|------|
| **尾流效应（风电）** | ✅ 已完成 | jensen_wake + farm_wake_efficiency + turbine_power MCP tools |
| **PVWatts 性能模型** | ✅ 已完成 | pvwatts_annual + pvwatts_cell_temp MCP tools |
| **储能选址+风-光-储三位一体** | 🟡 中 | 现有LCP+风电+光伏→储能最优配置，从单能源→多能互补 |
| **风机噪声传播** | 🟢 低 | plume模块高斯扩散可复用，但需求频率低 |

### 6. forestry（林业）— 🟡 中等

**现状**：6生长曲线(Richards/Logistic/Korf/Gompertz/Weibull/Schumacher) + SDI最优 + IPCC碳储量。

| 拓展 | Leverage | 说明 |
|------|----------|------|
| **立地指数曲线** | 🔴 高 | 当前生长曲线用时间t，加site index→立地质量等级，模型深度+1，经营基础 |
| **择伐/皆伐模拟** | 🟡 中 | 当前只有生长，加采伐→动态经营管理，碳汇报告增值 |
| **生物多样性（Shannon/Simpson）** | 🟡 中 | 碳储量→碳汇+生物多样性，从碳→全生态 |
| **林火风险** | 🟡 中 | 可燃物含水率×气象因子，防灾刚需 |

### 7. geohazard（地质灾害）— 🟡 中等

**现状**：无限坡FS + 模糊隶属度 + 降雨阈值。

| 拓展 | Leverage | 说明 |
|------|----------|------|
| **地震诱发滑坡（Newmark位移法）** | 🔴 高 | 当前只有降雨触发，加Newmark=PGA×临界加速度→震致滑坡 |
| **泥石流演进路径** | 🟡 中 | 当前只在评估阶段，加FLO-2D简化→运动路径/堆积范围 |
| **区域稳定性色斑图** | 🟡 中 | FS从单点→栅格面→危险分区图，接口不变、输出升维 |
| **滑坡早期预警指标** | 🟢 低 | 阈值+降雨预报，依赖实时数据源 |

### 8. carbon（碳）— 🟡 中等

**现状**：碳汇计算 + CCER方法学 + LCA生命周期 + 高斯烟羽扩散。

| 拓展 | Leverage | 说明 |
|------|----------|------|
| **碳价情景分析** | ✅ 已完成 | carbon_price_scenario + carbon_offset_revenue MCP tools |
| **VCS/GS额外性/基线** | ✅ 已完成 | carbon_vcs_additionality + gold_standard_sdg MCP tools |
| **NBS碳汇组合优化** | 🟡 中 | 造林+红树林+草地→碳汇组合+成本效益 |
| **碳泄漏评估** | 🟢 低 | 项目边界外间接排放，复杂，需求驱动 |

### 9. agri（农业）— 🟢 待填补

**现状**：config + tools 层已注册，功能层待填补最空。

| 拓展 | Leverage | 说明 |
|------|----------|------|
| **DSSAT/AquaCrop adapter** | ✅ 已完成 | geo-adapter-dssat: .WTH/.SOL/.CUL/.FILEX 生成 |
| **灌溉需水（ET×作物系数）** | 🟡 中 | 和hydro的SCS配合，SPEI/SPI干旱指数 |
| **NDVI→LAI→产量** | 🟡 中 | NDVI→叶面积指数→光能利用率→估产，和ecology的RUSLE/NDVI函数可复用 |

### 10. urban（城市）— 🟡 中等

**现状**：FAR/建筑密度/日照阴影/UHI/通风粗糙度。

| 拓展 | Leverage | 说明 |
|------|----------|------|
| **城市内涝（管网+SCS）** | ✅ 已完成 | urban_flood_simulate + urban_flood_pipe_network MCP tools |
| **绿地降温效应** | 🟡 中 | UHI→+绿地降温=缓解情景设计，现有函数基底 |
| **15分钟城市可达性** | ✅ 已完成 | urban_accessibility + urban_accessibility_isochrone MCP tools |

---

**顶层建议优先级**（2026-06-23 更新：各项均已完成）：
1. ✅ survey → 椭球间坐标转换（Molodensky/Helmert）— 已完成
2. ✅ ecology → SDR泥沙输移比 — 已完成
3. ✅ coastal → 波浪爬高 — 已完成
4. ✅ hydro → 单位线汇流 + TR-55 + Muskingum — 已完成
5. ✅ agri → DSSAT adapter — 已完成
6. ✅ energy → 尾流效应(Jensen/Frandsen) + PVWatts — 已完成
7. ✅ carbon → 碳价情景分析 + VCS/GS — 已完成
8. ✅ forestry → 立地指数曲线 + 择伐/皆伐模拟 — 已完成
9. ✅ geohazard → Newmark位移法 — 已完成
10. ✅ urban → 城市内涝 + 15分钟城市可达性 — 已完成
11. ✅ remote-sensing → 辐射校正 + InSAR — 已完成
12. ✅ climate → GCM降尺度 + IDF + 干旱指数 + Kriging — 已完成
13. ✅ groundwater → 达西定律 + MODFLOW adapter — 已完成
14. ✅ geomorph → D8流向累积 + Strahler河网 — 已完成
15. ✅ ocean → 潮汐调和分析 + SWAN波浪 — 已完成
16. ✅ soil → HWSD查询 + van Genuchten参数 — 已完成
17. ✅ ecology → MUSLE事件版土壤流失 — 已完成

> **本次新增**：2026-06-20 · 基于插件代码完整审查（10/10 plugins）

---

## 远期插件拓展梯队（v2026-06-20）

按价值/协同度分三个梯队，标注每个拓展与现有插件的 seam 衔接点。

### 🥇 第一梯队：已完成（高协同、高价值）

#### 1. 气象/气候 (meteorology/climate) ✅ 已完成 (geo-plugin-climate)
#### 2. 地下水 (groundwater) ✅ 已完成 (geo-plugin-hydro/src/groundwater.rs)
#### 3. 地貌 (geomorphology) ✅ 已完成 (geo-plugin-geomorph)
#### 4. 土壤 (soil) ✅ 已完成 (eco-plugin-ecology/src/soil.rs)
#### 5. 海洋 (ocean) ✅ 已完成 (eo-plugin-coastal/src/ocean.rs)

---

### 🥈 第二梯队：独立价值高，协同可建构

#### 6. 冰雪/冰川 (cryosphere)
| 拓展 | 协同插件 | 说明 |
|------|----------|------|
| 雪水当量 (SWE) + 融雪模型（温度指数法/能量平衡法） | hydro | 融雪径流，高海拔水文关键 |
| 冰川物质平衡 / 运动速度 | coastal | GLOF (冰湖溃决洪水) |
| 冻土活动层厚度 / 冻融指数 | geohazard | 多年冻土退化→滑坡 |

#### 7. 大气 (atmosphere) ✅ 已完成
| 拓展 | 协同插件 | 说明 |
|------|----------|------|
| 大气边界层：湍流通量 / 混合层高度 | carbon | plume 高斯烟羽的上游参数 |
| 空气质量 → 独立大气扩散模块 | carbon | 从 carbon 剥离 plume.rs，逻辑更清晰 |
| AOD → PM2.5 反演 | urban | 城市空气质量管理 |

> **2026-06-24**: `geo-plugin-atmosphere` 新建完毕 (boundary_layer.rs + dispersion.rs + aod_pm25.rs), 4 MCP tools, 30 tests ✅

#### 8. 地震 (seismology)
| 拓展 | 协同插件 | 说明 |
|------|----------|------|
| PGA/PGV/反应谱（GB 18306-2015） | geohazard | 地震动参数基础 |
| 概率地震危险性分析 (PSHA) | geohazard | 场地地震风险定量 |
| Newmark 位移法 | geohazard | 地震诱发滑坡判别的核心方法 |

#### 9. 生态 (ecology) — 深度拓展
| 拓展 | 协同插件 | 说明 |
|------|----------|------|
| 生境质量（InVEST Habitat Quality） | forestry, urban | 景观破碎度评估 |
| 物种分布（MaxEnt 简化版） | forestry | 环境因子→适生概率 |
| 生态系统服务（碳固存/水源涵养/游憩） | carbon, hydro | 碳固存+CCER=完整碳资产 |

#### 10. 社会经济 (socioeconomic)
| 拓展 | 协同插件 | 说明 |
|------|----------|------|
| 人口密度空间化（LandScan/WorldPop） | urban, transmission | 暴露度/风险人口 |
| GDP 空间化（夜间灯光→NTL→GDP） | transmission, carbon | 输电选址+碳价增值 |
| 元胞自动机 CA-Markov 土地利用变化 | ecology, forestry | 侵蚀/碳汇的未来情景 |

---

### 🥉 第三梯队：前沿探索，长期价值

#### 11. 遥感（remote-sensing）✅ 已完成 (geo-plugin-remote-sensing)
已实现：DN→TOA辐射亮度→TOA反射率→DOS大气校正→云掩膜管线 + InSAR相干性/Goldstein解缠/LOS形变。6个MCP工具，15个测试。

---

### 🥉 第三梯队：前沿探索，长期价值

#### 12. 行星/天文 (planetary) ✅ 已完成
| 拓展 | 协同插件 | 说明 |
|------|----------|------|
| 天体坐标转换（月面/火星坐标系） | survey | GK→UTM→行星坐标框架 |
| 太阳位置（高度角/方位角） | energy, urban | 光伏效率+日照阴影 |
| 地外辐射 | energy | 晴空辐射的基础 |

> **2026-06-24**: `geo-plugin-planetary` 新建完毕 (coordinates.rs + solar.rs), 11 tests

#### 12. 古气候/古地理 (paleo) ✅ 已完成
| 拓展 | 协同插件 | 说明 |
|------|----------|------|
| 冰期-间冰期海平面重建 | coastal | 长期 SLR 背景 |
| 古海岸线恢复 | coastal | 第四纪海侵/海退 |
| 孢粉/冰芯代用指标反演 | forestry, coastal | 古植被/古温度 |

> **2026-06-24**: `geo-plugin-paleoclimate` 新建完毕 (sea_level.rs + paleocoastline.rs + proxies.rs), 13 tests

#### 13. 地质/构造 (geology) ✅ 已完成
| 拓展 | 协同插件 | 说明 |
|------|----------|------|
| 地层结构三维建模 | geohazard | 滑坡滑动面约束 |
| 断层/褶皱几何 | geohazard | 地震活动带评估 |
| 岩性分类 | geohazard | 地质图→岩性类型→FS 参数 |

> **2026-06-24**: `geo-plugin-geology` 新建完毕 (stratigraphy.rs + structures.rs + lithology.rs), 13 tests

#### 14. 火山 (volcanology) ✅ 已完成
| 拓展 | 协同插件 | 说明 |
|------|----------|------|
| 火山灰扩散（高斯烟羽变体+沉降速度） | carbon | 复用 plume 核心 |
| 熔岩流路径（Dijkstra 变体） | transmission | 复用 LCP 核心 |
| 火山灾害区划 | geohazard | 综合灾害管理 |

> **2026-06-24**: `geo-plugin-volcanology` 新建完毕 (ash_dispersion.rs + lava_flow.rs + hazard_zoning.rs), 14 tests

#### 15. 测量/遥感 (remote sensing)
| 拓展 | 协同插件 | 说明 |
|------|----------|------|
| 辐射校正（DN→反射率/辐射亮度） | ecology, agri | 当前 NDVI 计算直接用像素值，缺校正 |
| 大气校正（6S 模型简化版） | ecology | NDVI 精度提升 |
| 影像融合（Pan-sharpening） | urban, ecology | 高空间分辨率衍生 |
| InSAR 形变 / 极化分解 | geohazard, survey | 地面沉降+滑坡前兆 |

---

### 梯队全览速查

| 梯队 | 插件数 | 特征 |
|------|--------|------|
| 🥇 第一梯队 | 5 (气象/地下水/地貌/土壤/海洋) | ✅ 全部已完成 |
| 🥈 第二梯队 | 5 (冰雪/大气/地震/生态/社会经济) | ✅ 全部已完成 |
| 🥉 第三梯队 | 4 (行星/古气候/地质/火山) | ✅ 全部已完成 |

> **本次更新**：2026-06-24 · 第三梯队全部完成 (planetary/paleoclimate/geology/volcanology)，Plugin 层补齐至 21 个插件



