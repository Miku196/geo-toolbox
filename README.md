# geo-toolbox

**Rust 地理空间工具链** — AI Agent 与地理空间重工具之间的高性能胶水层。

将 PostGIS、GEE、QGIS、GDAL 等重型 GIS 工具串联成一条自动化管线：
数据采集 → 入库存储 → 遥感分析 → 碳核算 → 成果输出。

采用 **Core → Plugin → Adapter 三层架构**：Rust 负责性能敏感路径（批写、格式转换、消息分发、碳核算），
遥感计算和空间分析仍委托 Python 生态（GEE SDK、PyQGIS、GDAL CLI、brightway2）。

[![Rust](https://img.shields.io/badge/rust-1.80+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-200%20pass-green.svg)]()
[![NPM](https://img.shields.io/badge/npm-geo--wasm-red)](https://www.npmjs.com/package/geo-wasm)

---

## 目录

- [架构](#架构)
- [编译指南](#编译指南)
- [快速开始](#快速开始)
- [三层架构详解](#三层架构详解)
- [🔌 插件配置 (rules.toml)](#插件配置-rulestoml)
- [🌐 浏览器端 (WASM)](#浏览器端-wasm)
- [📦 NPM 包](#npm-包)
- [CLI 使用手册](#cli-使用手册)
- [🤖 MCP 集成（AI Agent 调用）](#mcp-集成ai-agent-调用)
- [📚 库调用（Rust）](#库调用rust)
- [部署](#部署)
- [开发](#开发)
- [示例](#示例)

---

## 架构

```
┌─────────────────────────────────────────────────────────────┐
│                        geo-toolbox                          │
├─────────────────────────────────────────────────────────────┤
│  Layer 1: Core（11 crates）— 纯 Rust，零外部依赖             │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  几何/CRS · 栅格运算 · 矢量运算 · 空间索引 · 统计     │  │
│  │  IO 解析 · 报告模板 · 碳核算公式 · 云原生格式 · OGC   │  │
│  └───────────────────────────────────────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│  Layer 2: Plugins（7 crates）— 专业领域，配置驱动            │
│  ┌────────┬────────┬────────┬────────┬────────┬────────┐  │
│  │ 碳核算 │生态修复│ 测绘   │城乡规划│ 水文   │地质灾害│  │
│  └────────┴────────┴────────┴────────┴────────┴────────┘  │
│                        │ 农业   │                          │
│                        └────────┘                          │
├─────────────────────────────────────────────────────────────┤
│  Layer 3: Adapters（7 crates）— 外部生态系统桥接             │
│  ┌──────┬──────┬──────┬──────┬──────┬──────┬────────────┐ │
│  │PostGIS│ GEE │ QGIS │ CAD  │ CLI  │ MCP  │ IoT        │ │
│  └──────┴──────┴──────┴──────┴──────┴──────┴────────────┘ │
└─────────────────────────────────────────────────────────────┘

依赖方向（严格单向）：Adapter → Plugin → Core
```

核心设计原则：依赖方向严格单向、WASM 数据不出网、Rust 做胶水 Python 做重活、每 crate 独立可测、Feature flags 控制依赖。

---

## 编译指南

### 环境要求

| 组件 | 最低版本 | 必需 | 说明 |
|------|:------:|:----:|------|
| Rust | 1.80+ | ✅ | [rustup.rs](https://rustup.rs) 安装 |
| wasm-pack | 0.13+ | ⚠ WASM 需要 | `cargo install wasm-pack` |
| wasm32 target | — | ⚠ WASM 需要 | `rustup target add wasm32-unknown-unknown` |
| PostgreSQL | 15+ | ⚠ 数据库需要 | `brew install postgresql@16` / `apt install postgresql-16` |
| GDAL | 3.8+ | ⚠ GDAL 需要 | `brew install gdal` / `apt install gdal-bin` |
| QGIS | 3.34+ | ⚠ QGIS 需要 | [qgis.org/download](https://qgis.org/download/) |
| NATS | 2.10+ | ⚠ GEE 需要 | 或用文件队列回退 |

### 编译模式

```bash
# ── 全功能编译（需安装 GDAL/QGIS C 库）──
cargo build --release

# ── 轻量编译：仅 CLI + 碳核算 + PostGIS，无 GIS 重依赖 ──
cargo build --release --no-default-features --features minimal

# ── 按需组合 ──
cargo build --release --no-default-features --features gee,qgis    # 仅 GEE + QGIS
cargo build --release --no-default-features --features gdal,gee    # 仅 GDAL + GEE

# ── WASM 浏览器编译 ──
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
wasm-pack build --target web crates/geo-wasm --out-dir ../../pkg --out-name geo_wasm
```

### CLI 运行

编译产物在 `target/release/geo-toolbox`（Windows: `.exe`），可直接运行：

```bash
./target/release/geo-toolbox crs list
./target/release/geo-toolbox plugins list
```

或用 `cargo run` 开发模式：

```bash
cargo run -- crs transform --from 4326 --to 3857 104.06 30.57
```

---

## 快速开始

```bash
git clone https://github.com/Miku196/geo-toolbox.git
cd geo-toolbox

# 轻量编译（无需 GIS 依赖）
cargo build --release --no-default-features --features minimal

# 列出坐标系
cargo run -- crs list

# 坐标变换（纯 Rust，零 C 依赖）
cargo run -- crs transform --from 4326 --to 3857 104.06 30.57
cargo run -- crs transform --from 4326 --to 9000 116.40 39.90   # WGS84 → GCJ-02

# 列出已安装插件
cargo run -- plugins list

# 运行全部测试
cargo test --workspace
```

---

## 三层架构详解

### Layer 1: Core — 纯 Rust 核心引擎（11 crates）

所有核心计算均纯 Rust 实现，**零外部系统依赖**。

| Crate | 功能 | 关键类型/函数 |
|-------|------|-------------|
| `geo-core` | 几何基类、CRS、错误 | `BBox`, `CrsRegistry`, `GeoError`, `SpatialRow`, `builtin::wgs84_to_gcj02()` |
| `geo-raster` | 栅格运算、NDVI | `RasterBand`, `compute_ndvi()`, `band_add/sub/div` |
| `geo-vector` | 矢量运算 | `buffer()`, `intersect()`, `union_all()`, `centroid()` |
| `geo-tile` | 矢量/栅格瓦片 | `latlon_to_tile()`, `MvtEncoder`, `PmtilesReader/Writer` |
| `geo-temporal` | 时空序列分析 | `mann_kendall()`, `linear_trend()`, `RasterTimeSeries` |
| `geo-index` | 空间索引 | `encode()`, `decode()`, `neighbors()`, `bbox_to_geohashes()` |
| `geo-stats` | 空间统计 | `zonal_stats()`, `ZonalResult` |
| `geo-io` | 数据 IO | `parse_feature_collection()`, `extract_bbox()`, NMEA/CamoFox 解析 |
| `geo-carbon-math` | 碳核算公式 | `CarbonEngine`, `EmissionFactor`, `GeoFeature`, `CarbonReport` |
| `geo-report` | 报告模板 | `ReportEngine`, `ReportGenerator`, Tera 过滤器 |
| `geo-parquet` | GeoParquet | `GeoParquetReader`, `GeoParquetWriter`, `SpatialFilter` |
| `geo-ogc` | OGC 服务 | `WmsService`, `WfsService`, `WpsService` |
| `geo-registry` | 插件注册 | `PluginRegistry`, `ToolDef`, `generate_mcp_tools()` |

### Layer 2: Plugins — 专业领域插件（7 crates）

每个插件 = `rules.toml`（业务参数）+ 报告模板 + 组装 Core 调用的薄层。
**插件间禁止互相依赖**，如需共享功能则下沉到 Core。

| 插件 | 核心计算 | 输入 | 输出 |
|------|---------|------|------|
| `geo-plugin-carbon` | 排放 = 面积 × 因子 | GeoJSON + 碳密度 CSV | 碳核算报告 |
| `geo-plugin-ecology` | NDVI 变化检测 + 碳汇 | 多期遥感 + AOI | 生态修复评估报告 |
| `geo-plugin-survey` | 控制网平差、土方量 | 测量原始数据 | 测绘成果表 |
| `geo-plugin-urban` | 用地分类、容积率、密度 | 规划图 | 规划指标表 |
| `geo-plugin-hydro` | 流域提取、汇流、淹没 | DEM + 降雨 | 水文报告 |
| `geo-plugin-geohazard` | 滑坡敏感性 | 地质图 + DEM | 风险等级图 |
| `geo-plugin-agri` | 作物估产、土壤评级 | 农田 + 遥感 | 产量报告 |
| `geo-plugin-energy` | 光伏/风电选址 | DEM + 辐射/风速 | 适宜性等级 |

### Layer 3: Adapters — 外部适配器（9 crates）

| 适配器 | 外部系统 | 通信方式 |
|--------|---------|---------|
| `geo-adapter-postgis` | PostgreSQL + PostGIS | sqlx 连接池 |
| `geo-adapter-gee` | Google Earth Engine | NATS → Python worker |
| `geo-adapter-qgis` | QGIS | REST / qgis_process |
| `geo-adapter-cad` | CAD 格式 | DXF/DWG 读写 |
| `geo-adapter-cli` | GDAL/DVC/shell | 子进程 |
| `geo-adapter-mcp` | AI Agent | JSON-RPC stdio |
| `geo-adapter-iot` | 传感器 | MQTT/NATS |
| `geo-adapter-duckdb` | SQLite 嵌入式 | rusqlite 内存/文件 |
| `geo-adapter-stac` | STAC API | HTTP (reqwest) |

---

## 插件配置 (rules.toml)

每个插件通过 `rules.toml` 声明业务参数，无需改代码即可调整计算逻辑。

### 配置文件位置

```
plugins/geo-plugin-ecology/rules.toml     # 生态修复
plugins/geo-plugin-carbon/rules.toml      # 碳核算
plugins/geo-plugin-urban/rules.toml       # 城乡规划
# ... 每个插件一个
```

### 完整示例：geo-plugin-ecology

```toml
# plugins/geo-plugin-ecology/rules.toml

[plugin]
name = "ecology"
version = "0.1.0"
description = "生态修复评估插件 — NDVI 变化检测、碳汇计算"

# ── NDVI 阈值（可根据项目区调整）──
[ndvi]
healthy_min = 0.5              # ≥ 此值 = 健康植被
degraded_max = 0.2             # ≤ 此值 = 退化植被
improvement_threshold = 0.1    # NDVI 差值 > 此值 = 显著改善
degradation_threshold = -0.1   # NDVI 差值 < 此值 = 显著退化

# ── 碳密度参数（tCO₂e/ha/yr）──
# 正值 = 排放源，负值 = 碳汇
[carbon]
source = "IPCC_2019"
forest = -5.0                  # 森林年碳汇
grassland = -1.2               # 草地年碳汇
wetland = -8.5                 # 湿地年碳汇
cropland = 0.5                 # 农田排放
built_up = 2.0                 # 建设用地排放
bare = 0.0                     # 裸地/矿区

# ── 报告模板 ──
[report]
template = "restoration-report.md.tera"
format = "markdown"            # markdown | html
```

### 完整示例：geo-plugin-urban

```toml
[plugin]
name = "urban"
version = "0.1.0"
description = "城乡规划插件 — 用地分类、容积率、建筑密度"

[density]
far_max = 3.5                  # 容积率上限
building_density_max = 0.4     # 建筑密度上限
green_ratio_min = 0.3          # 绿地率下限
```

### 完整示例：geo-plugin-hydro

```toml
[plugin]
name = "hydro"
version = "0.1.0"
description = "水文分析插件"

[flood]
return_period_years = 100      # 洪水重现期
safety_factor = 1.2            # 安全系数
```

### 代码中加载配置

```rust
use geo_plugin_ecology::{EcologyPlugin, EcologyConfig};

// 方式 1：从默认 rules.toml 加载（编译期嵌入）
let plugin = EcologyPlugin::new(EcologyConfig::default());

// 方式 2：从文件路径加载
let config: EcologyConfig = toml::from_str(
    &std::fs::read_to_string("rules.toml")?
)?;
let plugin = EcologyPlugin::new(config);
```

### 报告模板

插件模板使用 [Tera](https://tera.netlify.app/) 语法，存放在 `plugins/<name>/templates/`。

公共组件（表格宏、布局）在 `core/geo-report/templates/`，通过 `{% include %}` 引用。

```markdown
{# plugins/geo-plugin-ecology/templates/restoration-report.md.tera #}

# {{ aoi_name }} 生态修复评估报告

**评估年份**：{{ assessment_year }}
**基准年份**：{{ baseline_year }}

## NDVI 变化

| 指标 | 基准年 | 评估年 | 变化 |
|------|--------|--------|------|
| 平均 NDVI | {{ baseline_ndvi.mean_ndvi | round(3) }} | {{ assessment_ndvi.mean_ndvi | round(3) }} | {{ ndvi_change.mean_diff | round(3) }} |

## 碳汇评估

| 土地覆盖类型 | 面积 (ha) | 年碳汇 (tCO₂/yr) |
|-------------|----------|-----------------|
{% for c in carbon.classes %}
| {{ c.landcover_class }} | {{ c.area_ha | round(1) }} | {{ c.emission_tco2e | round(2) }} |
{% endfor %}
| **合计** | **{{ carbon.total_area_ha | round(1) }}** | **{{ carbon.total_emission_tco2e | round(2) }}** |

## 评估结论

{{ conclusion.summary }}
```

---

## 🔌 插件开发指南

### 插件最小结构

每个插件是一个 Rust crate，包含三个核心文件：

```
plugins/geo-plugin-mine/
├── Cargo.toml          # 依赖声明（只能依赖 Core 层，禁止依赖 Adapter 或其他 Plugin）
├── rules.toml          # 业务参数（碳密度、阈值、报告模板路径等）
├── templates/          # 领域专属报告模板（可选）
│   └── mine-report.md.tera
└── src/
    ├── lib.rs          # 入口：pub mod + pub use
    ├── config.rs       # rules.toml 反序列化结构体
    └── mine.rs         # 核心编排逻辑（组装 Core 调用）
```

### 第一步：定义配置 (config.rs)

```rust
// plugins/geo-plugin-mine/src/config.rs
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct MineConfig {
    pub plugin: PluginMeta,
    #[serde(default)]
    pub thresholds: ThresholdConfig,
    #[serde(default)]
    pub carbon: CarbonConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThresholdConfig {
    /// 坡度阈值（度），超过此值标记为高风险
    #[serde(default = "default_slope")]
    pub slope_max_deg: f64,
    /// 植被覆盖度下限
    #[serde(default = "default_veg_cover")]
    pub veg_cover_min: f64,
}

fn default_slope() -> f64 { 25.0 }
fn default_veg_cover() -> f64 { 0.3 }

#[derive(Debug, Clone, Deserialize)]
pub struct CarbonConfig {
    #[serde(default = "default_source")]
    pub source: String,
    pub forest: f64,
    pub grassland: f64,
    pub bare: f64,
}

fn default_source() -> String { "IPCC_2019".into() }

impl Default for MineConfig {
    fn default() -> Self {
        toml::from_str(include_str!("../rules.toml")).unwrap()
    }
}
```

### 第二步：编写业务逻辑 (mine.rs)

```rust
// plugins/geo-plugin-mine/src/mine.rs
// 只能 import Core 层 crate，不能 import 其他 Plugin 或 Adapter

use geo_core::errors::GeoResult;
use geo_core::types::BBox;
use geo_raster::ndvi::{compute_ndvi, NdviResult};
use geo_raster::RasterBand;
use geo_carbon_math::{CarbonEngine, EmissionFactor, GeoFeature};
use crate::config::MineConfig;

pub struct MinePlugin {
    config: MineConfig,
}

impl MinePlugin {
    pub fn new(config: MineConfig) -> Self { Self { config } }

    pub fn from_file(path: &std::path::Path) -> GeoResult<Self> {
        let s = std::fs::read_to_string(path)?;
        let config: MineConfig = toml::from_str(&s)
            .map_err(|e| geo_core::GeoError::Validation(e.to_string()))?;
        Ok(Self { config })
    }

    /// 矿山风险评估（组装 Core 调用）
    pub fn assess(
        &self,
        aoi: &str,                     // GeoJSON FeatureCollection
        dem_red: &RasterBand,          // DEM 红波段用于 NDVI
        dem_nir: &RasterBand,
        year: u16,
    ) -> GeoResult<MineAssessment> {
        // 1. 解析 AOI
        let bbox = geo_io::extract_bbox(aoi)?;

        // 2. 计算 NDVI（调用 geo-raster）
        let ndvi = compute_ndvi(dem_red, dem_nir)?;

        // 3. 植被覆盖度评估（按配置阈值判断）
        let healthy = ndvi.mean_ndvi.unwrap_or(0.0) >= self.config.thresholds.veg_cover_min;

        // 4. 碳核算（调用 geo-carbon-math）
        let engine = CarbonEngine::new();
        let factors = vec![
            EmissionFactor::new("forest", self.config.carbon.forest, &self.config.carbon.source),
            EmissionFactor::new("grassland", self.config.carbon.grassland, &self.config.carbon.source),
            EmissionFactor::new("bare", self.config.carbon.bare, &self.config.carbon.source),
        ];
        let features = geo_io::geojson::parse_feature_collection(aoi)?.0
            .into_iter()
            .filter_map(|f| GeoFeature::from_feature_json(&serde_json::to_string(&f).ok()?).ok())
            .collect::<Vec<_>>();
        let carbon = engine.calculate(&features, &factors, year)
            .map_err(|e| geo_core::GeoError::Validation(e))?;

        Ok(MineAssessment {
            aoi_name: "矿区".into(),
            bbox,
            ndvi_mean: ndvi.mean_ndvi,
            vegetation_healthy: healthy,
            carbon_report: carbon,
            risk_level: if !healthy { "高风险" } else { "低风险" }.into(),
        })
    }
}

#[derive(Debug, serde::Serialize)]
pub struct MineAssessment {
    pub aoi_name: String,
    pub bbox: BBox,
    pub ndvi_mean: Option<f64>,
    pub vegetation_healthy: bool,
    pub carbon_report: geo_carbon_math::CarbonReport,
    pub risk_level: String,
}
```

### 第三步：配置文件 (rules.toml)

```toml
# plugins/geo-plugin-mine/rules.toml

[plugin]
name = "mine"
version = "0.1.0"
description = "矿山生态风险评估插件"

[thresholds]
slope_max_deg = 25.0
veg_cover_min = 0.3

[carbon]
source = "IPCC_2019"
forest = -5.0
grassland = -1.2
bare = 0.0
```

### 第四步：仓库入口 (lib.rs)

```rust
// plugins/geo-plugin-mine/src/lib.rs
pub mod config;
pub mod mine;

pub use config::MineConfig;
pub use mine::{MinePlugin, MineAssessment};
```

### 第五步：注册到 workspace

```toml
# Cargo.toml (workspace root)
[workspace]
members = [
    # ... 其他成员 ...
    "plugins/geo-plugin-mine",   # ← 添加新插件
]
```

### 第六步：在 CLI/MCP 中使用

```rust
// crates/geo-cli/src/commands/mine.rs
use geo_plugin_mine::MinePlugin;

let plugin = MinePlugin::from_file(Path::new("plugins/geo-plugin-mine/rules.toml"))?;
let assessment = plugin.assess(aoi_geojson, &red, &nir, 2025)?;
println!("风险等级: {}", assessment.risk_level);
```

### 插件开发约束速查

| 约束 | 说明 |
|------|------|
| ✅ 可依赖 | `geo-core`, `geo-raster`, `geo-stats`, `geo-io`, `geo-carbon-math`, `geo-report`, `geo-vector`, `geo-index` |
| ❌ 禁止依赖 | 任何 `geo-adapter-*`（Adapter 层）、任何其他 `geo-plugin-*`（插件横向） |
| ✅ 可包含 | `rules.toml`（业务参数）、`templates/`（报告模板）、单元测试 + 集成测试 |
| ❌ 不应包含 | 数据库连接池、网络请求、子进程调用、文件系统写操作（这些都是 Adapter 的职责） |

---

## 🔗 外部适配器调用

外部适配器是 geo-toolbox 与 PostGIS、GEE、QGIS、CAD 等外部系统之间的桥梁。
适配器层可以依赖 Plugin 和 Core，是唯一允许持有数据库连接/网络连接/子进程的层。

### PostGIS 适配器

```rust
use geo_adapter_postgis::{PostgisStore, PostgisCarbonEngine, run_migrations};

// 连接数据库
let store = PostgisStore::connect("postgres://geo:geo@localhost/geo_test").await?;

// 运行迁移（建表）
run_migrations(store.pool()).await?;

// 查询（自动做 SQL 注入防护）
let rows = store.query_json("SELECT * FROM spatial_assets WHERE aoi_id = '...'").await?;

// 写入几何
use uuid::Uuid;
let aoi_id = Uuid::new_v4();
let wkb_bytes = vec![/* WKB 二进制 */];
store.insert_geometry(Some(aoi_id), "sentinel-2", &wkb_bytes, &serde_json::json!({"class": "forest"})).await?;

// ── PostGIS 碳核算引擎 ──
let pool = store.pool().clone(); // 或直接用 sqlx 建池
let engine = PostgisCarbonEngine::new(pool);

// 排放因子计算（单条 SQL 含空间聚合 + 因子查表 + 面积计算）
let results = engine.calculate_emission_factor(aoi_id, 2025, "IPCC_2019").await?;

// 注册排放因子
engine.register_factor(geo_adapter_postgis::FactorInput {
    source: "IPCC_2019".into(),
    category: "forest".into(),
    factor_value: -5.0,
    unit: "tCO2e/ha/yr".into(),
    valid_from_year: 2019,
    valid_to_year: None,
    region: Some("CN-51".into()),
}).await?;

// 从 CSV 批量导入
engine.import_factors_csv("emission-factors.csv").await?;
```

### QGIS 适配器

```rust
// 方式 A：qgis_process 子进程（推荐批处理）
use geo_adapter_qgis::process_runner::{BatchQgisRunner, QgisProcessConfig};

let runner = BatchQgisRunner::new(QgisProcessConfig::default());

// 重投影
runner.reproject("input.geojson", 3405, "equalarea.gpkg").await?;

// 缓冲区
runner.buffer("sites.gpkg", 2000.0, "sites_buffer.gpkg").await?;

// 流水线：重投影 → 缓冲区 → 相交
use geo_adapter_qgis::process_runner::QgisTool;
runner.run_pipeline(&[
    QgisTool {
        algorithm: "native:reprojectlayer".into(),
        params: vec![
            ("INPUT".into(), "".into()),
            ("TARGET_CRS".into(), "EPSG:3405".into()),
            ("OUTPUT".into(), "step0.gpkg".into()),
        ],
    },
    QgisTool {
        algorithm: "native:buffer".into(),
        params: vec![
            ("INPUT".into(), "".into()),
            ("DISTANCE".into(), "500".into()),
            ("OUTPUT".into(), "step1.gpkg".into()),
        ],
    },
], Path::new("input.geojson")).await?;

// 方式 B：PyQGIS REST 服务（推荐交互式）
use geo_adapter_qgis::grpc_client::{QgisClient, QgisInput};

let client = QgisClient::new("http://localhost:9100");
if client.health_check().await? {
    let output = client.buffer("sites.gpkg", 100.0, Some("sites_buffered")).await?;
    println!("输出: {output}");
}
```

### GEE 适配器（Google Earth Engine）

```rust
use geo_adapter_gee::GeeAdapter;

let adapter = GeeAdapter::new_default().await?;

// 提交土地覆盖分类任务
adapter.submit_classification(
    "projects/my-project/aoi/gpkg",
    2025,
    "COPERNICUS/S2_SR_HARMONIZED",
).await?;

// 查询任务状态
let status = adapter.job_status("task-uuid").await?;

// 导出到 GCS
adapter.export_to_gcs("task-uuid", "gs://my-bucket/lc_2025.tif").await?;
```

### CAD 适配器

```rust
use geo_adapter_cad::{DxfExporter, ExcelDashboard, GeoJsonExporter};

// DXF 导出（需先有数据行）
let rows: Vec<serde_json::Value> = /* 从 PostGIS 或其他来源读取 */;
// ... 具体导出见 geo-adapter-cad 文档
```

### CLI 适配器（GDAL / DVC 子进程）

```rust
use geo_adapter_cli::{gdal_translate_cog, dvc_snapshot};

// GDAL COG 转换
geo_adapter_cli::raster::to_cog("input.tif", "output.cog.tif", "DEFLATE")?;

// DVC 版本快照
let hash = geo_adapter_cli::gcs_bridge::dvc_hash("data/carbon_factors.csv")?;
```

### IoT 适配器（MQTT 传感器接入）

```rust
use geo_adapter_iot::MqttAdapter;

let adapter = MqttAdapter::connect("mqtt://localhost:1883", "geo-sensors").await?;
adapter.subscribe("gps/+/location").await?;

while let Some(msg) = adapter.next_message().await {
    println!("传感器 {}: {:?}", msg.topic, msg.payload);
    // 解析 → 验证坐标 → 写入 PostGIS
}
```

### 适配器使用约束速查

| 约束 | 说明 |
|------|------|
| ✅ 可依赖 | Core 层所有 crate、Plugin 层所有 crate、其他 Adapter |
| ✅ 可持有 | 数据库连接池 (`PgPool`)、HTTP Client、子进程句柄、MQTT 连接 |
| ❌ 禁止 | 在 Core/Plugin 中使用 Adapter（依赖方向不可逆） |
| ✅ 实现 trait | `ExternalAdapter`（提供 `health_check`、`push`/`pull`/`execute`） |

---

## 浏览器端 (WASM)

### 编译

```bash
# 安装工具链（仅首次）
rustup target add wasm32-unknown-unknown
cargo install wasm-pack

# 编译（产物在 pkg/）
wasm-pack build --target web crates/geo-wasm --out-dir ../../pkg --out-name geo_wasm
```

### 启动 Demo

```bash
cd geo-toolbox
python -m http.server 8899
# 浏览器打开 http://127.0.0.1:8899/demo.html
```

Demo 页面功能：CRS 变换、NMEA GPS 解析、碳核算、空间运算、IndexedDB 存储。

---

## NPM 包

### 安装

```bash
npm install geo-wasm
```

### API 参考

| 类/函数 | 说明 | 参数 | 返回值 |
|---------|------|------|--------|
| `CrsEngine` | CRS 坐标变换 | — | — |
| `.transform(from, to, x, y)` | 坐标变换 | EPSG 代码 + 坐标 | `[x, y]` |
| `.list()` | 列出内置坐标系 | — | `[{epsg, name, ...}]` |
| `CarbonEngine` | 碳核算 | — | — |
| `.calculate(geojson, csv, year)` | 计算碳排放 | FeatureCollection, CSV 文本, 年份 | `CarbonReport` |
| `GeoStore` | IndexedDB 存储 | — | — |
| `.init()` | 初始化数据库 | — | `void` |
| `.putFeature(id, feature)` | 存储要素 | string, GeoJSON Feature | `void` |
| `.getAllFeatures()` | 获取全部要素 | — | `Feature[]` |
| `parseNmea(nmea)` | NMEA 解析 | NMEA 语句字符串 | `GgaFix \| RmcFix` |
| `validateCoord(lon, lat)` | 坐标校验 | 经纬度 | `boolean` |
| `computeArea(geojson)` | 面积计算 | GeoJSON 几何 | `{m2, ha}` |
| `computeBbox(geojson)` | 边界框 | GeoJSON 几何 | `[minX,minY,maxX,maxY]` |
| `exportExcel(data, sheet)` | 导出 Excel | `[string[][]]`, sheet 名 | `Uint8Array` |
| `exportGeoJson(features)` | 导出 GeoJSON | Feature 数组 | FeatureCollection JSON |
| `exportCarbonReport(report)` | 碳核算报告 | CarbonReport | Markdown 字符串 |

### 完整示例

```typescript
import { CrsEngine, CarbonEngine, GeoStore, parseNmea } from 'geo-wasm';

// ── CRS 变换 ──
const crs = new CrsEngine();
const [x, y] = await crs.transform(4326, 3857, 104.06, 30.57);
console.log(`Web Mercator: (${x}, ${y})`);

// ── 碳核算 ──
const engine = new CarbonEngine();
const geojson = JSON.stringify({
  type: "FeatureCollection",
  features: [{
    type: "Feature",
    properties: { class: "forest", subcategory: "evergreen_broadleaf" },
    geometry: { type: "Polygon", coordinates: [[[104,30.5],[104.1,30.5],[104.1,30.6],[104,30.6],[104,30.5]]] }
  }]
});

const csv = `source,category,subcategory,factor_value,unit,region
IPCC_2019,forest,evergreen_broadleaf,-380.0,tCO2e/ha,CN-51`;

const report = await engine.calculate(geojson, csv, 2025);
console.log(`总碳汇: ${report.total_emission_tco2e} tCO₂`);

// ── GPS 解析 ──
const fix = parseNmea('$GPGGA,123519,4807.038,N,01131.000,E,1,08,1.2,545.4,M,,,*47');
console.log(`位置: (${fix.lat}, ${fix.lng}), 卫星数: ${fix.satellites}`);

// ── IndexedDB 存储 ──
const store = new GeoStore();
await store.init();
await store.putFeature('aoi-chengdu', {
  type: 'Feature',
  properties: { name: '成都高新区' },
  geometry: { type: 'Polygon', coordinates: [[[104,30.5],[104.1,30.5],[104.1,30.6],[104,30.6],[104,30.5]]] }
});
const all = await store.getAllFeatures();
console.log(`已存储 ${all.length} 个要素`);
```

---

## CLI 使用手册

### CRS 坐标

```bash
geo-toolbox crs list
geo-toolbox crs transform --from 4326 --to 3857 104.06 30.57
geo-toolbox crs transform --from 4326 --to 9000 116.40 39.90   # WGS84→火星
echo "104.06,30.57" | geo-toolbox crs transform --from 4326 --to 3857 --batch
```

### 数据入库

```bash
geo-toolbox ingest camofox data.json          # CamoFox JSON → PostGIS
geo-toolbox ingest nmea gps_log.txt           # NMEA GPS 解析
geo-toolbox store migrate                     # 数据库迁移
geo-toolbox store write sites chengdu.geojson # GeoJSON 写入
geo-toolbox store read "SELECT ST_AsGeoJSON(geom) FROM sites"
```

### 遥感处理

```bash
# GEE 土地覆盖分类
geo-toolbox process gee classify \
    --aoi s3://geo-data/aoi.gpkg --year 2025 \
    --output-gcs gs://gee-exports/lc.tif

# GDAL COG 转换
geo-toolbox process gdal cog input.tif output.cog.tif
geo-toolbox process gdal ogr2ogr in.geojson out.gpkg --overwrite

# QGIS 缓冲区
geo-toolbox process qgis batch \
    --algorithm native:buffer --input sites.gpkg \
    --output buf.gpkg --extra '[["DISTANCE","2000"]]'
```

### 碳核算

```bash
geo-toolbox carbon emission-factor register factors.csv
geo-toolbox carbon emission-factor calculate --aoi <uuid> --year 2025 --source IPCC_2019
```

### 成果导出

```bash
geo-toolbox output excel "SELECT ..." --output report.xlsx
geo-toolbox output dxf "SELECT ..." --output cad.dxf --to-epsg 32649
geo-toolbox output geojson --from-file in.geojson --output out.geojson --to-epsg 3857
geo-toolbox output report --aoi <uuid> --year 2025 --name "成都高新区" --output carbon.md
```

### 插件管理

```bash
geo-toolbox plugins list                     # 列出全部
geo-toolbox plugins list --category carbon   # 按类别
geo-toolbox plugins show crs                 # 查看详情
```

---

## MCP 集成（AI Agent 调用）

geo-toolbox 内置 MCP Server，支持 AI Agent（如 Claude、Pi Agent）通过 JSON-RPC 直接调用。

### 启动

```bash
geo-toolbox mcp-serve --port 9378
```

### 协议交互

MCP Server 通过 stdio 通信，AI Agent 只需读写 stdin/stdout。

#### 1. 握手

```json
// → 发送
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"pi-agent","version":"1.0"}}}

// ← 返回
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"geo-toolbox","version":"0.1.0"}}}
```

#### 2. 获取工具列表

```json
// → 发送
{"jsonrpc":"2.0","id":2,"method":"tools/list"}

// ← 返回（从 PluginRegistry 动态生成）
{"jsonrpc":"2.0","id":2,"result":{"tools":[
  {"name":"crs_list","description":"List all registered coordinate reference systems","inputSchema":{"type":"object","properties":{},"required":[]}},
  {"name":"crs_transform","description":"Transform coordinates between CRS","inputSchema":{"type":"object","properties":{"from_epsg":{"type":"integer"},"to_epsg":{"type":"integer"},"x":{"type":"number"},"y":{"type":"number"}},"required":["from_epsg","to_epsg","x","y"]}},
  {"name":"store_migrate","description":"Run PostGIS database migrations","inputSchema":{"type":"object","properties":{},"required":[]}},
  {"name":"store_query","description":"Execute a SQL query and return results as JSON","inputSchema":{"type":"object","properties":{"sql":{"type":"string"}},"required":["sql"]}},
  {"name":"ingest_camofox","description":"Parse a CamoFox JSON file and write to PostGIS","inputSchema":{"type":"object","properties":{"file":{"type":"string"}},"required":["file"]}},
  {"name":"ingest_nmea","description":"Parse an NMEA GPS log file and return fixes","inputSchema":{"type":"object","properties":{"file":{"type":"string"}},"required":["file"]}},
  {"name":"carbon_calculate","description":"Calculate carbon emissions using emission factor method","inputSchema":{"type":"object","properties":{"aoi_id":{"type":"string"},"year":{"type":"integer"},"source":{"type":"string","default":"IPCC_2019"}},"required":["aoi_id","year"]}},
  {"name":"carbon_import_factors","description":"Import emission factors from a CSV file","inputSchema":{"type":"object","properties":{"csv_path":{"type":"string"}},"required":["csv_path"]}},
  {"name":"dvc_snapshot","description":"Run DVC add + push on a file for version tracking","inputSchema":{"type":"object","properties":{"file":{"type":"string"}},"required":["file"]}},
  {"name":"dvc_hash","description":"Get the DVC MD5 hash of a tracked file","inputSchema":{"type":"object","properties":{"file":{"type":"string"}},"required":["file"]}}
]}}
```

#### 3. 调用工具

```json
// → 坐标变换
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"crs_transform","arguments":{"from_epsg":4326,"to_epsg":3857,"x":104.06,"y":30.57}}}

// ← 返回
{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"EPSG:4326 (104.06, 30.57) → EPSG:3857 (11583906.2148, 3577030.4672)"}]}}

// → 碳核算
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"carbon_calculate","arguments":{"aoi_id":"550e8400-e29b-41d4-a716-446655440000","year":2025}}}

// ← 返回
{"jsonrpc":"2.0","id":4,"result":{"content":[{"type":"text","text":"{\"aoi_id\":\"550e8400-...\",\"year\":2025,\"total_tco2e\":-125.3,\"results\":[...]}"}]}}
```

### Python Agent 调用示例

```python
import subprocess, json

proc = subprocess.Popen(
    ['./target/release/geo-toolbox', 'mcp-serve'],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True
)

def call(method, params=None):
    req = {"jsonrpc": "2.0", "id": 1, "method": method}
    if params: req["params"] = params
    proc.stdin.write(json.dumps(req) + '\n')
    proc.stdin.flush()
    return json.loads(proc.stdout.readline())

# 初始化
call("initialize", {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "py-agent", "version": "1.0"}})

# 坐标变换
result = call("tools/call", {"name": "crs_transform", "arguments": {"from_epsg": 4326, "to_epsg": 3857, "x": 104.06, "y": 30.57}})
print(result)
```

---

## 库调用（Rust）

在 Rust 项目中直接依赖 geo-toolbox crate。

### 添加依赖

```toml
[dependencies]
geo-core = { git = "https://github.com/Miku196/geo-toolbox" }
geo-carbon-math = { git = "https://github.com/Miku196/geo-toolbox" }
geo-raster = { git = "https://github.com/Miku196/geo-toolbox" }
# 按需添加
```

### CRS 坐标变换

```rust
use geo_core::crs::CrsRegistry;

let reg = CrsRegistry::new();
// 纯 Rust 变换（零 C 依赖）
let (x, y) = reg.transform_point(4326, 3857, 104.06, 30.57)?;
// 特殊坐标系
let (gx, gy) = reg.transform_point(4326, 9000, 116.40, 39.90)?;  // WGS84 → GCJ-02
```

### 碳核算

```rust
use geo_carbon_math::{CarbonEngine, EmissionFactor, GeoFeature};

let engine = CarbonEngine::new();
let factors = vec![
    EmissionFactor::new("forest", -5.0, "IPCC_2019"),
    EmissionFactor::new("grassland", -1.2, "IPCC_2019"),
];
let features = vec![
    GeoFeature::new("forest", r#"{"type":"Polygon","coordinates":[[[104,30.5],[104.1,30.5],[104.1,30.6],[104,30.6],[104,30.5]]]}"#)?,
];
let report = engine.calculate(&features, &factors, 2025)?;
println!("总碳汇: {:.1} tCO₂", report.total_emission_tco2e);
```

### NDVI 计算

```rust
use geo_raster::{RasterBand, ndvi::compute_ndvi};

let red = RasterBand::new("B4", 100, 100, vec![0.05; 10000], -999.0);
let nir = RasterBand::new("B8", 100, 100, vec![0.50; 10000], -999.0);
let result = compute_ndvi(&red, &nir)?;
println!("平均 NDVI: {:?}", result.mean_ndvi);
```

### GeoJSON 解析

```rust
use geo_io::geojson::parse_feature_collection;

let geojson = r#"{"type":"FeatureCollection","features":[...]}"#;
let (features, bbox) = parse_feature_collection(geojson)?;
println!("要素数: {}, bbox: {:?}", features.len(), bbox);
```

### 生态修复评估（完整管线）

```rust
use geo_plugin_ecology::{EcologyPlugin, EcologyConfig};
use geo_core::types::BBox;

let plugin = EcologyPlugin::new(EcologyConfig::default());
let assessment = plugin.assess_restoration(
    "XX矿山修复区",
    aoi_geojson,
    &red_2020, &nir_2020,  // 基准年波段
    &red_2025, &nir_2025,  // 评估年波段
    2020, 2025,
    BBox::new(104.0, 30.5, 104.1, 30.6),
)?;
println!("生态修复评级: {}", assessment.conclusion.grade);
println!("年碳汇: {:.1} tCO₂", assessment.conclusion.carbon_sink_tco2_per_yr);
```

---

## 部署

### PostgreSQL + PostGIS

```bash
# macOS
brew install postgresql@16 postgis
brew services start postgresql@16

# Ubuntu/Debian
sudo apt install postgresql-16 postgis
sudo systemctl start postgresql

# 创建数据库
sudo -u postgres createuser geo -P          # 密码: geo
sudo -u postgres createdb geo_test -O geo
sudo -u postgres psql geo_test -c "CREATE EXTENSION postgis;"
```

```bash
# 设置环境变量
export DATABASE_URL=postgres://geo:geo@localhost/geo_test

# 运行迁移
geo-toolbox store migrate
```

### GEE 消息队列

```bash
# NATS（可选）
export GEO_NATS_URL=nats://localhost:4222
nats-server -js &

# 或文件队列（默认，无需额外服务）
# 任务自动写入 ./queue/gee-tasks.jsonl
```

---

## 开发

### 项目结构

```
geo-toolbox/
├── core/                    # Layer 1: 纯 Rust（11 crates）
│   ├── geo-core/            # 几何基类、CRS、错误
│   ├── geo-carbon-math/     # IPCC 碳核算公式
│   ├── geo-raster/          # 栅格运算 + NDVI
│   ├── geo-vector/          # 矢量运算
│   ├── geo-tile/            # MVT/PMTiles 瓦片
│   ├── geo-temporal/        # 时空序列分析
│   ├── geo-index/           # GeoHash 空间索引
│   ├── geo-stats/           # 分区统计
│   ├── geo-io/              # GeoJSON/CSV/NMEA 解析
│   ├── geo-report/          # Tera 模板引擎
│   ├── geo-parquet/         # GeoParquet 读写
│   ├── geo-ogc/             # WMS/WFS/WPS
│   └── geo-registry/        # 插件注册中心
├── plugins/                 # Layer 2: 专业插件（7 crates）
│   ├── geo-plugin-energy/   # 新能源选址
│   └── geo-plugin-{carbon,ecology,survey,urban,hydro,geohazard,agri}/
├── adapters/                # Layer 3: 外部适配器（9 crates）
│   ├── geo-adapter-duckdb/  # SQLite 嵌入式
│   ├── geo-adapter-stac/    # STAC 数据发现
│   └── geo-adapter-{postgis,gee,qgis,cad,cli,mcp,iot}/
├── crates/                  # 入口（2 crates）
│   ├── geo-cli/             # CLI + MCP
│   └── geo-wasm/            # WASM + NPM
├── examples/                # 成都碳核算 + 中国风险评估
├── demo.html                # 浏览器 DEMO
├── Cargo.toml               # workspace
└── DEVPLAN.md               # 改造开发流程
```

### 测试

```bash
cargo test --workspace                         # 全部（198 tests）
cargo test -p geo-raster                       # 单个 crate
cargo test -p geo-plugin-ecology               # 插件集成测试
# 含数据库测试（需设置 DATABASE_URL）
DATABASE_URL=postgres://geo:geo@localhost/geo_test cargo test --workspace
```

### 代码质量

```bash
cargo clippy --workspace
cargo fmt --all -- --check
```

---

## 示例

- `examples/chengdu-carbon/` — 成都开发区碳收支评估完整案例
- `examples/china-risk-assessment/` — 中国洪水+地震风险评估管线
- `demo.html` — 浏览器 WASM 演示

---

## 参与贡献

[github.com/Miku196/geo-toolbox](https://github.com/Miku196/geo-toolbox)

```bash
git clone https://github.com/Miku196/geo-toolbox.git
cd geo-toolbox
git checkout -b feature/my-feature
cargo fmt --all -- --check && cargo clippy && cargo test
git commit -m "feat: add something"
git push origin feature/my-feature
```
