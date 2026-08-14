# geo-toolbox 使用指南

> API 参考、插件开发、适配器集成、常见问题。
> 架构与编译指南见 [README](README.md)。

---

## 1. 核心功能使用

### 1.1 CRS 坐标变换

```rust
use geo_core::crs::CrsRegistry;

let reg = CrsRegistry::new();
let (x, y) = reg.transform_point(4326, 3857, 104.06, 30.57).unwrap();
```

### 1.2 碳核算

```rust
use geo_carbon_math::{CarbonEngine, EmissionFactor, GeoFeature};

let engine = CarbonEngine::new();
let factors = vec![EmissionFactor::new("forest", -5.0, "IPCC_2019")];
let features = vec![GeoFeature::new("forest", geojson_str).unwrap()];
let report = engine.calculate(&features, &factors, 2025).unwrap();
```

#### 1.2.1 5 碳库模型

IPCC 全五池 (AGB/BGB/Deadwood/Litter/SOC):

```rust
let stock = engine.calculate_pool_stock(
    10.0, 200.0,
    &BiomassParams::temperate_broadleaf(),
    &SocParams::native_forest(70.0),
);
```

公式: AGB = V × WD × BEF × CF × 44/12; BGB = AGB × R

#### 1.2.2 VCS/CCB 方法学映射

`vcs_gs.rs` 实现 9 种方法学:
VM0010 (IFM)、VM0015 (Afforestation)、VM0024 (Wetlands)、VM0026 (Grassland)、VM0032 (REDD+)、VM0033 (Tidal Wetland)、VM0034 (Peatland)、VM0036 (Agroforestry)、VM0046 (Blue Carbon)

### 1.3 遥感辐射校正与 InSAR

TOA 辐射亮度/反射率、DOS 大气校正、云检测、Goldstein 相位解缠、LOS 形变估计。

```rust
use geo_plugin_remote_sensing::{full_radiometric_pipeline, full_insar_pipeline};
```

### 1.4 RUSLE 土壤流失方程

A = R · K · LS · C · P — 5 因子完整计算，侵蚀等级分类。

### 1.5 SCS-CN 径流曲线数

26 种土地利用 CN 查表，AMC 干旱/正常/湿润修正。

### 1.6 InVEST 碳存储与水源涵养

4 碳库评估 + Budyko 蒸散发曲线产水量。

### 1.7 气象气候

GCM 降尺度 (Delta + 分位数映射)、IDF 曲线 (Sherman)、干旱指数 (SPI/SPEI/PDSI)、Kriging 插值。

### 1.8 地貌分析

D8 流向累积 + Strahler 河网 + 河谷断面。

### 1.9 地下水模块

达西定律、Cooper-Jacob/Theis 抽水试验、MODFLOW 适配。

### 1.10 单位线汇流

SCS 三角单位线、Snyder 合成单位线、Nash IUH。

### 1.11 波浪爬高

Stockdon / Mase 公式、越浪量计算。

### 1.12 海洋物理

Ekman 输运、SWAN 波浪变形、潮汐调和分析。

### 1.13 土壤模块

USDA 质地三角分类、van Genuchten 参数、HWSD 数据库。

### 1.14 坐标转换

四参数/七参数 Helmert、仿射变换、椭球识别。

### 1.15 MUSLE 事件版土壤流失

A = 11.8 · (Q · qp)^0.56 · K · LS · C · P

### 1.16 CLI 管道模式

```bash
geo pipeline read input.geojson | geo buffer 500 | geo write output.geojson
geo pipeline read city.geojson | geo filter key=class value=park | geo area
geo pipeline read data.geojson | geo reproject --from-epsg 4326 --to-epsg 3857 | geo write out.json
```

可用子命令: `read`, `buffer`, `simplify`, `reproject`, `write`, `area`, `filter`

---

## 2. 插件开发

### 2.1 创建插件骨架

```bash
mkdir -p plugins/geo-plugin-myplugin/src
```

**Cargo.toml:**

```toml
[package]
name = "geo-plugin-myplugin"
version.workspace = true
edition.workspace = true

[dependencies]
geo-core = { path = "../../core/geo-core" }
geo-registry = { path = "../../core/geo-registry" }
serde.workspace = true
serde_json.workspace = true
toml.workspace = true
```

### 2.2 配置

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyPluginConfig {
    pub plugin: geo_core::plugin::PluginMeta,
    pub my_param: f64,
}

impl Default for MyPluginConfig {
    fn default() -> Self {
        Self {
            plugin: geo_core::plugin::PluginMeta {
                name: "myplugin".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                description: "My plugin description".into(),
            },
            my_param: 1.0,
        }
    }
}
```

### 2.3 实现业务逻辑

```rust
pub struct MyPlugin { config: MyPluginConfig }

