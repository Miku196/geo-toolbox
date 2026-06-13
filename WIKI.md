# geo-toolbox Wiki

> 从零到一：安装、开发、部署全流程指南。
> 最后更新：2026-06-13

---

## 目录

- [1. 环境搭建](#1-环境搭建)
  - [1.1 Rust 工具链](#11-rust-工具链)
  - [1.2 必需依赖](#12-必需依赖)
  - [1.3 可选依赖（按需安装）](#13-可选依赖按需安装)
  - [1.4 验证安装](#14-验证安装)
- [2. 克隆与首次编译](#2-克隆与首次编译)
- [3. 项目结构速览](#3-项目结构速览)
- [4. 核心功能使用](#4-核心功能使用)
  - [4.1 CRS 坐标变换](#41-crs-坐标变换)
  - [4.2 碳核算](#42-碳核算)
  - [4.3 NDVI 计算](#43-ndvi-计算)
  - [4.4 GeoJSON IO](#44-geojson-io)
- [5. 插件开发](#5-插件开发)
  - [5.1 创建插件骨架](#51-创建插件骨架)
  - [5.2 编写配置](#52-编写配置)
  - [5.3 实现业务逻辑](#53-实现业务逻辑)
  - [5.4 注册到项目](#54-注册到项目)
  - [5.5 在 CLI 中使用](#55-在-cli-中使用)
- [6. 适配器使用](#6-适配器使用)
  - [6.1 PostgreSQL + PostGIS](#61-postgresql--postgis)
  - [6.2 QGIS 集成](#62-qgis-集成)
  - [6.3 MQTT 传感器接入](#63-mqtt-传感器接入)
  - [6.4 CamoFox 网页数据接入](#64-camofox-网页数据接入)
  - [6.5 NMEA GPS 解析](#65-nmea-gps-解析)
- [7. 适配器开发](#7-适配器开发)
  - [7.1 ExternalAdapter trait](#71-externaladapter-trait)
  - [7.2 创建适配器骨架](#72-创建适配器骨架)
  - [7.3 示例：开发一个 TiDB 适配器](#73-示例开发一个-tidb-适配器)
  - [7.4 适配器开发约束](#74-适配器开发约束)
  - [7.5 在 CLI 中注册适配器](#75-在-cli-中注册适配器)
- [8. 测试](#8-测试)
- [9. 常见问题](#9-常见问题)
- [10. MCP 集成（AI Agent 调用）](#10-mcp-集成ai-agent-调用)
  - [10.1 启动 MCP Server](#101-启动-mcp-server)
  - [10.2 完整工具列表](#102-完整工具列表)
  - [10.3 安全审计](#103-安全审计)

---

## 1. 环境搭建

### 1.1 Rust 工具链

```bash
# 安装 Rust（如果已安装则跳过）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 重启终端或执行
source "$HOME/.cargo/env"

# 确认版本 ≥ 1.80
rustc --version
```

### 1.2 必需依赖

这些是**编译 geo-toolbox 本身**所需的：

```bash
# ── macOS ──
xcode-select --install          # 命令行工具（含 C 编译器）

# ── Ubuntu/Debian ──
sudo apt update
sudo apt install build-essential pkg-config libssl-dev
```

### 1.3 可选依赖（按需安装）

根据你实际要用的功能选择性安装，**不需要全装**。

#### PostgreSQL + PostGIS（存储和碳核算数据库）

```bash
# ── macOS ──
brew install postgresql@16 postgis
brew services start postgresql@16

# ── Ubuntu ──
sudo apt install postgresql-16 postgis
sudo systemctl start postgresql
```

创建数据库和用户：

```bash
sudo -u postgres createuser geo -P
# 输入密码：geo（开发用，生产请换强密码）

sudo -u postgres createdb geo_test -O geo

sudo -u postgres psql geo_test -c "CREATE EXTENSION postgis;"
sudo -u postgres psql geo_test -c "CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";"
```

设置环境变量：

```bash
echo 'export DATABASE_URL=postgres://geo:geo@localhost/geo_test' >> ~/.bashrc
source ~/.bashrc
```

#### QGIS（空间分析、制图输出）

从 [qgis.org/download](https://qgis.org/download/) 下载安装。

确认 `qgis_process` 可用：

```bash
qgis_process --version
# 应输出版本号
```

#### GDAL（栅格处理）

```bash
# macOS
brew install gdal

# Ubuntu
sudo apt install gdal-bin libgdal-dev

# 验证
gdal_translate --version
```

#### NATS（GEE 分布式消息队列）

```bash
# macOS
brew install nats-server

# 直接下载二进制
curl -sf https://binaries.nats.dev/nats-io/nats-server/v2@main | sh

# 验证
nats-server --version
```

#### WASM（浏览器端发布）

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

### 1.4 验证安装

```bash
# 检查 Rust
rustc --version    # ≥ 1.80
cargo --version

# 检查数据库（如果安装了）
psql -U geo -d geo_test -c "SELECT PostGIS_Version();"

# 检查 QGIS（如果安装了）
qgis_process list | head -5
```

---

## 2. 克隆与首次编译

```bash
git clone https://github.com/Miku196/geo-toolbox.git
cd geo-toolbox
```

### 最小化编译（无需任何外部依赖）

```bash
cargo build --release --no-default-features --features minimal
```

编译产物：`target/release/geo-toolbox`（Windows: `.exe`）

```bash
# 快速验证
cargo run -- crs list
# 输出：
#   EPSG:4326 | WGS 84              | Storage
#   EPSG:3857 | WGS 84 / Pseudo-Mercator | Display
#   ...
```

### 按需开启 feature

```bash
# 仅需要 PostGIS
cargo build --release --no-default-features --features minimal,postgis

# 仅需要 QGIS
cargo build --release --no-default-features --features qgis

# 仅需要 MQTT
cargo build --release --features mqtt
```

### 运行全部测试（无需任何外部服务）

```bash
cargo test --workspace
# 预期：167 个测试全部通过
```

---

## 3. 项目结构速览

```
geo-toolbox/
├── core/                          # 纯 Rust 核心引擎（13 crates）
│   ├── geo-core/                  # 几何基类、CRS 注册、错误类型、插件 trait
│   ├── geo-carbon-math/           # IPCC Tier 1 碳核算公式
│   ├── geo-raster/                # 栅格运算 + NDVI/NDWI
│   ├── geo-vector/                # 矢量空间运算
│   ├── geo-tile/                  # MVT/PMTiles 瓦片
│   ├── geo-temporal/              # 时空序列分析
│   ├── geo-stats/                 # 空间统计
│   ├── geo-io/                    # GeoJSON/CSV/NMEA/GPS 解析
│   ├── geo-report/                # Tera 报告模板引擎
│   ├── geo-index/                 # GeoHash 空间索引
│   ├── geo-parquet/               # GeoParquet 云原生格式
│   ├── geo-ogc/                   # WMS/WFS/WPS 标准
│   └── geo-registry/              # 插件注册调度中心
│
├── plugins/                       # 专业领域插件（10 crates）
│   ├── geo-plugin-carbon/         # 碳核算插件
│   ├── geo-plugin-ecology/        # 生态修复评估（NDVI + 碳汇 + 报告）
│   ├── geo-plugin-survey/         # 测绘
│   ├── geo-plugin-urban/          # 城乡规划
│   ├── geo-plugin-hydro/          # 水文分析
│   ├── geo-plugin-geohazard/      # 地质灾害
│   └── geo-plugin-agri/           # 农业
│   ├── geo-plugin-energy/         # 新能源选址
│   ├── geo-plugin-forestry/       # 林业碳汇
│   ├── geo-plugin-coastal/        # 海岸带
│
├── adapters/                      # 外部适配器（10 crates）
│   ├── geo-adapter-duckdb/        # SQLite 嵌入式
│   ├── geo-adapter-stac/          # STAC 数据发现
│   ├── geo-adapter-osm/           # OpenStreetMap
│   ├── geo-adapter-postgis/       # PostgreSQL + PostGIS
│   ├── geo-adapter-gee/           # Google Earth Engine
│   ├── geo-adapter-qgis/          # QGIS 桥接
│   ├── geo-adapter-cad/           # CAD 格式
│   ├── geo-adapter-cli/           # GDAL/DVC 子进程
│   ├── geo-adapter-mcp/           # MCP 协议（AI Agent）
│   └── geo-adapter-iot/           # MQTT 传感器
│
├── crates/                        # 入口
│   ├── geo-cli/                   # CLI + MCP Server
│   └── geo-wasm/                  # WASM + NPM 包
│
├── examples/                      # 示例
│   ├── chengdu-carbon/            # 成都碳收支
│   ├── china-risk-assessment/     # 中国风险评估
│   └── dexing-copper/             # 德兴铜矿生态修复
│
├── Cargo.toml                     # workspace 配置
├── README.md
└── DEVPLAN.md                     # 改造开发流程详规
```

**依赖方向**：`Adapter → Plugin → Core`（严格单向，下层不能依赖上层）

---

## 4. 核心功能使用

以下代码均可直接编译运行，无需任何外部服务。

### 4.1 CRS 坐标变换

```rust
use geo_core::crs::CrsRegistry;

fn main() {
    let reg = CrsRegistry::new();

    // WGS84 → Web Mercator
    let (x, y) = reg.transform_point(4326, 3857, 104.06, 30.57).unwrap();
    println!("成都 (104.06, 30.57) → Web Mercator ({:.0}, {:.0})", x, y);

    // WGS84 → GCJ-02（火星坐标系）
    let (gx, gy) = reg.transform_point(4326, 9000, 116.40, 39.90).unwrap();
    println!("北京 (116.40, 39.90) → GCJ-02 ({:.6}, {:.6})", gx, gy);

    // GCJ-02 → BD-09（百度坐标系）
    let (bx, by) = reg.transform_point(9000, 9001, gx, gy).unwrap();
    println!("     → BD-09 ({:.6}, {:.6})", bx, by);

    // 列出所有内置坐标系
    for c in reg.list() {
        println!("EPSG:{} | {:30} | {}", c.epsg, c.name, reg.by_category(c.category).len());
    }
}
```

**Cargo.toml：**

```toml
[dependencies]
geo-core = { git = "https://github.com/Miku196/geo-toolbox" }
```

### 4.2 碳核算

```rust
use geo_carbon_math::{CarbonEngine, EmissionFactor, GeoFeature};

fn main() {
    let engine = CarbonEngine::new();

    // 排放因子表（正值 = 排放源，负值 = 碳汇）
    let factors = vec![
        EmissionFactor::new("forest", -5.0, "IPCC_2019"),
        EmissionFactor::new("grassland", -1.2, "IPCC_2019"),
        EmissionFactor::new("built_up", 2.0, "IPCC_2019"),
    ];

    // 土地覆盖斑块（GeoJSON 多边形 + 类型标注）
    let forest_poly = r#"{"type":"Polygon","coordinates":[[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}"#;
    let builtup_poly = r#"{"type":"Polygon","coordinates":[[[104.1,30.5],[104.2,30.5],[104.2,30.6],[104.1,30.6],[104.1,30.5]]]}"#;

    let features = vec![
        GeoFeature::new("forest", forest_poly).unwrap(),
        GeoFeature::new("built_up", builtup_poly).unwrap(),
    ];

    let report = engine.calculate(&features, &factors, 2025).unwrap();

    println!("总评估面积: {:.1} ha", report.total_area_ha);
    println!("净碳排放:   {:.1} tCO₂e/yr", report.total_emission_tco2e);

    for c in &report.classes {
        let tag = if c.emission_tco2e < 0.0 { "🌿碳汇" } else { "🏭碳源" };
        println!("  {:<12} {:>8.1} ha  {:>8.1} tCO₂e  {}", c.landcover_class, c.area_ha, c.emission_tco2e, tag);
    }
}
```

### 4.3 NDVI 计算

```rust
use geo_raster::RasterBand;
use geo_raster::ndvi::compute_ndvi;

fn main() {
    // 模拟 100×100 像素的 Sentinel-2 波段
    let red = RasterBand::new("B4", 100, 100, vec![0.05; 10000], -999.0);
    let nir = RasterBand::new("B8", 100, 100, vec![0.50; 10000], -999.0);

    let result = compute_ndvi(&red, &nir).unwrap();

    println!("平均 NDVI:       {:?}", result.mean_ndvi);
    println!("健康植被比例:    {:?}", result.healthy_ratio);
    println!("退化植被比例:    {:?}", result.degraded_ratio);
    println!("有效像素:        {}", result.valid_pixels);
}
```

### 4.4 GeoJSON IO

```rust
use geo_io::geojson::parse_feature_collection;

fn main() {
    let geojson = r#"{
        "type": "FeatureCollection",
        "features": [
            {
                "type": "Feature",
                "properties": {"name": "成都高新区"},
                "geometry": {"type": "Polygon", "coordinates": [[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}
            }
        ]
    }"#;

    let (features, bbox) = parse_feature_collection(geojson).unwrap();
    println!("要素数: {}", features.len());
    println!("范围:   ({}, {}) ~ ({}, {})", bbox.min_x, bbox.min_y, bbox.max_x, bbox.max_y);
}
```

### 4.5 瓦片索引 (MVT/PMTiles)

```rust
use geo_tile::{latlon_to_tile, tile_to_latlon, tile_bounds, MvtEncoder};

// 经纬度 → 瓦片坐标
let (x, y, z) = latlon_to_tile(104.06, 30.57, 14);
println!("成都 z14: ({x}, {y})");

// 瓦片边界
let (w, s, e, n) = tile_bounds(x, y, z);
println!("范围: ({w}, {s}) ~ ({e}, {n})");

// GeoJSON → MVT 矢量瓦片（可直接喂给 MapLibre）
let encoder = MvtEncoder::new(4096);
let features = vec![serde_json::json!({
    "type": "Feature",
    "properties": {"name": "Chengdu"},
    "geometry": {"type": "Point", "coordinates": [104.06, 30.57]}
})];
let mvt_bytes = encoder.encode_tile("cities", &features, x, y, z)?;
// mvt_bytes 可直接 HTTP 返回给 MapLibre GL JS
```

### 4.6 时空趋势分析

```rust
use geo_temporal::trend::{linear_trend, mann_kendall};
use geo_temporal::raster_ts::RasterTimeSeries;

// 单像素趋势
let ndvi_series = vec![0.32, 0.35, 0.38, 0.41, 0.45];
let result = linear_trend(&ndvi_series);
println!("斜率: {:.4}/yr, 显著: {}", result.sen_slope, result.significant);

// 多期栅格逐像素 MK 趋势
let mut ts = RasterTimeSeries::new();
ts.add(2020, ndvi_2020)?;
ts.add(2021, ndvi_2021)?;
ts.add(2023, ndvi_2023)?;
let tau_map = ts.pixelwise_trend()?;   // 每个像素的 τ
let change = ts.change_detection(2020, 2023, 0.1)?;  // 改善/退化图
```

### 4.7 新能源选址

```rust
use geo_plugin_energy::{EnergyPlugin, EnergyConfig};

let plugin = EnergyPlugin::new(EnergyConfig::default());

// 光伏选址：坡度 < 25° + 年辐射 > 1500 kWh/m²
let solar = plugin.assess_solar("场址A", aoi_geojson, &dem, &radiation)?;
println!("适宜比例: {:.0}%, 评级: {}", solar.suitable_ratio * 100.0, solar.grade);

// 风电选址：风速 > 5.5 m/s + 坡度 < 15°
let wind = plugin.assess_wind("风场B", aoi_geojson, &dem, &wind_speed)?;
println!("均风速: {:.1} m/s, 评级: {}", wind.mean_windspeed, wind.grade);
```

### 4.8 DuckDB 嵌入式数据库

```rust
use geo_adapter_duckdb::DuckDbStore;

// 内存模式（零部署）
let store = DuckDbStore::in_memory()?;

// GeoJSON 直接导入
let count = store.ingest_geojson_raw("sites", geojson_fc_str)?;

// SQL 查询（自动 SQL 注入拦截）
let rows = store.query_json("SELECT name, lon, lat FROM sites WHERE lon > 104")?;

// 空间范围查询
let in_bbox = store.query_bbox("sites", 104.0, 30.0, 105.0, 31.0)?;
```

### 4.9 STAC 影像搜索

```rust
use geo_adapter_stac::StacClient;

let client = StacClient::new("https://planetarycomputer.microsoft.com/api/stac/v1");

let items = client.search(
    "sentinel-2-l2a",
    104.0, 30.0, 105.0, 31.0,
    "2025-06-01", "2025-06-30", 10,
).await?;
```

### 4.10 林业碳汇

```rust
use geo_plugin_forestry::{ForestryPlugin, ForestryConfig};

let plugin = ForestryPlugin::new(ForestryConfig::default());
let result = plugin.assess_carbon_stock(
    "林场A", aoi, &red_old, &nir_old, &red_new, &nir_new,
    2020, 2025, 200.0, 500.0,
)?;
println!("年碳汇: {:.0} tCO₂/yr, CCER: {}", -result.annual_sink_tco2_per_yr, result.ccer_applicable);
```

### 4.11 海岸带监测

```rust
use geo_plugin_coastal::CoastalPlugin;

let report = CoastalPlugin::new().assess_shoreline(
    "上海", aoi, &dem, &ndvi_2015, &ndvi_2025, 2015, 2025, 1.0,
)?;
println!("侵蚀: {:.0}%, 淹没: {:.0} ha", report.erosion_ratio*100.0, report.inundated_area_ha);
```

### 4.12 OSM 数据拉取

```rust
use geo_adapter_osm::OsmClient;

let client = OsmClient::new();
let elements = client.query_bbox(104.0,30.0,105.0,31.0, OsmFeature::Highway).await?;
let fc = OsmClient::to_geojson(&elements);
```

---

## 5. 插件开发

以开发一个"矿山风险评估插件"为例，走完整流程。

### 5.1 创建插件骨架

```bash
mkdir -p plugins/geo-plugin-mine/src
mkdir -p plugins/geo-plugin-mine/templates
```

**Cargo.toml：**

```toml
# plugins/geo-plugin-mine/Cargo.toml
[package]
name = "geo-plugin-mine"
version.workspace = true
edition.workspace = true
description = "矿山风险评估插件 — 坡度分析、植被覆盖、碳汇评估"

[dependencies]
geo-core = { path = "../../core/geo-core" }
geo-raster = { path = "../../core/geo-raster" }
geo-stats = { path = "../../core/geo-stats" }
geo-io = { path = "../../core/geo-io" }
geo-carbon-math = { path = "../../core/geo-carbon-math" }
geo-report = { path = "../../core/geo-report" }

serde.workspace = true
serde_json.workspace = true
toml.workspace = true
thiserror.workspace = true

# ⚠ 注意：不依赖 geo-adapter-*、不依赖其他 geo-plugin-*
```

### 5.2 编写配置

```rust
// plugins/geo-plugin-mine/src/config.rs
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct MineConfig {
    pub plugin: PluginMeta,
    #[serde(default)]
    pub slope: SlopeConfig,
    #[serde(default)]
    pub vegetation: VegetationConfig,
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
pub struct SlopeConfig {
    #[serde(default = "default_low_risk")]
    pub low_risk_deg: f64,
    #[serde(default = "default_high_risk")]
    pub high_risk_deg: f64,
}

fn default_low_risk() -> f64 { 10.0 }
fn default_high_risk() -> f64 { 25.0 }

#[derive(Debug, Clone, Deserialize)]
pub struct VegetationConfig {
    #[serde(default = "default_ndvi_healthy")]
    pub ndvi_healthy_min: f64,
    #[serde(default = "default_ndvi_degraded")]
    pub ndvi_degraded_max: f64,
}

fn default_ndvi_healthy() -> f64 { 0.5 }
fn default_ndvi_degraded() -> f64 { 0.2 }

#[derive(Debug, Clone, Deserialize)]
pub struct CarbonConfig {
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default)]
    pub forest: f64,
    #[serde(default)]
    pub grassland: f64,
    #[serde(default)]
    pub bare: f64,
}

fn default_source() -> String { "IPCC_2019".into() }

impl Default for MineConfig {
    fn default() -> Self {
        toml::from_str(include_str!("../rules.toml")).unwrap()
    }
}
```

**rules.toml：**

```toml
# plugins/geo-plugin-mine/rules.toml

[plugin]
name = "mine"
version = "0.1.0"
description = "矿山风险评估插件"

[slope]
low_risk_deg = 10.0
high_risk_deg = 25.0

[vegetation]
ndvi_healthy_min = 0.5
ndvi_degraded_max = 0.2

[carbon]
source = "IPCC_2019"
forest = -5.0
grassland = -1.2
bare = 0.0
```

### 5.3 实现业务逻辑

```rust
// plugins/geo-plugin-mine/src/mine.rs
use geo_core::errors::GeoResult;
use geo_core::types::BBox;
use geo_raster::ndvi::compute_ndvi;
use geo_raster::RasterBand;
use geo_carbon_math::{CarbonEngine, EmissionFactor, GeoFeature, CarbonReport};

use crate::config::MineConfig;

pub struct MinePlugin {
    config: MineConfig,
}

#[derive(Debug, serde::Serialize)]
pub struct MineAssessment {
    pub aoi_name: String,
    pub bbox: BBox,
    pub ndvi_mean: Option<f64>,
    pub vegetation_healthy: bool,
    pub slope_risk: String,
    pub carbon_report: CarbonReport,
    pub overall_risk: String,
    pub summary: String,
}

impl MinePlugin {
    pub fn new(config: MineConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &MineConfig {
        &self.config
    }

    pub fn from_file(path: &std::path::Path) -> GeoResult<Self> {
        let s = std::fs::read_to_string(path)?;
        let config: MineConfig = toml::from_str(&s)
            .map_err(|e| geo_core::GeoError::Validation(e.to_string()))?;
        Ok(Self { config })
    }

    /// 完整矿山风险评估
    pub fn assess(
        &self,
        aoi_name: &str,
        aoi_geojson: &str,
        red: &RasterBand,
        nir: &RasterBand,
        year: u16,
    ) -> GeoResult<MineAssessment> {
        let bbox = geo_io::extract_bbox(aoi_geojson)?;

        // 1. NDVI 植被分析
        let ndvi = compute_ndvi(red, nir)?;
        let mean_ndvi = ndvi.mean_ndvi.unwrap_or(0.0);
        let veg_healthy = mean_ndvi >= self.config.vegetation.ndvi_healthy_min;

        // 2. 坡度风险判定（简化：基于 NDVI 代理）
        //    实际应用中需结合 DEM 栅格数据
        let slope_risk = if mean_ndvi < self.config.vegetation.ndvi_degraded_max {
            "高风险".to_string()
        } else if mean_ndvi < self.config.vegetation.ndvi_healthy_min {
            "中风险".to_string()
        } else {
            "低风险".to_string()
        };

        // 3. 碳核算
        let engine = CarbonEngine::new();
        let factors = vec![
            EmissionFactor::new("forest", self.config.carbon.forest, &self.config.carbon.source),
            EmissionFactor::new("grassland", self.config.carbon.grassland, &self.config.carbon.source),
            EmissionFactor::new("bare", self.config.carbon.bare, &self.config.carbon.source),
        ];

        let fc: serde_json::Value = serde_json::from_str(aoi_geojson)
            .map_err(|e| geo_core::GeoError::Serde(e))?;
        let features = fc["features"].as_array()
            .ok_or_else(|| geo_core::GeoError::Validation("no features".into()))?
            .iter()
            .filter_map(|f| {
                let s = serde_json::to_string(f).ok()?;
                GeoFeature::from_feature_json(&s).ok()
            })
            .collect::<Vec<_>>();

        let carbon = engine.calculate(&features, &factors, year)
            .map_err(|e| geo_core::GeoError::Validation(e))?;

        // 4. 综合评级
        let risk_count = [
            veg_healthy,
            slope_risk == "低风险",
            carbon.is_net_sink(),
        ].iter().filter(|&&x| x).count();

        let (overall_risk, summary) = match risk_count {
            3 => ("🟢 低风险", format!("{} 矿山生态状况良好：植被健康、坡度安全、净碳汇。", aoi_name)),
            2 => ("🟡 中风险", format!("{} 矿山需关注：{} 项指标未达标，建议加强监测。", aoi_name, 3 - risk_count)),
            _ => ("🔴 高风险", format!("{} 矿山生态退化严重，需立即启动修复措施。", aoi_name)),
        };

        Ok(MineAssessment {
            aoi_name: aoi_name.to_string(),
            bbox,
            ndvi_mean: ndvi.mean_ndvi,
            vegetation_healthy: veg_healthy,
            slope_risk,
            carbon_report: carbon,
            overall_risk: overall_risk.to_string(),
            summary: summary.to_string(),
        })
    }
}
```

### 5.4 注册到项目

**lib.rs：**

```rust
// plugins/geo-plugin-mine/src/lib.rs
pub mod config;
pub mod mine;

pub use config::MineConfig;
pub use mine::{MinePlugin, MineAssessment};
```

**workspace Cargo.toml（根目录）：**

```toml
[workspace]
members = [
    # ... 其他成员 ...
    "plugins/geo-plugin-mine",   # ← 添加这一行
]
```

### 5.5 在 CLI 中使用

```rust
// crates/geo-cli/src/commands/mine.rs
use geo_plugin_mine::MinePlugin;
use geo_raster::RasterBand;

pub fn handle_mine_assess(
    aoi_name: &str,
    aoi_geojson: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let plugin = MinePlugin::from_file(
        std::path::Path::new("plugins/geo-plugin-mine/rules.toml")
    )?;

    // 构造测试波段（实际应从文件读取）
    let red = RasterBand::new("B4", 10, 10, vec![0.1; 100], -999.0);
    let nir = RasterBand::new("B8", 10, 10, vec![0.5; 100], -999.0);

    let result = plugin.assess(aoi_name, aoi_geojson, &red, &nir, 2025)?;

    println!("风险评估: {}", result.overall_risk);
    println!("{}", result.summary);
    Ok(())
}
```

---

## 6. 适配器使用

### 6.1 PostgreSQL + PostGIS

```rust
use geo_adapter_postgis::{PostgisStore, PostgisCarbonEngine, run_migrations};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = "postgres://geo:geo@localhost/geo_test";

    // 1. 连接数据库
    let store = PostgisStore::connect(db_url).await?;

    // 2. 确认 PostGIS 可用
    let version = store.check_postgis().await?;
    println!("PostGIS: {version}");

    // 3. 运行数据迁移
    run_migrations(store.pool()).await?;

    // 4. 查询（SQL 注入自动拦截）
    let rows = store.query_json("SELECT PostGIS_Version()").await?;
    println!("{:#?}", rows);

    // 5. 写入几何
    let wkb = vec![/* WKB 二进制数据 */];
    store.insert_geometry(
        None,
        "manual-import",
        &wkb,
        &serde_json::json!({"name": "测试区域"}),
    ).await?;

    // 6. 碳核算引擎
    let engine = PostgisCarbonEngine::new(store.pool().clone());

    // 导入排放因子 CSV
    engine.import_factors_csv("examples/chengdu-carbon/emission-factors.csv").await?;

    // 查询有效因子
    let factors = engine.query_factors(2025, Some("IPCC_2019")).await?;
    for f in &factors {
        println!("{} = {:.2} {} ({}..{:?})",
            f.category, f.factor_value, f.unit,
            f.valid_from_year, f.valid_to_year);
    }

    Ok(())
}
```

### 6.2 QGIS 集成

#### 方式 A：qgis_process 子进程

```rust
use geo_adapter_qgis::process_runner::{BatchQgisRunner, QgisProcessConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runner = BatchQgisRunner::new(QgisProcessConfig::default());

    // 重投影
    runner.reproject("input.geojson", 3405, "equalarea.gpkg").await?;

    // 缓冲区
    runner.buffer("sites.gpkg", 2000.0, "sites_buffer.gpkg").await?;

    // 裁剪
    runner.clip("landcover.gpkg", "aoi.gpkg", "clipped.gpkg").await?;

    // 相交
    runner.intersect("layer_a.gpkg", "layer_b.gpkg", "intersection.gpkg").await?;

    println!("处理完成");
    Ok(())
}
```

#### 方式 B：PyQGIS REST 服务

**启动服务端**（在 QGIS Python 控制台或独立脚本）：

```python
# 保存为 pyqgis_service.py
from flask import Flask, request, jsonify
import processing, uuid, threading

app = Flask(__name__)
jobs = {}

@app.route('/health')
def health():
    return "ok"

@app.route('/process', methods=['POST'])
def submit():
    data = request.json
    job_id = str(uuid.uuid4())
    jobs[job_id] = {"status": "pending", "progress": 0}

    def run():
        jobs[job_id]["status"] = "running"
        for i, tool in enumerate(data["tools"]):
            result = processing.run(tool["algorithm"], tool["params"])
            jobs[job_id]["progress"] = (i + 1) / len(data["tools"]) * 100
        jobs[job_id] = {"status": "completed", "progress": 100,
                        "output": result.get("OUTPUT")}
    threading.Thread(target=run).start()
    return jsonify({"job_id": job_id, "status": "accepted"})

@app.route('/status/<job_id>')
def status(job_id):
    return jsonify(jobs.get(job_id, {"status": "unknown"}))

@app.route('/result/<job_id>')
def result(job_id):
    job = jobs.get(job_id, {})
    return jsonify({"output_path": job.get("output", "")})

app.run(host="127.0.0.1", port=9100)
```

启动：

```bash
# 在 QGIS 安装目录的 Python 环境中
pip install flask
python pyqgis_service.py
```

**Rust 端调用**：

```rust
use geo_adapter_qgis::grpc_client::{QgisClient, QgisInput};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = QgisClient::new("http://localhost:9100");

    if !client.health_check().await? {
        eprintln!("PyQGIS 服务未启动");
        return Ok(());
    }

    // 缓冲区
    let output = client.buffer("sites.gpkg", 100.0, Some("sites_buffered")).await?;
    println!("输出: {output}");

    // 重投影
    let reprojected = client.reproject("input.geojson", 3857).await?;
    println!("重投影: {reprojected}");

    Ok(())
}
```

#### 终端直接调用（无需 Rust）

```bash
# 列出所有可用算法
qgis_process list

# 重投影到等积投影
qgis_process run native:reprojectlayer \
  --INPUT=chengdu-zones.geojson \
  --TARGET_CRS=EPSG:3405 \
  --OUTPUT=equalarea.gpkg

# 缓冲区分析
qgis_process run native:buffer \
  --INPUT=sites.gpkg \
  --DISTANCE=2000 \
  --OUTPUT=sites_buffer.gpkg

# 分区统计（栅格按矢量汇总）
qgis_process run native:zonalstatisticsfb \
  --INPUT=zones.gpkg \
  --INPUT_RASTER=dem.tif \
  --STATISTICS='mean,sum,count' \
  --OUTPUT=zones_stats.gpkg
```

### 6.3 MQTT 传感器接入

```rust
use geo_adapter_iot::mqtt::MqttIngestor;
use sqlx::postgres::PgPool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = PgPool::connect("postgres://geo:geo@localhost/geo_test").await?;

    let ingestor = MqttIngestor::new(pool);

    // 订阅传感器 topic，自动写入 iot_readings 表
    ingestor.start("localhost", 1883, "sensors/#").await?;

    Ok(())
}
```

**消息格式**（设备端按此 JSON 发布）：

```json
{
  "device_id": "gps-tracker-001",
  "sensor_type": "temperature",
  "value": 25.5,
  "lat": 30.57,
  "lng": 104.06
}
```

**验证规则**：
- `temperature` ∈ [-50, 60]
- `humidity` ∈ [0, 100]
- `pm25` ∈ [0, 1000]

**测试消息**：

```bash
mosquitto_pub -h localhost -t sensors/env/temperature \
  -m '{"device_id":"ws-001","sensor_type":"temperature","value":22.3,"lat":30.57,"lng":104.06}'

mosquitto_pub -h localhost -t sensors/env/pm25 \
  -m '{"device_id":"air-001","sensor_type":"pm25","value":45,"lat":30.66,"lng":104.07}'
```

**查询入库数据**：

```sql
SELECT time, device_id, sensor_type, value,
       ST_AsText(geom) AS location
FROM iot_readings
ORDER BY time DESC LIMIT 20;
```

### 6.4 CamoFox 网页数据接入

CamoFox 是 geo-toolbox 与 Pi Agent 的 `camoufox-browser` 技能配合使用的网页数据采集管道。
浏览器端抓取数据后，通过 `geo_io::camofox` 解析、验证、转换为空间要素。

#### 数据格式

支持三种 JSON 格式：

**格式 A：JSON 数组**

```json
[
  {"name": "梧桐山", "lat": 22.55, "lng": 114.06, "type": "forest", "area_ha": 3170},
  {"name": "深圳湾", "lat": 22.54, "lng": 113.93, "type": "park", "area_ha": 128.5}
]
```

**格式 B：单个 JSON 对象**

```json
{"name": "梧桐山", "lat": 22.55, "lng": 114.06, "type": "forest", "area_ha": 3170}
```

**格式 C：GeoJSON FeatureCollection**

```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "geometry": {"type": "Point", "coordinates": [114.06, 22.55]},
      "properties": {"name": "梧桐山", "type": "forest", "area_ha": 3170}
    }
  ]
}
```

#### 字段说明

| 字段 | 类型 | 必需 | 说明 |
|------|------|:---:|------|
| `name` | string | ✅ | 地点名称 |
| `lat` | number | ✅ | 纬度（-90 ~ 90） |
| `lng` | number | ✅ | 经度（-180 ~ 180） |
| `type` | string | ❌ | 土地覆盖类型（forest/grassland/park/water/built_up/...） |
| `area_ha` | number | ❌ | 面积（公顷） |
| _其他_ | any | ❌ | 额外字段自动合并到 properties |

#### Rust 代码调用

```rust
use geo_io::camofox::parse_camofox_file;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let json = std::fs::read_to_string("scraped_sites.json")?;

    // 解析 + 验证 + 生成 WKB SpatialRow
    let (rows, result) = parse_camofox_file(&json, "shenzhen-parks")?;

    println!("解析完成: {} 接受, {} 拒绝", result.accepted, result.rejected);

    for err in &result.errors {
        eprintln!("  拒绝: {err}");
    }

    // rows 可直接写入 PostGIS（见 6.1）
    for row in &rows {
        // row.wkb        — WKB 二进制几何
        // row.properties — JSON 属性
        // row.source     — 数据来源标识
        println!("  {:?}", row);
    }

    Ok(())
}
```

#### 写入 PostGIS

```rust
// 接上例，批量写入数据库
use geo_adapter_postgis::PostgisStore;

let store = PostgisStore::connect("postgres://geo:geo@localhost/geo_test").await?;

for row in &rows {
    let props: serde_json::Value = serde_json::from_str(&row.properties)?;
    store.insert_geometry(None, &row.source, &row.wkb, &props).await?;
}
println!("已写入 {} 条记录", rows.len());
```

#### CLI 命令行调用

```bash
# 解析 CamoFox JSON 文件（纯解析，不写库）
geo-toolbox ingest camofox scraped_sites.json
# 输出:
#   CamoFox ingest: 2 accepted, 0 rejected
#   Sample records:
#     1. 梧桐山 (forest) @ (114.06, 22.55)
#     2. 深圳湾 (park) @ (113.93, 22.54)

# 解析 + 写入 PostGIS
DATABASE_URL=postgres://geo:geo@localhost/geo_test \
  geo-toolbox ingest camofox scraped_sites.json
# 输出:
#   Written 2 rows to spatial_assets
```

#### MCP 协议调用（AI Agent）

```json
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"ingest_camofox","arguments":{"file":"scraped_sites.json"}}}
```

#### 验证规则

解析时自动执行：
- 经纬度 ∈ 合法区间（lat: [-90, 90], lng: [-180, 180]）
- 超出范围 → 记入 `result.rejected` + `result.errors`，跳过该条
- 格式错误 → 返回 `Err(GeoError::Validation(...))`

#### 与 camoufox-browser 技能配合

```
┌─────────────────┐     JSON      ┌──────────────┐    WKB+props   ┌──────────┐
│ camoufox-browser │ ───────────→ │ geo-io       │ ─────────────→ │ PostGIS  │
│ (网页数据采集)    │  scrape结果   │ camofox解析   │  SpatialRow   │ spatial_ │
│                  │              │ +坐标验证      │               │ assets   │
└─────────────────┘              └──────────────┘              └──────────┘
```

### 6.5 NMEA GPS 解析

解析标准 NMEA 0183 协议的 GPS 日志，支持 GGA（定位信息）和 RMC（推荐最小数据）语句。

#### 支持的语句

| 语句 | 说明 | 关键字段 |
|------|------|---------|
| `$GPGGA` | 全球定位系统固定数据 | 时间、经纬度、定位质量、卫星数、HDOP、海拔 |
| `$GPRMC` | 推荐最小导航信息 | 时间、经纬度、速度（节）、航向、日期 |

#### Rust 代码调用

```rust
use geo_io::nmea::{parse_nmea_line, NmeaMessage};

fn main() {
    let nmea = "$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47";

    match parse_nmea_line(nmea).unwrap() {
        NmeaMessage::Gga(fix) => {
            println!("定位时间: {}", fix.time);
            println!("纬度:     {:.6}°", fix.lat);
            println!("经度:     {:.6}°", fix.lng);
            println!("精度因子: {:.1}", fix.hdop);
            println!("卫星数:   {}", fix.satellites);
            println!("海拔:     {:.1}m", fix.altitude);
        }
        NmeaMessage::Rmc(rmc) => {
            println!("速度:     {:.1} 节", rmc.speed_knots);
            println!("航向:     {:.1}°", rmc.track);
        }
        _ => {}
    }
}
```

#### 批量解析日志文件

```rust
use geo_io::nmea::{parse_nmea_line, NmeaMessage};

fn main() {
    let log = std::fs::read_to_string("gps_log.txt").unwrap();
    let mut fixes = 0;

    for line in log.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }

        match parse_nmea_line(line) {
            Ok(NmeaMessage::Gga(fix)) => {
                println!("[{}] ({:.6}, {:.6}) 精度={:.1} 卫星={}",
                    fix.time, fix.lat, fix.lng, fix.hdop, fix.satellites);
                fixes += 1;
            }
            Ok(NmeaMessage::Rmc(rmc)) => {
                println!("[{}] ({:.6}, {:.6}) 速度={:.1}kt 航向={:.1}°",
                    rmc.time, rmc.lat, rmc.lng, rmc.speed_knots, rmc.track);
                fixes += 1;
            }
            Err(e) => eprintln!("解析失败: {e}"),
            _ => {}
        }
    }
    println!("共解析 {fixes} 条定位记录");
}
```

#### CLI 命令行调用

```bash
geo-toolbox ingest nmea gps_log.txt
```

#### MCP 协议调用

```json
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"ingest_nmea","arguments":{"file":"gps_log.txt"}}}
```

#### GPS 质量验证

`geo_io::validator::validate_gps_fix()` 提供：

```rust
use geo_io::validator::validate_gps_fix;

// 检查 HDOP ≤ 5.0 且卫星数 ≥ 4
validate_gps_fix(1.5, 12).unwrap();  // ✅ 好的定位
validate_gps_fix(6.0, 10).unwrap();  // ❌ HDOP 太高
validate_gps_fix(2.0, 3).unwrap();   // ❌ 卫星太少
```

---

## 7. 适配器开发

适配器是 geo-toolbox 与外部系统（数据库、GIS 引擎、消息队列、文件格式）之间的桥梁。
与插件不同，适配器**允许持有外部连接和调用外部进程**。

### 7.1 ExternalAdapter trait

所有适配器必须实现 `ExternalAdapter` trait（定义在 `geo-core`）：

```rust
// core/geo-core/src/plugin.rs

pub trait ExternalAdapter: Plugin {
    /// 外部服务端点标识（URL、连接串、命令名）
    fn external_endpoint(&self) -> &str;

    /// 健康检查：外部服务是否可达
    async fn health_check(&self) -> GeoResult<bool>;

    /// 获取外部工具版本
    async fn external_version(&self) -> GeoResult<String>;

    /// 是否需要网络
    fn requires_network(&self) -> bool { true }

    // ── 双向通信核心 ──

    /// 推送数据到外部系统
    async fn push(&self, table: &str, data: &[GeoFeature]) -> GeoResult<u64>;

    /// 从外部系统拉取数据
    async fn pull(&self, query: &str) -> GeoResult<Vec<GeoFeature>>;

    /// 执行外部命令/操作
    async fn execute(&self, command: &str, params: serde_json::Value) -> GeoResult<serde_json::Value>;
}
```

同时还需要实现基 trait `Plugin`：

```rust
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    fn category(&self) -> PluginCategory;
    fn init(&mut self) -> GeoResult<()> { Ok(()) }
    fn shutdown(&mut self) -> GeoResult<()> { Ok(()) }
    fn is_healthy(&self) -> bool { true }

    /// 构建 PluginMeta 快照（默认委托给上面方法；可覆盖提供 extra 字段）
    fn metadata(&self) -> PluginMeta {
        PluginMeta {
            name: self.name().to_string(),
            version: self.version().to_string(),
            description: self.description().to_string(),
            category: self.category(),
            healthy: self.is_healthy(),
            extra: serde_json::Value::Null,
        }
    }

    /// Adapter 判断快捷方式
    fn is_adapter(&self) -> bool {
        self.category() == PluginCategory::Adapter
    }

    /// Carbon 插件判断快捷方式
    fn is_carbon(&self) -> bool {
        self.category() == PluginCategory::Carbon
    }
}
```

### 7.2 创建适配器骨架

```bash
mkdir -p adapters/geo-adapter-mysystem/src
```

**Cargo.toml：**

```toml
# adapters/geo-adapter-mysystem/Cargo.toml
[package]
name = "geo-adapter-mysystem"
version.workspace = true
edition.workspace = true
description = "MySystem 适配器 — 双向数据桥接"

[dependencies]
geo-core = { path = "../../core/geo-core" }
# 适配器可以依赖 Plugin 层
geo-plugin-carbon = { path = "../../plugins/geo-plugin-carbon" }
# 适配器可以依赖其他 Adapter
geo-adapter-postgis = { path = "../geo-adapter-postgis" }
# 外部系统 SDK
# 例如: sqlx、reqwest、rumqttc、object_store 等

serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
tracing.workspace = true
```

**lib.rs：**

```rust
// adapters/geo-adapter-mysystem/src/lib.rs
#![allow(missing_docs)]

mod adapter;
pub use adapter::MySystemAdapter;
```

**adapter.rs：**

```rust
use geo_core::errors::GeoResult;
use geo_core::plugin::{
    ExternalAdapter, Plugin, PluginCategory, GeoFeature,
};

/// MySystem 适配器。
pub struct MySystemAdapter {
    endpoint: String,
    connected: bool,
    // 可以持有任何外部资源
    // client: SomeClient,
    // pool: PgPool,
}

impl MySystemAdapter {
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            connected: false,
        }
    }
}

// ── Plugin trait ──

impl Plugin for MySystemAdapter {
    fn name(&self) -> &str { "mysystem" }

    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }

    fn description(&self) -> &str {
        "MySystem bidirectional adapter"
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::Adapter
    }

    fn init(&mut self) -> GeoResult<()> {
        tracing::info!("MySystemAdapter connecting to {}", self.endpoint);
        // 连接外部系统
        // self.client = SomeClient::connect(&self.endpoint)?;
        self.connected = true;
        Ok(())
    }

    fn shutdown(&mut self) -> GeoResult<()> {
        self.connected = false;
        Ok(())
    }

    fn is_healthy(&self) -> bool {
        self.connected
    }
}

// ── ExternalAdapter trait ──

impl ExternalAdapter for MySystemAdapter {
    fn external_endpoint(&self) -> &str {
        &self.endpoint
    }

    async fn health_check(&self) -> GeoResult<bool> {
        // 尝试 ping 外部系统
        // self.client.ping().await?;
        Ok(self.connected)
    }

    async fn external_version(&self) -> GeoResult<String> {
        // self.client.version().await
        Ok("MySystem v2.0".into())
    }

    fn requires_network(&self) -> bool {
        true
    }

    async fn push(
        &self,
        table: &str,
        data: &[GeoFeature],
    ) -> GeoResult<u64> {
        // 批量写入外部系统
        let count = data.len() as u64;
        tracing::info!("Pushed {count} features to {table}");
        Ok(count)
    }

    async fn pull(
        &self,
        query: &str,
    ) -> GeoResult<Vec<GeoFeature>> {
        // 从外部系统拉取数据
        tracing::info!("Pulling data: {query}");
        Ok(vec![])
    }

    async fn execute(
        &self,
        command: &str,
        params: serde_json::Value,
    ) -> GeoResult<serde_json::Value> {
        // 通用操作
        tracing::info!("Executing {command} with {params}");
        Ok(serde_json::json!({"status": "ok"}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let adapter = MySystemAdapter::new("http://localhost:8080");
        assert_eq!(adapter.name(), "mysystem");
        assert_eq!(adapter.category(), PluginCategory::Adapter);
        assert!(adapter.requires_network());
    }
}
```

### 7.3 示例：开发一个 TiDB 适配器

假设需要支持 TiDB（兼容 MySQL 协议的分布式数据库）作为 PostGIS 的替代存储后端。

**目录结构：**

```
adapters/geo-adapter-tidb/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── adapter.rs      # ExternalAdapter 实现
    ├── store.rs        # TiDB 空间查询封装
    └── migrate.sql     # 建表 DDL
```

**Cargo.toml：**

```toml
[package]
name = "geo-adapter-tidb"
version.workspace = true
edition.workspace = true
description = "TiDB spatial adapter — MySQL-compatible distributed storage"

[dependencies]
geo-core = { path = "../../core/geo-core" }
sqlx = { workspace = true, features = ["mysql", "runtime-tokio"] }
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
tracing.workspace = true
uuid.workspace = true
```

**lib.rs：**

```rust
pub mod adapter;
pub mod store;

pub use adapter::TidbAdapter;
pub use store::TidbStore;
```

**store.rs — 空间查询封装：**

```rust
use geo_core::errors::{GeoError, GeoResult};
use sqlx::mysql::MySqlPool;
use uuid::Uuid;

pub struct TidbStore {
    pool: MySqlPool,
}

impl TidbStore {
    pub async fn connect(url: &str) -> GeoResult<Self> {
        let pool = MySqlPool::connect(url).await
            .map_err(|e| GeoError::Database(e.to_string()))?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &MySqlPool { &self.pool }

    /// 写入空间要素（TiDB 支持 WKB + ST_GeomFromWKB）
    pub async fn insert_feature(
        &self,
        aoi_id: Uuid,
        wkb: &[u8],
        props: &serde_json::Value,
    ) -> GeoResult<Uuid> {
        let row = sqlx::query(
            "INSERT INTO spatial_assets (aoi_id, geom, properties)
             VALUES (?, ST_GeomFromWKB(?, 4326), ?)
             RETURNING id"
        )
        .bind(aoi_id)
        .bind(wkb)
        .bind(props)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| GeoError::Database(e.to_string()))?;

        Ok(row.get(0))
    }

    /// 空间范围查询（利用 TiDB 空间索引）
    pub async fn query_bbox(
        &self,
        min_x: f64, min_y: f64, max_x: f64, max_y: f64,
    ) -> GeoResult<Vec<serde_json::Value>> {
        let rows = sqlx::query(
            "SELECT id, ST_AsGeoJSON(geom) AS geojson, properties
             FROM spatial_assets
             WHERE MBRContains(
                 ST_GeomFromText(CONCAT('POLYGON((',?, ' ',?, ',', ?, ' ',?, ',', ?, ' ',?, ',', ?, ' ',?, ',', ?, ' ',?, '))'), 4326),
                 geom
             )"
        )
        .bind(min_x).bind(min_y)  // 左下
        .bind(max_x).bind(min_y)  // 右下
        .bind(max_x).bind(max_y)  // 右上
        .bind(min_x).bind(max_y)  // 左上
        .bind(min_x).bind(min_y)  // 闭合
        .fetch_all(&self.pool)
        .await
        .map_err(|e| GeoError::Database(e.to_string()))?;

        // 转换为 JSON
        let result: Vec<serde_json::Value> = rows.iter().map(|row| {
            serde_json::json!({
                "id": row.get::<String, _>(0),
                "geojson": row.get::<String, _>(1),
                "properties": row.get::<serde_json::Value, _>(2),
            })
        }).collect();

        Ok(result)
    }

    /// 运行迁移 DDL
    pub async fn migrate(&self) -> GeoResult<()> {
        let sql = include_str!("migrate.sql");
        for stmt in sql.split(';').filter(|s| !s.trim().is_empty()) {
            sqlx::query(stmt).execute(&self.pool).await
                .map_err(|e| GeoError::Database(e.to_string()))?;
        }
        tracing::info!("TiDB migration complete");
        Ok(())
    }
}
```

**adapter.rs — ExternalAdapter 实现：**

```rust
use geo_core::errors::GeoResult;
use geo_core::plugin::{
    ExternalAdapter, Plugin, PluginCategory, GeoFeature,
};
use crate::store::TidbStore;

pub struct TidbAdapter {
    url: String,
    store: Option<TidbStore>,
}

impl TidbAdapter {
    pub fn new(url: &str) -> Self {
        Self { url: url.to_string(), store: None }
    }

    pub fn store(&self) -> Option<&TidbStore> {
        self.store.as_ref()
    }
}

impl Plugin for TidbAdapter {
    fn name(&self) -> &str { "tidb" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
    fn description(&self) -> &str {
        "TiDB distributed spatial storage adapter"
    }
    fn category(&self) -> PluginCategory { PluginCategory::Adapter }

    fn init(&mut self) -> GeoResult<()> {
        // 注意：async init 需要特殊处理
        // 实际项目中 init() 应为 async，或使用 tokio::runtime::Handle
        tracing::info!("TidbAdapter registered (connect on first use)");
        Ok(())
    }
}

impl ExternalAdapter for TidbAdapter {
    fn external_endpoint(&self) -> &str { &self.url }

    async fn health_check(&self) -> GeoResult<bool> {
        match &self.store {
            Some(s) => {
                sqlx::query("SELECT 1").execute(s.pool()).await
                    .map(|_| true)
                    .map_err(|e| geo_core::GeoError::Database(e.to_string()))
            }
            None => {
                // 未连接时尝试快速连接测试
                let test_store = TidbStore::connect(&self.url).await?;
                sqlx::query("SELECT 1").execute(test_store.pool()).await
                    .map(|_| true)
                    .map_err(|e| geo_core::GeoError::Database(e.to_string()))
            }
        }
    }

    async fn external_version(&self) -> GeoResult<String> {
        match &self.store {
            Some(s) => {
                let row: (String,) = sqlx::query_as("SELECT VERSION()")
                    .fetch_one(s.pool()).await
                    .map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
                Ok(row.0)
            }
            None => Ok("TiDB (not connected)".into()),
        }
    }

    fn requires_network(&self) -> bool { true }

    async fn push(&self, table: &str, data: &[GeoFeature]) -> GeoResult<u64> {
        let store = self.store.as_ref()
            .ok_or_else(|| geo_core::GeoError::Other("TiDB not connected".into()))?;

        let mut count = 0u64;
        for feature in data {
            let wkb = feature.to_wkb_bytes(); // 假设 GeoFeature 有此方法
            // 实际需要从 GeoJSON geometry 编码为 WKB
            // store.insert_feature(uuid, &wkb, &feature.properties).await?;
            count += 1;
        }
        tracing::info!("Pushed {count} features to TiDB table {table}");
        Ok(count)
    }

    async fn pull(&self, query: &str) -> GeoResult<Vec<GeoFeature>> {
        let store = self.store.as_ref()
            .ok_or_else(|| geo_core::GeoError::Other("TiDB not connected".into()))?;

        // 查询并转换为 GeoFeature 列表
        // let results = store.query_bbox(...).await?;
        tracing::info!("Pulling from TiDB: {query}");
        Ok(vec![])
    }

    async fn execute(&self, command: &str, params: serde_json::Value) -> GeoResult<serde_json::Value> {
        let store = self.store.as_ref()
            .ok_or_else(|| geo_core::GeoError::Other("TiDB not connected".into()))?;

        match command {
            "migrate" => {
                store.migrate().await?;
                Ok(serde_json::json!({"status": "migrated"}))
            }
            _ => Err(geo_core::GeoError::Unimplemented(
                format!("Unknown command: {command}")
            )),
        }
    }
}
```

**migrate.sql：**

```sql
CREATE TABLE IF NOT EXISTS spatial_assets (
    id        VARCHAR(36) PRIMARY KEY,
    aoi_id    VARCHAR(36),
    geom      GEOMETRY NOT NULL,
    properties JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    SPATIAL INDEX(geom)
);

CREATE TABLE IF NOT EXISTS carbon_accounting_results (
    calc_id          VARCHAR(36) PRIMARY KEY,
    workflow_run_id  VARCHAR(36) NOT NULL,
    aoi_id           VARCHAR(36) NOT NULL,
    landcover_class  VARCHAR(100) NOT NULL,
    area_ha          DOUBLE NOT NULL,
    emission_tco2e   DOUBLE NOT NULL,
    factor_set_id    VARCHAR(36),
    audit_status     VARCHAR(20) DEFAULT 'pending',
    calculation_at   TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### 7.4 适配器开发约束

| 约束 | 说明 |
|------|------|
| ✅ 必须实现 | `Plugin` + `ExternalAdapter` 两个 trait |
| ✅ 可依赖 | Core 层所有 crate、Plugin 层所有 crate、其他 Adapter |
| ✅ 可持有 | 数据库连接池、HTTP Client、子进程句柄、MQTT 连接、文件句柄 |
| ✅ 可调用 | 外部命令行工具（`qgis_process`、`gdal_translate`、`dvc`） |
| ❌ 禁止方向 | Adapter 不能反过来被 Core/Plugin 依赖 |
| ✅ 需注册 | 在 workspace `Cargo.toml` 的 `members` 中、CLI `Cargo.toml` 的 `dependencies` 中 |

### 7.5 在 CLI 中注册适配器

```toml
# crates/geo-cli/Cargo.toml
[dependencies]
geo-adapter-tidb = { path = "../../adapters/geo-adapter-tidb" }
```

```rust
// crates/geo-cli/src/main.rs
fn build_registry() -> PluginRegistry {
    let mut reg = PluginRegistry::new();

    // 注册适配器
    reg.register(PluginMeta {
        name: "tidb".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        description: "TiDB distributed spatial storage".into(),
        category: PluginCategory::Adapter,
        healthy: true,
        extra: serde_json::json!({"endpoint": "tidb://localhost:4000"}),
    });

    // 注册工具
    reg.register_tool("tidb", ToolDef {
        name: "tidb_migrate".into(),
        description: "Run TiDB schema migration".into(),
        input_schema: serde_json::json!({"type": "object", "properties": {}}),
    });

    reg
}
```

```rust
// crates/geo-cli/src/mcp.rs — 处理工具调用
"tidb_migrate" => {
    let mut adapter = geo_adapter_tidb::TidbAdapter::new("tidb://localhost:4000");
    adapter.init()?;
    let result = adapter.execute("migrate", serde_json::json!({})).await?;
    Ok(json!({"jsonrpc": "2.0", "result": {"content": [{"type": "text", "text": result.to_string()}]}}))
}
```

---

## 8. 测试

### 运行测试

```bash
# 全部测试（无需任何外部服务）
cargo test --workspace

# 单个 crate
cargo test -p geo-core
cargo test -p geo-carbon-math
cargo test -p geo-plugin-ecology

# 含数据库的测试（需先安装并启动 PostgreSQL）
DATABASE_URL=postgres://geo:geo@localhost/geo_test cargo test -p geo-adapter-postgis
```

### 测试结果期望

```
running 167 tests
test result: ok. 167 passed; 0 failed; ...
```

### 代码质量

```bash
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

---

## 9. 常见问题

### Q: 编译报错 `error: linker 'cc' not found`

```bash
# macOS
xcode-select --install

# Ubuntu
sudo apt install build-essential
```

### Q: `cargo test` 卡住不动

某些测试用了 `tokio::time::sleep`，但超时时间很短（≤ 1s）。如果确实卡住：

```bash
cargo test --workspace -- --test-threads=1
```

### Q: PostGIS 连接失败

```bash
# 检查 PostgreSQL 是否运行
pg_isready -U geo -d geo_test

# 检查 PostGIS 扩展
psql -U geo -d geo_test -c "SELECT PostGIS_Version();"

# 确认环境变量
echo $DATABASE_URL
```

### Q: `qgis_process` 找不到

Windows 需将 QGIS bin 目录加入 PATH：

```powershell
$env:Path += ";C:\Program Files\QGIS 3.34\bin"
```

或使用完整路径：

```bash
"C:/Program Files/QGIS 3.34/bin/qgis_process-qgis.bat" run native:buffer ...
```

### Q: WASM 编译缺少 wasm32 target

```bash
rustup target add wasm32-unknown-unknown
```

### Q: 如何只编译核心功能（不依赖任何外部系统）

```bash
cargo build --release --no-default-features --features minimal
```

这条命令只会编译 `geo-core` + `geo-cli` 的基础部分，不需要 PostGIS/GDAL/QGIS。

### Q: 插件之间的依赖约束是什么

```
✅ Plugin 可以依赖 → Core 层的任何 crate
❌ Plugin 不能依赖 → 其他 Plugin（横向）
❌ Plugin 不能依赖 → Adapter 层的任何 crate
❌ Core 不能依赖   → Plugin 或 Adapter
```

如果多个插件需要同一功能，应将其下沉到 Core 层。

---

## 10. MCP 集成（AI Agent 调用）

geo-toolbox 内置 MCP (Model Context Protocol) Server，支持 AI Agent（Claude、Pi Agent 等）通过 JSON-RPC stdio 直接调用全部 44 个空间分析工具。

### 10.1 启动 MCP Server

```bash
# 编译（全功能）
cargo build --release

# 启动 MCP Server（stdio 模式）
./target/release/geo-toolbox mcp-serve

# 轻量模式（仅 Core 功能，零外部依赖）
cargo build --release --no-default-features --features minimal
```

### 10.2 完整工具列表

| 分类 | 工具名称 | 功能 | 来源 crate |
|------|---------|------|-----------|
| **CRS** | `crs_list` | 列出所有坐标系 | geo-io |
| | `crs_transform` | 坐标变换（含GCJ-02/BD-09） | geo-io |
| **瓦片** | `tile_latlon_to_tile` | 经纬度→瓦片坐标 | geo-tile |
| | `tile_bounds` | 瓦片边界 | geo-tile |
| | `tile_url` | OSM/高德/天地图URL | geo-tile |
| **索引** | `geohash_encode` | 经纬度→GeoHash | geo-index |
| | `geohash_decode` | GeoHash→边界框 | geo-index |
| | `geohash_neighbors` | 8邻域GeoHash | geo-index |
| **矢量** | `vector_buffer` | 多边形缓冲区 | geo-vector |
| | `vector_intersect` | 多边形相交 | geo-vector |
| | `vector_area` | 多边形面积 | geo-vector |
| | `vector_centroid` | 多边形质心 | geo-vector |
| **时序** | `temporal_trend` | Mann-Kendall趋势+Sen斜率 | geo-temporal |
| **统计** | `zonal_stats` | 栅格分区统计 | geo-stats |
| **碳核算** | `carbon_calculate_raw` | GeoJSON+CSV→碳排放 | geo-carbon-math |
| | `carbon_calculate_geojson` | 插件配置+GeoJSON→碳报告 | geo-plugin-carbon |
| **报告** | `report_carbon` | 碳核算Markdown报告 | geo-report |
| | `report_render` | 通用Tera模板渲染 | geo-report |
| **生态** | `ecology_assess` | 生态修复评估（NDVI+碳汇） | geo-plugin-ecology |
| **能源** | `energy_solar_suitability` | 光伏选址适宜性 | geo-plugin-energy |
| **林业** | `forestry_carbon_stock` | 林业碳储量变化 | geo-plugin-forestry |
| **海岸** | `coastal_shoreline` | 海岸带侵蚀+淹没 | geo-plugin-coastal |
| **测绘** | `survey_earthwork` | 土方量计算 | geo-plugin-survey |
| **水文** | `hydro_inundation` | 洪水淹没面积 | geo-plugin-hydro |
| | `hydro_runoff` | 径流系数 | geo-plugin-hydro |
| **地灾** | `geohazard_landslide` | 滑坡敏感性指数 | geo-plugin-geohazard |
| **农业** | `agri_yield` | 作物估产 | geo-plugin-agri |
| | `agri_soil` | 土壤评级 | geo-plugin-agri |
| **规划** | `urban_far` | 容积率+合规检查 | geo-plugin-urban |
| **数据** | `ingest_camofox` | CamoFox JSON解析 | geo-io |
| | `ingest_nmea` | NMEA GPS日志解析 | geo-io |
| | `duckdb_query` | DuckDB SQL查询 | geo-adapter-duckdb |
| | `duckdb_ingest_geojson` | GeoJSON→DuckDB | geo-adapter-duckdb |
| | `stac_search` | STAC影像目录搜索 ⚠网络 | geo-adapter-stac |
| | `osm_query_bbox` | OSM要素查询 ⚠网络 | geo-adapter-osm |
| | `store_query` | PostGIS SQL查询 ⚠DB | geo-adapter-postgis |
| | `store_migrate` | PostGIS迁移 ⚠DB | geo-adapter-postgis |
| **外部** | `qgis_buffer` | QGIS缓冲区 ⚠QGIS | geo-adapter-qgis |
| | `qgis_reproject` | QGIS重投影 ⚠QGIS | geo-adapter-qgis |
| | `cli_cog_convert` | GDAL COG转换 ⚠GDAL | geo-adapter-cli |
| | `cli_ogr2ogr` | 矢量格式转换 ⚠GDAL | geo-adapter-cli |
| | `gee_classify` | GEE土地覆盖分类 ⚠GEE | geo-adapter-gee |
| | `gee_status` | GEE任务状态查询 ⚠GEE | geo-adapter-gee |
| | `cad_export_geojson` | PostGIS→GeoJSON ⚠DB | geo-adapter-cad |
| | `dvc_snapshot` | DVC数据版本快照 ⚠DVC | geo-adapter-postgis |
| | `dvc_hash` | DVC文件哈希 ⚠DVC | geo-adapter-postgis |

> ⚠ 标记表示需要特定的外部依赖或环境变量。

### 10.3 安全审计

geo-toolbox 实施了以下安全措施：

| 防护 | 机制 | 范围 |
|------|------|------|
| SQL 注入防护 | `validate_select_sql()` 拦截 DROP/DELETE/INSERT/UPDATE/CREATE 等危险关键词，仅允许 SELECT | PostGIS 查询 |
| 路径遍历防护 | `validate_safe_path()` 拦截 `..`、shell 元字符(`;\|&$`()<>`)、敏感系统路径(/etc、/proc、C:\Windows) | QGIS、CLI adapter |
| 参数化查询 | 所有 INSERT 使用 `$1`、`$2` 绑定参数，不拼接 SQL | PostGIS 写入 |
| 子进程参数隔离 | `Command::new().args()` 逐参数传递，不拼接 shell 命令 | QGIS、GDAL 子进程 |
| 凭证管理 | 数据库密码通过 `DATABASE_URL` 环境变量注入，不硬编码 | 全局 |
| Rust 内存安全 | 无 unsafe 代码块，编译器保证内存安全 | 全局 |

**已知风险**（低优先级）：
- CLI `store write` 命令不校验文件路径，可能读取任意文件（但仅读取，不写入）
- `format!("COMPRESS={}", user_input)` 在 GDAL CLI 中可能被空格拆分参数

---

## 11. HTTP API Server（`crates/geo-server`）

### 11.1 启动

```bash
cargo run -p geo-server --release
# 监听 http://0.0.0.0:9378
```

### 11.2 API

| 端点 | 方法 | 说明 |
|------|------|------|
| `/health` | GET | 健康检查 |
| `/api/tools` | GET | 列出所有工具 |
| `/api/call/{tool}` | POST | 调用工具，body 为 JSON 参数 |

### 11.3 使用示例

```bash
# 坐标变换
curl -X POST http://localhost:9378/api/call/crs_transform \
  -d '{"from_epsg":4326,"to_epsg":3857,"x":104.06,"y":30.57}'

# 碳核算
curl -X POST http://localhost:9378/api/call/carbon_calculate_raw \
  -d '{"geojson":"...","csv":"source,category,factor_value\nIPCC_2019,forest,-5.0","year":2025}'

# 列出所有可用工具
curl http://localhost:9378/api/tools
```

---

## 12. Plugin 开发（ProcessPlugin trait）

所有插件实现统一的 `ProcessPlugin` trait，可通过 `PluginRegistry::dispatch()` 调度。

### 示例：UrbanPlugin

```rust
// plugins/geo-plugin-urban/src/trait_impl.rs
use crate::UrbanPlugin;
use geo_core::plugin::{Plugin, ProcessPlugin, PluginCategory};
use geo_core::errors::GeoResult;

impl Plugin for UrbanPlugin {
    fn name(&self) -> &str { "urban" }
    fn description(&self) -> &str { "Urban planning" }
    fn category(&self) -> PluginCategory { PluginCategory::Process }
}

impl ProcessPlugin for UrbanPlugin {
    fn process_type(&self) -> &str { "urban" }
    async fn execute(&self, p: serde_json::Value) -> GeoResult<serde_json::Value> {
        // 调用现有方法
        Ok(serde_json::json!({"far": self.far(...)}))
    }
}
```

所有 10 个插件均已实现此 trait，可通过 `registry.dispatch("urban", args)` 统一调用。
