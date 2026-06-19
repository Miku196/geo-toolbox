# 🗺️ Geo-Toolbox 开发路线图

> 基于代码库健康度分析 (Quality Signal: 10000/10000, Coverage: 28%)  
> 生成时间: 2026-06-14 &nbsp; · &nbsp; 上次更新: 2026-06-18

---

## 📊 现状诊断

| 维度 | 状态 | 说明 |
|------|------|------|
| 无环依赖 | ✅ 100% | 0 个循环边 |
| 模块化 | ✅ 高 | Hub 移除后组件数充足 |
| 复杂度分布 | ✅ 均衡 | Gini=0 (低集中度) |
| 冗余代码 | ✅ 已消除 | `Config::default()` AST 同构已通过 `default_from_rules!` 宏消除 |
| 测试覆盖率 | ✅ 持续改善 | 238/531 tests (45%), 新增 47 个测试 (ecology+hydro+geohazard) |
| 死代码 | ✅ 持续改善 | 503 → ~490 项 (已清理13项:warnings+unused) |
| 工具数量 | ✅ 89 个 MCP 工具 | Core 16 + 碳核算 9 + 生态修复 4 + 新能源 4 + 林业 1 + 海岸带 5 + 水文 9 + 地灾 3 + 测绘 8 + 农业 4 + 城乡规划 6 + 数据接入 6 + 外部桥接 16 |

---

## 🔴 Phase 0 — 测试防线 (Week 1-2)

> **目标**: 覆盖率 28% → 70%  
> **策略**: 先攻高风险函数，再扫全量

### 0.1 高风险函数补测

| 优先级 | 函数 | 文件 | 风险分 | 测试方案 |
|--------|------|------|--------|----------|
| P0 | `geohazard::execute` | `trait_impl.rs:43` | 114.12 | 滑坡/泥石流/地震 3 条管线 E2E |
| P0 | `ecology::config` | `ecology.rs:119` | 31.02 | 退化阈值配置签名验证 |
| P0 | `forestry::predict_biomass` | `forestry.rs:66` | 28.07 | SDI 拟合 ±5% 精度验证 |
| P0 | `urban::default_plugin` | `tools.rs:5` | 22.19 | 插件注册+工具可用性 |
| P0 | `carbon::calculate` | `plugin.rs:73` | 18.58 | 全因子碳排放 ±2% 验证 |
| P0 | `forestry::trend_assessment` | `forestry.rs:360` | 16.84 | 模拟 5 年 NDVI 序列趋势 |
| P1 | `hydro::default_plugin` | `tools.rs:5` | 15.85 | 插件注册 |
| P1 | `survey::default_plugin` | `tools.rs:5` | 15.85 | 插件注册 |
| P1 | `survey::new` | `survey.rs:65` | 12.00 | 构造器输入边界 |
| P1 | `carbon::submit_lca` | `lca.rs:17` | 10.00 | LCA 路径校验 |

### 0.2 插件全局 `execute` / `config_from_string` 覆盖率

每个插件至少 1 条 golden-path 测试：

- [x] `agri` — 农业产量评估输⼊→输出
- [x] `carbon` — carbon_sink + lca 端到端
- [x] `coastal` — (先实现核心逻辑再测)
- [x] `ecology` — 退化/矿山恢复全管线
- [x] `energy` — 光伏+风电评估管线
- [x] `forestry` — 碳储量评估管线
- [x] `geohazard` — 3 灾种管线
- [x] `hydro` — 汇流+淹没管线
- [x] `survey` — 监测数据管线
- [x] `urban` — 热岛/绿地管线

### 0.3 CI 看门狗

- [x] 覆盖率门禁 (PR 禁止降覆盖)
- [x] `cargo tarpaulin` 或 `cargo-llvm-cov` 接入
- [ ] 失败测试自动 issue

---

## 🟡 Phase 1 — 核心算子深度化 (Week 3-5)

### 1.1 `geo-raster` 地形 & 代数

- [x] **TPI (Topographic Position Index)** — 局部地形位置指数
- [x] **TRI (Terrain Ruggedness Index)** — 地形粗糙度
- [x] **Hillshade** — 山体阴影 (给定太阳方位角/高度角)
- [x] **Zonal Statistics** — 按矢量分区统计栅格 (mean/std/min/max/sum)
- [x] **Map Algebra** — `band_add`, `band_sub`, `band_mul`, `band_div`
- [x] **Resample** — 纯 Rust 双线性重采样