impl MyPlugin {
    pub fn new(config: MyPluginConfig) -> Self { Self { config } }
    pub fn do_work(&self, input: f64) -> f64 { input * self.config.my_param }
}
```

### 2.4 实现 Plugin trait

```rust
use geo_core::plugin::{Plugin, PluginCategory};

impl Plugin for MyPlugin {
    type Config = MyPluginConfig;
    fn new(config: Self::Config) -> Self { Self::new(config) }
    fn name(&self) -> &str { &self.config.plugin.name }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
    fn description(&self) -> &str { &self.config.plugin.description }
    fn category(&self) -> PluginCategory { PluginCategory::Process }
}
```

### 2.5 注册工具

```rust
// tools.rs
use geo_registry::register_plugin;
use crate::MyPlugin;

pub fn register_tools(registry: &mut geo_core::plugin::PluginRegistry) {
    register_plugin!(registry, myplugin, {
        "myplugin_do_work" => |args| {
            let input = args["input"].as_f64().unwrap_or(0.0);
            let plugin = MyPlugin::new(MyPluginConfig::default());
            Ok(serde_json::json!({"result": plugin.do_work(input)}))
        },
    });
}
```

### 2.6 注册到 workspace

在根 `Cargo.toml` 的 `[workspace].members` 添加 `"plugins/geo-plugin-myplugin"`。

---

## 3. 适配器使用

### 3.1 PostgreSQL + PostGIS

```bash
export DATABASE_URL=postgres://geo:geo@localhost/geo_test
```

```rust
use geo_adapters_geo::postgis::PostgisAdapter;
let adapter = PostgisAdapter::from_env()?;
adapter.push("features", &features).await?;
let results = adapter.pull("SELECT * FROM features").await?;
```

### 3.2 QGIS 集成

支持 Subprocess 和 REST 双后端:

```bash
export QGIS_PROCESS_PATH="E:/QGIS/bin/qgis_process.bat"
# 或 REST 模式:
export QGIS_BACKEND=rest
```

### 3.3 DuckDB 嵌入式数据库

```rust
use geo_adapters_io::duckdb::DuckDbStore;
let store = DuckDbStore::new_in_memory()?;
store.ingest_geojson("my_layer", geojson_str).await?;
```

### 3.4 STAC 影像搜索

```rust
use geo_adapters_io::stac::StacClient;
let client = StacClient::new("https://planetarycomputer.microsoft.com/api/stac/v1");
let items = client.search(bbox, "2024-01-01", "2024-06-01", 10).await?;
```

### 3.5 NMEA GPS 解析

```rust
use geo_io::nmea::parse_nmea_line;
let msg = parse_nmea_line("$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47")?;
```

---

## 4. 适配器开发

### 4.1 ExternalAdapter trait

```rust
pub trait ExternalAdapter: Plugin {
    fn external_endpoint(&self) -> &str;
    async fn health_check(&self) -> GeoResult<bool>;
    async fn external_version(&self) -> GeoResult<String>;
    fn requires_network(&self) -> bool { true }
    async fn push(&self, table: &str, data: &[GeoFeature]) -> GeoResult<u64>;
    async fn pull(&self, query: &str) -> GeoResult<Vec<GeoFeature>>;
    async fn execute(&self, command: &str, params: Value) -> GeoResult<Value>;
}
```

同时需实现 `Plugin` trait（name/version/description/category/init/shutdown）。

### 4.2 创建适配器骨架

详见 `adapters/` 下现有适配器源码作参考，推荐直接复制 `geo-adapters-io` 作为模板（含 DuckDB / STAC / OSM / CAD 示例）。

---

## 5. 测试

```bash
# 全部测试
cargo test --workspace

# 单 crate
cargo test -p geo-plugin-carbon

# 带输出
cargo test -p geo-plugin-carbon -- --nocapture

# benchmark
cargo bench --workspace
```

---

## 6. 常见问题

**Q: 首次编译报错 "找不到 gdal-sys"?**
A: 用轻量编译 `cargo build --no-default-features --features minimal`，或安装 GDAL C 库。

**Q: PostGIS 连接失败?**
A: 检查 `DATABASE_URL` 环境变量，确保 PostgreSQL 运行且已 `CREATE EXTENSION postgis`。

**Q: QGIS 工具调用报错?**
A: 确保 `QGIS_PROCESS_PATH` 指向正确的 `qgis_process` 可执行文件。

**Q: WASM 编译报错?**
A: 确保 `rustup target add wasm32-unknown-unknown` 且 `cargo install wasm-pack`。

**Q: 如何添加新插件?**
A: 参考 [§2 插件开发](#2-插件开发)，确保遵循 Core → Plugin 单向依赖。

---

## 7. HTTP API + WMS

```bash
cargo run -p geo-server --release
# http://0.0.0.0:9378
```

| 端点 | 方法 | 说明 |
|------|------|------|
| `/health` | GET | 健康检查 |
| `/api/tools` | GET | 列出所有工具 |
| `/api/call/{tool}` | POST | 调用工具 |
| `/wms` | GET/POST | OGC WMS 1.3.0 |

```bash
curl -X POST http://localhost:9378/api/call/crs_transform \
  -d '{"from_epsg":4326,"to_epsg":3857,"x":104.06,"y":30.57}'

