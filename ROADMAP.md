# 🗺️ Geo-Toolbox 开发路线图

> 基于代码库健康度分析 (Quality Signal: 10000/10000, Coverage: 28%)  
> 生成时间: 2026-06-14

---

## 📊 现状诊断

| 维度 | 状态 | 说明 |
|------|------|------|
| 无环依赖 | ✅ 100% | 0 个循环边 |
| 模块化 | ✅ 高 | Hub 移除后组件数充足 |
| 复杂度分布 | ✅ 均衡 | Gini=0 (低集中度) |
| 冗余代码 | ⚠️ | Config::default() 6 处 AST 同构 |
| 测试覆盖率 | ❌ 28% | 296 函数中仅 82 有测试 |
| 死代码 | ⚠️ | 503 项 (含 WASM/py 辅助函数) |

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

- [ ] `agri` — 农业产量评估输⼊→输出
- [ ] `carbon` — carbon_sink + lca 端到端
- [ ] `coastal` — (先实现核心逻辑再测)
- [ ] `ecology` — 退化/矿山恢复全管线
- [ ] `energy` — 光伏+风电评估管线
- [ ] `forestry` — 碳储量评估管线
- [ ] `geohazard` — 3 灾种管线
- [ ] `hydro` — 汇流+淹没管线
- [ ] `survey` — 监测数据管线
- [ ] `urban` — 热岛/绿地管线

### 0.3 CI 看门狗

- [ ] 覆盖率门禁 (PR 禁止降覆盖)
- [ ] `cargo tarpaulin` 或 `cargo-llvm-cov` 接入
- [ ] 失败测试自动 issue

---

## 🟡 Phase 1 — 核心算子深度化 (Week 3-5)

### 1.1 `geo-raster` 地形 & 代数

- [ ] **TPI (Topographic Position Index)** — 局部地形位置指数
- [ ] **TRI (Terrain Ruggedness Index)** — 地形粗糙度
- [ ] **Hillshade** — 山体阴影 (给定太阳方位角/高度角)
- [ ] **Zonal Statistics** — 按矢量分区统计栅格 (mean/std/min/max/sum)
- [ ] **Map Algebra** — `band_add`, `band_sub`, `band_mul`, `band_div` (已有 `band.rs` 骨架，需补 `mul`)
- [ ] **Resample** — 纯 Rust 双线性/三次卷积重采样

### 1.2 `geo-vector` 拓扑 & 分析

- [ ] **Douglas-Peucker 简化** — 线/面几何抽稀
- [ ] **拓扑修复** — `make_valid()` (自相交分解、环向修正)
- [ ] **Voronoi 图** — 点集泰森多边形
- [ ] **Delaunay 三角网** — TIN 构建
- [ ] **内核密度** — Kernel Density Estimation
- [ ] **线密度** — Line Density

### 1.3 `geo-index` 空间索引

- [ ] **S2 单元** — Google S2 层级编码
- [ ] **H3 六边形** — Uber H3 全球格网
- [ ] **R 树** — 内存 R-tree (rstar crate 集成)
- [ ] **四叉树** — 自适应四叉树

### 1.4 `geo-temporal` 时间序列

- [ ] **季节性 Mann-Kendall** — 趋势检验 (含季节性)
- [ ] **Pettitt 断点检测** — 突变点定位
- [ ] **Sen's Slope** — 稳健斜率估计
- [ ] **BFAST** — 断点+季节分解一体化

---

## 🟡 Phase 2 — 插件深度化 (Week 4-8)

### 2.1 `geo-plugin-carbon` — 完整 IPCC 体系

- [ ] **5 碳库模型**: AGB (地上生物量) / BGB (地下) / Deadwood / Litter / SOC
- [ ] **3 场景**: 造林 (Afforestation) / 森林管理 (IFM) / 毁林 (Deforestation)
- [ ] **排放因子数据库**: IPCC 默认值 + 中国省级参数
- [ ] **不确定性分析**: 蒙特卡洛模拟，95% CI
- [ ] **VCS/CCB 方法学映射**: VM0010, VM0015 等

### 2.2 `geo-plugin-hydro` — 流域分析

- [ ] **流域提取**: Pour-point delineation → watershed polygon
- [ ] **河网分级**: Strahler / Shreve 分级
- [ ] **SCS-CN 径流模型**: 曲线数法产流计算
- [ ] **单位线**: Snyder / SCS 单位线汇流