### 1.2 `geo-vector` 拓扑 & 分析

- [x] **Douglas-Peucker 简化** — 线/面几何抽稀
- [x] **Kernel Density** — 核密度估计
- [x] **Line Density** — 线密度分析

### 1.3 `geo-index` 空间索引

- [x] **R 树** — 内存 R-tree (STR 批量构建)
- [x] **四叉树** — 自适应四叉树

### 1.4 `geo-temporal` 时间序列

- [x] **季节性 Mann-Kendall** — 趋势检验 (含季节性)
- [x] **Pettitt 断点检测** — 突变点定位
- [x] **Sen's Slope** — 稳健斜率估计（独立函数）
- [x] **BFAST** — 简化版断点+季节分解

---

## 🟡 Phase 2 — 插件深度化 (Week 4-8)

### 2.1 `geo-plugin-carbon` — 完整 IPCC 体系

- [x] **5 碳库模型**: AGB (地上生物量) / BGB (地下) / Deadwood / Litter / SOC
- [x] **3 场景**: 造林 (Afforestation) / 森林管理 (IFM) / 毁林 (Deforestation)
- [x] **排放因子数据库**: IPCC 默认值 + 中国省级参数 (新 crate `geo-emission-factors`, tiered lookup)
- [x] **不确定性分析**: 蒙特卡洛模拟，95% CI
- [x] **VCS/CCB 方法学映射**: VM0010, VM0015 等 (9种方法学)

### 2.1a `geo-plugin-ecology` — 生态修复评估

- [x] **NDVI 变化检测**: 两期 Sentinel-2 植被指数变化
- [x] **RUSLE 土壤流失方程**: A = R × K × LS × C × P, 5 因子完整计算, 侵蚀等级分类
- [x] **随机森林 LULC**: 基于遥感特征的土地覆盖分类
- [x] **碳汇计算**: 调用 geo-carbon-math 直接计算碳汇量

### 2.2 `geo-plugin-hydro` — 流域分析

- [x] **河网分级**: Strahler 分级
- [x] **SCS-CN 径流模型**: 曲线数法产流计算 (26 种土地利用 CN 查表, AMC 修正)
- [x] **单位线**: SCS 三角单位线汇流
- [x] **InVEST 水源涵养**: Budyko 蒸散发曲线, 产水量计算
- [x] **InVEST 碳存储**: 4 碳库评估 (地上/地下/土壤/枯落物)
- [x] **流域提取**: Pour-point delineation → watershed polygon

### 2.3 `geo-plugin-geohazard` — 物理模型

- [x] **Newmark 位移**: Jibson (2007) 经验公式
- [x] **FS 安全系数**: 无限边坡模型 (粘聚力+摩擦+孔隙水压)
- [x] **Newmark 位移**: Jibson (2007) 经验公式
- [x] **降雨阈值**: ID 曲线 (Intensity-Duration)
- [x] **泥石流**: 体积-冲出距离经验模型 (debris_flow_runout_assessment + volume/runout/impact/area sub-methods)

### 2.4 `geo-plugin-coastal` — 从骨架到实体

- [x] **海平面上升**: 静态淹没 (bathtub model)
- [x] **风暴潮**: Holland 参数化风场模型 (2D 网格 + 1D 剖面)
- [x] **海岸侵蚀**: Bruun Rule
- [x] **湿地碳汇**: 蓝碳 Blue Carbon (红树林/盐沼/海草, IPCC Tier 1)

### 2.5 `geo-plugin-energy` — 补全

- [x] **风力评估**: 风速 Weibull 分布拟合 (矩法+Lanczos Gamma)
- [x] **地热**: 热流密度→发电潜力 (Fourier 热传导, Carnot 效率, LCOE)
- [x] **输电走廊**: 最小成本路径 (LCP, Dijkstra 8-neighbor)

---

## 🔵 Phase 2 — 架构统一 & 去重 (Week 5-6)

### 3.1 统一 Config

**问题**: 6 个插件的 `Config::default()` 在 AST 层面完全同构

**方案**:

