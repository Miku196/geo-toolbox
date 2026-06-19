# Geo-Toolbox 领域词汇表

> 地理空间分析工具箱 — 支持碳核算、矢量分析、栅格处理、瓦片服务。
> 架构：Core 层 (无业务语义) → Plugin 层 (业务领域逻辑) → Adapter 层 (外部系统集成)。

## 架构层级

### Core 层 — 基础能力
| 模块 | 职责 |
|------|------|
| `geo-core` | 跨 crate 共享类型：`GeoResult<T>`, `GeoFeature`, `BBox`, `Plugin` trait, `ExternalAdapter` trait, `PluginRegistry` |
| `geo-carbon-math` | IPCC 碳核算引擎：排放因子解析、5-pool 碳汇模型、情景计算、VCS 方法学匹配 |
| `geo-vector` | 矢量几何运算：buffer、intersect、clip、simplify、KDE、line density |
| `geo-raster` | 栅格处理：波段读取/写入、地形分析（slope/aspect flow accumulation） |
| `geo-tile` | 瓦片数学 + MVT 编码/解码 + PMTiles v3 格式 |
| `geo-io` | 多种格式导入：NMEA、Camofox、DXF、CSV |
| `geo-index` | 空间索引：Geohash、QuadTree |
| `geo-stats` | 空间统计：Zonal stats、Moran's I、热点分析 |
| `geo-temporal` | 时间序列分析：Mann-Kendall 趋势、Sen's slope、STL 分解 |
| `geo-report` | 报告生成引擎（Tera 模板 + Markdown） |
| `geo-ogc` | OGC 标准协议实现：WMS、WMTS、WFS、WPS、CSW |
| `geo-parquet` | GeoParquet 格式读写 |
| `geo-emission-factors` | 碳排放因子数据集 |

### Plugin 层 — 业务插件
| 模块 | 领域 |
|------|------|
| `geo-plugin-carbon` | 碳核算流程编排：LCA、碳盘查、GWP 计算 |
| `geo-plugin-forestry` | 林业碳汇：生物量参数、生长模型 |
| `geo-plugin-energy` | 可再生能源选址：太阳辐射、风速分析 |
| `geo-plugin-geohazard` | 地质灾害评估：滑坡、泥石流风险 |
| `geo-plugin-ecology` | 生态评估 |
| `geo-plugin-survey` | 测量测绘 |
| `geo-plugin-urban` | 城市规划 |
| `geo-plugin-hydro` | 水文分析 |
| `geo-plugin-agri` | 农业 |
| `geo-plugin-coastal` | 海岸带 |

### Adapter 层 — 外部系统
| 模块 | 集成对象 |
|------|----------|
| `geo-adapter-duckdb` | SQLite 空间存储引擎 |
| `geo-adapter-postgis` | PostGIS 数据库适配器 |
| `geo-adapter-cli` | GDAL/OGR CLI 封装 |
| `geo-adapter-qgis` | QGIS Processing Toolbox 封装 |
| `geo-adapter-iot` | MQTT IoT 传感器集成 |

### Crate 层 — 可交付制品
| 模块 | 制品类型 |
|------|----------|
| `geo-cli` | CLI 二进制 |
| `geo-wasm` | WASM 浏览器包 |
| `geo-server` | HTTP 服务器 |
| `geo-wiring` | DI/wiring 组合根 |
| `bindings/python` | PyO3 Python 包 |

## 关键概念

### 碳核算 (Carbon Accounting)
- **EmissionFactor** — 单个 GHG 的排放因子（值 + 单位 + GWP version）
- **ActivityRecord** — 活动数据记录（category + quantity + unit + scope）
- **CarbonEngine** — 碳核算引擎入口，接收 features + factors → 输出 CarbonReport
- **CarbonReport** — 碳报告（按 scope/fuel/category 汇总的 tCO₂e）
- **ScenarioInput / ScenarioResult** — 土地利用变化情景的输入/输出
- **MultiPoolChange** — 5-pool 碳储量变化结果
- **GwpVersion** — IPCC 评估报告版本 (AR4/AR5/AR6)