### 2.3 `geo-plugin-geohazard` — 物理模型

- [ ] **Newmark 位移**: 地震滑坡永久位移
- [ ] **FS 安全系数**: 无限边坡模型
- [ ] **降雨阈值**: ID 曲线 (Intensity-Duration)
- [ ] **泥石流**: 体积-冲出距离经验模型

### 2.4 `geo-plugin-coastal` — 从骨架到实体

- [ ] **海平面上升**: 静态淹没 (bathtub model)
- [ ] **风暴潮**: SLOSH 简化模型
- [ ] **海岸侵蚀**: Bruun Rule
- [ ] **湿地碳汇**: 蓝碳 (Blue Carbon) 计量

### 2.5 `geo-plugin-energy` — 补全

- [ ] **风力评估**: 风速 Weibull 分布拟合
- [ ] **地热**: 热流密度→发电潜力
- [ ] **输电走廊**: 最小成本路径 (LCP)

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

- [ ] 抽取 `PluginConfig` trait → `geo-core`
- [ ] ecology/agri/urban/geohazard/hydro/survey 全部迁移
- [ ] 消除 6 处 AST 重复

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

- [ ] 宏 `register_plugin!` 定义
- [ ] 全部 9 插件迁移
- [ ] 平均每插件代码量 -60%

### 3.3 统一 `GeoPlugin` trait 默认实现

- [ ] `default_plugin()` — trait 提供默认 impl
- [ ] `make_default_config()` — Config trait 关联类型
- [ ] `config_from_string()` — `toml::from_str` 统一

---

## 🔵 Phase 3 — 适配器层补齐 (Week 7-9)

### 4.1 REST API Server

- [ ] **Axum 或 Actix** HTTP 包装 `PluginRegistry`
- [ ] OpenAPI / Swagger 自生成 (utoipa)
- [ ] `/api/v1/tools` — 工具发现
- [ ] `/api/v1/execute` — 工具调用
- [ ] `/api/v1/health` — 健康检查

### 4.2 WMS/WMTS Tile Server

- [ ] 用 `geo-tile` 的 mvt/pmtiles 直接出瓦片
- [ ] WMS GetCapabilities / GetMap
- [ ] WMTS GetTile
- [ ] 预缓存热瓦片

### 4.3 Jupyter Kernel

- [ ] Python 包装 (`maturin` 或 `pyo3`)
- [ ] `%%geo` magic command
- [ ] 内联 matplotlib 可视化
- [ ] pandas DataFrame ↔ GeoJSON 双向转换

### 4.4 QGIS Plugin Adapter (兑现已有骨架)

- [ ] `geo-adapter-qgis` gRPC → 实际 QGIS 处理引擎
- [ ] 批处理任务队列
- [ ] 进度回调

---

## ⚪ Phase 3 — 运维 & 发布 (Week 8-10)

### 5.1 文档 & 报告

- [ ] CCER 碳信用报告模板 (Tera 引擎)
- [ ] 中国省级排放因子数据集打包
- [ ] `geo-report` → 一键生成 PDF/HTML

### 5.2 MCP Server 升级

- [ ] Resource 层: 数据集目录 (STAC 兼容)
- [ ] Prompt 层: 分析提示词模板
- [ ] Tool 层: 全部注册工具的 JSON Schema 文档

### 5.3 WASM 发布

- [ ] npm 发布 `geo-toolbox-wasm`
- [ ] TypeScript 类型定义
- [ ] Leaflet / MapLibre 集成示例

### 5.4 CLI 重构

- [ ] 子命令模式: `geo carbon assess`, `geo hydro basin`
- [ ] 管道模式: `geo read input.geojson | geo buffer 100 | geo write output.geojson`
- [ ] `--format=json|geojson|gpkg|shp`

---

## 📋 快速索引

| Phase | 主题 | 预估工时 | 优先级 |
|-------|------|----------|--------|
| 0 | 测试防线 | 2 周 | 🔴 最高 |
| 1 | 核心算子 | 3 周 | 🟡 高 |
| 2a | 插件深度 | 4 周 | 🟡 高 |
| 2b | 架构去重 | 2 周 | 🔵 中 |
| 3a | 适配器 | 3 周 | 🔵 中 |
| 3b | 运维发布 | 3 周 | ⚪ 低 |

---

> **建议启动顺序**: Phase 0 测试 → Phase 1 算子 (与 2b 架构去重并行) → Phase 2a 插件深度 → Phase 3