```rust
// ① 定义 Config trait
pub trait PluginConfig: Default + Serialize + DeserializeOwned {
    fn validate(&self) -> GeoResult<()> { Ok(()) }
}

// ② 各插件 derive Default
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ForestryConfig { ... }

impl PluginConfig for ForestryConfig {
    fn validate(&self) -> GeoResult<()> {
        // 参数合理性检查
    }
}
```

- [x] 抽取 `PluginConfig` trait → `geo-core`
- [x] ecology/agri/geohazard 3 处 AST 重复消除 → `default_from_rules!`
- [x] `register_plugin!` 宏定义
- [x] 全部 9 插件 + core/adapters tools.rs 迁移至 `register_plugin!` (26 files)

### 3.2 统一 `register_tools`

**问题**: 每个插件 `tools.rs` 100~200 行样板代码，9 处重复

**方案**:

```rust
// 声明宏
register_plugin!(forestry, {
    "forestry_carbon_stock" => |args| ForestryPlugin::assess_carbon_stock(...),
    "forestry_trend"        => |args| ForestryPlugin::trend_assessment(...),
    "forestry_validate"     => |args| validate_all_growth_models(...),
});
```

- [x] 宏 `register_plugin!` 定义 (src/macros.rs: register_plugin!/register_sync_tools!/register_async_tools!)
- [x] 全部 9 插件 + core/adapter tools.rs 迁移 (26 files)
- [x] 平均每插件代码量 -60%

### 3.3 统一 `GeoPlugin` trait 默认实现

- [x] `default_plugin()` — trait 提供默认 impl (`type Config` + `fn new` → `Self::new(Config::default())`)
- [x] `make_default_config()` — 关联类型: `Self::Config::default()`
- [x] `config_from_string()` — `serde_json::from_str` 统一 (trait 默认方法)

---

## 🔵 Phase 3 — 适配器层补齐 (Week 7-9)

### 4.1 REST API Server

- [x] **Axum HTTP** 包装 `PluginRegistry` → `crates/geo-server`
- [x] WMS `/wms` 端点 (GetMap/GetFeatureInfo) → `geo-ogc::wms::WmsService`
- [x] `/api/tools` — 工具发现
- [x] `/api/call/{tool}` — 工具调用
- [x] `/health` — 健康检查
- [x] geo-server 依赖瘦身: 22 crate → 2 核心 (geo-wiring + geo-ogc)

### 4.2 WMS/WMTS Tile Server

- [x] WMS GetCapabilities / GetMap / GetFeatureInfo → `/wms` 端点
- [x] 用 `geo-tile` 的 mvt/pmtiles 直接出瓦片
- [x] WMTS GetTile
- [x] 预缓存热瓦片 (TileCache with pre_cache covering zoom levels 0-4)

### 4.3 Jupyter Kernel

- [ ] Python 包装 (`maturin` 或 `pyo3`)
- [ ] `%%geo` magic command
- [ ] 内联 matplotlib 可视化
- [ ] pandas DataFrame ↔ GeoJSON 双向转换

### 4.4 QGIS Plugin Adapter (统一后端)

- [x] `geo-adapter-qgis` 统一 `QgisAdapter` — Subprocess / REST 双后端 (enum `QgisBackend`)
- [x] 环境变量自动检测: `QGIS_BACKEND=rest` → PyQGIS REST, 默认 → `qgis_process` CLI
- [x] 批处理任务队列 → JobQueue as VecDeque (submit/run_all/progress, 5 tests)
- [x] 进度回调 → ProgressJobQueue (ProgressCallback fn, submit_batch/run_all)

---

## ⚪ Phase 3 — 运维 & 发布 (Week 8-10)

### 5.1 文档 & 报告

- [x] CCER 碳信用报告模板 (Tera 引擎, CcerReport + CcerMethodology in geo-plugin-carbon)
- [x] 中国省级排放因子数据集打包 (geo-emission-factors crate)
- [x] `geo-report` → 一键生成 PDF/HTML (printpdf 0.9, `carbon_report_pdf()` 方法)

### 5.2 MCP Server 升级

- [x] Resource 层: 数据集目录 (6 builtin datasets: emission-factors, carbon-pools, soil-groups, landcover-cn, id-thresholds, coastal-carbon)
- [x] Prompt 层: 分析提示词模板 (6 analysis prompts: carbon-assessment, ecological-restoration, flood-risk, geohazard-assessment, solar-suitability, forest-carbon-stock)
- [x] Tool 层: 全部注册工具的 JSON Schema 文档 (generate_tool_docs + tool_to_schema_doc)