curl "http://localhost:9378/wms?service=WMS&request=GetCapabilities"
```

## 8. 算法清单

### Core — 纯 Rust 核心

**栅格**: 坡度(Horn) · 坡向 · TPI · TRI · Hillshade · NDVI · NDWI · 波段代数 · 重采样(Nearest/Cubic/Bilinear) · Zonal统计

**矢量**: 缓冲区 · 相交 · 合并 · 差集 · 裁剪 · Douglas-Peucker简化 · KDE · 线密度

**空间统计**: Jenks · Moran's I · Gi\*热点 · IDW插值 · K-means

**时序**: Mann-Kendall · Sen's Slope · 季节性MK · Pettitt断点 · BFAST

**索引**: Geohash · STR R-tree · 四叉树

**碳核算**: IPCC 5碳库 · 造林/森林经营/毁林场景 · VCS/CCB方法学 · 蒙特卡洛不确定性

### Plugin — 专业领域

**水文**: SCS-CN产流(26种CN表) · InVEST水源涵养(Budyko) · InVEST碳存储(4碳库) · Strahler分级 · SCS三角单位线 · Snyder合成 · Nash IUH · 达西定律 · 融雪

**生态**: RUSLE(A=RKLSCP) · MUSLE暴雨侵蚀 · SDR泥沙输移 · 随机森林LULC · USDA质地 · van Genuchten · HWSD

**气候**: GCM降尺度 · IDF曲线 · SPI/SPEI/PDSI · 普通克里金

**地貌**: D8流向(填洼) · D8累积 · Strahler河网

**海岸**: Ekman输运 · SWAN波浪 · 潮汐调和 · Holland风暴潮 · Stockdon爬高 · 蓝碳

**能源**: Weibull风能 · 风机功率曲线 · Jensen尾流 · 地热(Fourier) · 输电LCP(Dijkstra)

**地灾**: 信息量模型 · Newmark位移 · 安全系数FS · 降雨ID阈值

**测绘**: GK正反算 · Helmert七参数 · 仿射变换

**林业**: 树高生长(Richards/Logistic/Korf/Gompertz/Weibull/Schumacher) · 立地指数

**遥感**: TOA辐射校正 · DOS大气校正 · 云检测 · InSAR相干性 · Goldstein相位解缠

**大气**: AOD→PM2.5反演 · 大气边界层 · 污染物扩散

**地震**: 地震动参数(GMPE) · PSHA概率危险性 · 地震活动性

**社会经济**: 设施可达性 · 土地利用变化 · 人口分布

**火山**: 火山灰扩散 · 灾害分区 · 熔岩流模拟

**地下水**: 达西定律 · Cooper-Jacob/Theis抽水试验 · 含水层参数

---

## 9. 术语表（Glossary）

### 碳核算

- **EmissionFactor** — 单个 GHG 的排放因子（值 + 单位 + GWP version）
- **CarbonEngine** — 碳核算引擎入口，features + factors → CarbonReport
- **CarbonPool** — 5 个 IPCC 碳池：AGB、BGB、Deadwood、Litter、SOC
- **BiomassParams / SocParams** — 生物量参数（WD/BEF/CF/R）/ 土壤有机碳参数（SOCref/FLU/FMG/FI）
- **EcoZone × LandUseScenario** — 4 生态区 × 4 土地利用情景的 ScenarioMatrix
- **GwpVersion** — IPCC 评估报告版本 (AR4/AR5/AR6)

### 空间运算

- **BufferMode** — Bbox（快速）/ ConvexHull（中等）/ Precise（精确）
- **RusleAssessment** — RUSLE 年均土壤流失 A=R·K·LS·C·P；**MusleResult** — 事件版 A=11.8·(Q·qp)^0.56·K·LS·C·P
- **RadiometricResult / InsarResult** — 辐射校正 / InSAR 形变结果
- **MvtEncoder/Decoder · PmtilesWriter/Reader** — MVT 瓦片、PMTiles v3 归档

### 服务与插件系统

- **WmsService / WmtsService / WfsService / WpsService / CswService** — OGC 标准服务实现
- **Plugin trait** — 插件基 trait（name/version/description/category/is_healthy）
- **ExternalAdapter** — 外部适配器 trait（push/pull/execute/health_check）
- **PluginRegistry** — 插件注册中心，管理生命周期与工具发现