### 碳汇模型 (Carbon Pool Model)
- **CarbonPool** — 5 个 IPCC 碳池：AGB、BGB、Deadwood、Litter、SOC
- **BiomassParams** — 生物量参数：wood_density, BEF, carbon_fraction, root_shoot_ratio, deadwood_ratio, litter_ratio
- **SocParams** — 土壤有机碳参数：soc_ref, FLU, FMG, FI
- **PoolStock** — 单个碳池在某时间点的储量（tCO₂e/ha）
- **PoolChange** — 单个碳池在两个时间点之间的变化量
- **EcoZone** — IPCC 生态区划 (TropicalMoist / TemperateConiferous / TemperateBroadleaf / Boreal)
- **LandUseScenario** — 土地利用情景 (NativeForest / DegradedCropland / AfforestationCropland / DeforestationCropland)

### 矢量运算 (Vector Operations)
- **BufferMode** — 缓冲区模式：Bbox（快速）/ ConvexHull（中等）/ Precise（精确）
- **buffer** — 多边形缓冲区分析
- **simplify / simplify_line** — 线/多边形简化（Douglas-Peucker）
- **kernel_density / line_density** — 点/线核密度估计
- **intersect / union_all / clip / difference / sym_difference** — 布尔运算

### 瓦片 (Tiles)
- **TileSource** — 瓦片源枚举 (OpenStreetMap / Gaode / TianDiTu)
- **MvtEncoder / MvtDecoder** — MVT (Mapbox Vector Tile) 编码/解码
- **PmtilesWriter / PmtilesReader** — PMTiles v3 归档格式
- **WmtsRequest** — WMTS 请求类型 (GetCapabilities / GetTile / GetFeatureInfo)

### OGC 标准
- **WmsService** — WMS 1.3.0 服务 (GetCapabilities / GetMap / GetFeatureInfo)
- **WmtsService** — WMTS 1.0.0 服务 (GetCapabilities / GetTile / GetFeatureInfo)
- **WfsService** — WFS 2.0.0 服务 (GetCapabilities / GetFeature / DescribeFeatureType)
- **WpsService** — WPS 1.0.0 服务 (GetCapabilities / DescribeProcess / Execute)
- **CswService** — CSW 2.0.2 目录服务

### 注册与插件系统
- **Plugin trait** — 所有插件的基 trait (name/version/description/category/is_healthy)
- **ExternalAdapter** — 外部系统适配器 trait (push/pull/execute/health_check)
- **PluginRegistry** — 插件注册中心，管理插件生命周期和工具发现
- **register_tools()** — 每个模块手工注册工具的入口函数

### 可观测性 (Observability)
- **tracing crate** — 全项目使用 tokio tracing 而非 `log` 宏
- **结构化字段** — 使用 `tracing::info!(field = value, "msg")` 而非 `tracing::info!("msg {interpolation}")`
- **统一 key 命名** — path / table / count / latency_ms / error / bbox / crs / source / bytes
- **span 自动注入** — 关键入口方法使用 `#[tracing::instrument]`
- **init_default_subscriber()** — `geo-core::observability` 提供标准初始化器
- **GEO_LOG_FORMAT=json** — 生产环境切换 JSON 输出
- **TracingContext** — 包含 trace_id、parent_span_id，贯穿整个调用链。geo-server 响应头返回 X-Trace-Id

### 系统韧性 (Resilience)
- **ResourceGuard** — 输入大小/分辨率/要素数量限制 (`geo-core::guard`)。默认 50MB / 100万要素 / 10k²栅格
- **CachedHealth** — 带 TTL 缓存的健康状态探针 (`geo-core::health`)。调用方后台定期 ping，is_ok() 返回 O(1) 缓存值
- **BlockingPool** — CPU 密集型算法（STL 分解、Mann-Kendall、栅格卷积）应通过 `tokio::task::spawn_blocking` 提交到独立线程池，避免阻塞异步 I/O 运行时
- **ScenarioMatrix** — 单选入口查找 IPCC 参数 (`geo-carbon-math::pools::scenario_matrix`)。覆盖 4 EcoZone × 4 LandUseScenario，禁止枚举

## 架构原则

1. **Core 层不依赖 Plugin/Adapter 层** — Core 是纯基础能力
2. **通过 PluginRegistry 发现工具** — 插件注册后在 CLI / Server / WASM 中统一调用
3. **插件使用 trait 接口隔离** — 每个 plugin 通过 Plugin trait 暴露能力
4. **Adapter 使用 trait 接口** — ExternalAdapter 统一 push/pull/execute
5. **深度优先于广度** — 优先让一个模块做深做透，而非分散的浅封装
6. **结构化可观测性** — 所有 tracing 使用结构化字段，入口方法标注 `#[instrument]`