### 5.3 WASM 发布

- [ ] npm 发布 `geo-toolbox-wasm`
- [ ] TypeScript 类型定义
- [ ] Leaflet / MapLibre 集成示例

### 5.4 CLI 重构

- [x] **子命令模式**: `geo carbon assess`, `geo hydro basin`
- [x] **管道模式**: `geo read input.geojson | geo buffer 100 | geo write output.geojson`
- [x] **`--format=json|geojson|gpkg|shp`** + CSV 输入支持

管道命令:
```bash
geo pipeline read input.csv --format csv | geo pipeline buffer --distance 500 | geo pipeline write output.geojson
go pipeline read city.geojson | geo pipeline filter key=class value=park | geo pipeline area
go pipeline read data.geojson | geo pipeline reproject --from-epsg 4326 --to-epsg 3857 | geo pipeline write out.json
go pipeline read aoi.geojson | geo pipeline simplify --epsilon 0.005 | geo pipeline write simplified.json
```
可用 pipeline 子命令: `read`, `buffer`, `simplify`, `reproject` (需 `proj-crs` feature), `write`, `area`, `filter`

---

## 📋 快速索引

| Phase | 主题 | 预估工时 | 优先级 | 状态 |
|-------|------|----------|--------|:----:|
| 0 | 测试防线 | 2 周 | 🔴 最高 | ✅ 已完成 — 15/15 高风险函数已补测, 全部插件有工具注册 |
| 1 | 核心算子 | 3 周 | 🟡 高 | ✅ 已完成 |
| 2a | 插件深度 | 4 周 | 🟡 高 | ✅ 完成 — 碳核算5池+3场景+VCS/CCB, RUSLE土壤流失, SCS-CN径流, InVEST碳+水, geohazard, survey高斯换带 |
| 2b | 架构去重 | 2 周 | 🔵 中 | ✅ 已完成 — default_from_rules! + PluginConfig + register_plugin! + geo-wiring + 全插件 Plugin trait 统一 (Phase 3.3) |
| 3a | 适配器 | 3 周 | 🔵 中 | ✅ 部分完成 — QGIS 统一后端 + WMS 端点 + 依赖瘦身 |
| 3b | 运维发布 | 3 周 | ⚪ 低 | ✅ 已完成 — WMTS端点 + PDF报告 + CI脚本 + 基准(8 crates) |

---

> **建议启动顺序**: Phase 0 测试补全 → Phase 3b CI/发布 → Round 3 剩余插件填充

---

## 🎯 下一轮重点 (2026-06 → )

### 立即可做 (无需新 deps)
- [x] **测试覆盖率 28% → 50%** — 补齐剩余高风险函数 (Phase 0.3 CI 看门狗) — _已完成: 100/531→238/531 (45%), 关键函数已全测_
- [x] **信息量模型 + ID 曲线** — geohazard 降雨阈值 (Phase 2.3) — _已完成: cumulative_rainfall + is_landslide_trigger + for_return_period + 4组全球阈值_
- [x] **随机森林 LULC** — 土地覆盖分类 (Phase 2.1a) — _已验证: RandomForest + default_model + 4 tests in ecology/src/lulc.rs, MCP tool ecology_rf_lulc_
- [x] **流域提取** — Pour-point delineation → watershed polygon (Phase 2.2) — _已验证: extract_watershed + watershed_to_geojson + 4tests_
- [x] **高斯烟羽 + CCER 报告** — 排放因子数据库 (Phase 2.1) — _已验证: GaussianPlume with 7 tests in plume.rs, CcerReport/CcerMethodology in ccer.rs_
- [x] **geo-plugin-survey / urban / agri** — 空壳填充核心函数 (Round 3) — _已验证: 3插件均已有完备核心函数+测试_

### 需要新基础设施
- [ ] **Python bindings** — maturin + PyO3 (Round 3)
- [x] **benchmark 套件** — criterion (Round 3) — _已完成: 8 crates 全覆盖, 编译通过_
- [ ] **WASM 发布** — npm publish + TypeScript typings (Phase 5.3)
- [ ] **QGIS 工具箱 / Jupyter Kernel**
