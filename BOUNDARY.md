# Geo-Toolbox 基座边界（BOUNDARY）

> 本文档定义 geo-toolbox 主仓库（基座）的仓库边界、与 split/ 分仓库的对应关系、
> 制品层在基座中的角色，以及后续同步机制。
>
> 记录时间：2026-06 基座收口任务（步骤 3）· 状态：**决策已定，代码未删**

---

## 1. 基座包含什么

geo-toolbox（`D:\geo\geo-toolbox`）是**唯一主仓库**，采用五层洋葱架构，workspace 共约 40 个 crate：

| 层 | 目录 | 内容 | 说明 |
|----|------|------|------|
| Layer 1 Core | `core/` | geo-core, geo-facade, geo-carbon-math, geo-raster, geo-stats, geo-io, geo-vector, geo-index, geo-report, geo-emission-factors, geo-parquet, geo-ogc, geo-tile, geo-temporal, geo-registry | 纯 Rust，零外部硬依赖；含抽象 trait 与 `PluginRegistry` |
| Layer 2 Facade | `core/geo-facade` | 高频函数聚合门面 | 归入 core/ 目录 |
| Layer 3 Plugins | `plugins/` | geo-plugin-carbon, ecology, survey, urban, hydro, geohazard, agri, energy, forestry, coastal, climate, geomorph, remote-sensing, seismology, socioeconomic, atmosphere, volcanology, groundwater | 专业领域插件 |
| Layer 4 Wiring | `crates/geo-wiring` | DI 依赖注入组合根 | 唯一允许同时依赖 Plugin 和 Adapter 的工厂 |
| Layer 5 Adapters | `adapters/` | geo-adapters-geo, geo-adapters-io, geo-adapters-sim, geo-adapter-qgis, geo-adapter-pygeoapi | 外部桥接，Feature-gated |
| 制品（开发验证） | `crates/` + `bindings/` | geo-cli, geo-wasm, geo-server, bindings/python | 见第 3 节 |
| 示例/工具 | `examples/`, `apps/`, `contrib/`, `fuzz/`, `scripts/`, `docs/` | dexing-copper 等示例、field-pwa、归档插件、fuzz 目标、CI 脚本 | 辅助内容 |

依赖方向严格单向：Core ← Facade ← Plugin ← Wiring ← Adapter。

---

## 2. 基座 ↔ split/ 分仓库对应关系

split/ 分仓库位于 **`D:\geo\split\`**（与主仓库同级，不在 geo-toolbox 内部），共 4 个分仓库，是**发布形态**。

| 基座路径 | split/ 分仓库 | 仓库内容 | 角色 |
|----------|---------------|----------|------|
| `core/**`, `plugins/**`, `examples/**` | `split/geo-toolbox-core` | Core 15 crates + Plugins 18 crates + examples（chengdu-carbon / china-risk-assessment / dexing-copper / maplibre-carbon） | 核心引擎 + 领域插件分仓库 |
| `crates/geo-cli`, `crates/geo-server`, `crates/geo-wiring`, `adapters/**` | `split/geo-toolbox-agent` | CLI + Server + Wiring + 全部 5 个 Adapter | Agent 侧分仓库（服务端/CLI 制品） |
| `crates/geo-wasm` | `split/geo-toolbox-web` | WASM 浏览器包 | Web 侧分仓库 |
| `crates/geo-cli`（裁剪版，仅 CLI） | `split/geo-toolbox-edge` | 单 crate：geo-cli | 边缘/端侧分仓库 |
| `bindings/python` | **（无对应分仓库）** | PyO3 Python 包 | ⚠️ 目前仅存在于基座，见第 4 节 |

> 注意：`bindings/python`、`bindings/jupyter`、`bindings/qgis`、`bindings/maplibre-gl-geo-toolbox`、
> `apps/field-pwa` 当前**没有**对应 split 分仓库，属于基座独有内容。

---

## 3. 制品层在基座中的角色（边界决策）

**决策（2026-06，基座收口）：基座保留制品 crate —— `crates/geo-cli`、`crates/geo-wasm`、
`crates/geo-server`、`bindings/python` —— 作为开发验证与参考实现，不删除。**

理由：

1. **主仓库自身测试/CI 依赖它们**：`cargo test --workspace`、`cargo check --workspace`、
   ROADMAP 中的回归门槛（1001 tests, 0 warning）都以 workspace 全绿为前提，制品 crate 是全链路
   （Core → Plugin → Wiring → CLI/Server/WASM/Python）的集成验证入口。
2. **制品是五层架构的消费端**：geo-cli / geo-server / geo-wasm / bindings/python 是
   Wiring 组合根之上的"第六层"（入口层），基座没有它们就无法端到端验证依赖方向与 DI 装配。
3. **split/ 分仓库是发布形态**：分仓库（core / agent / web / edge）承担对外发布职责
   （crates.io / npm / PyPI / 部署包），基座内的制品是"同一代码的开发副本"，不参与发布。
4. **不删代码**：本任务只记录边界决策，不删除/不移动任何文件（约束：保留全部未提交改动）。

制品在基座中的职责定位：

| 制品 | 基座内职责 | 发布形态（split） |
|------|-----------|------------------|
| `crates/geo-cli` | CLI 入口 + 管道模式 + Agent execute；主仓库批处理验证 | geo-toolbox-agent / geo-toolbox-edge |
| `crates/geo-wasm` | WASM 浏览器包（CrsEngine / CarbonEngine 等） | geo-toolbox-web |
| `crates/geo-server` | Axum HTTP + WMS/WMTS 瓦片服务 | geo-toolbox-agent |
| `bindings/python` | PyO3 Python 包 | （暂无分仓库） |
| `crates/geo-wiring` | DI 组合根（严格说属于 Wiring 层，但目录在 crates/） | geo-toolbox-agent |

---

## 4. 后续如何同步（基座 → split/）

同步方向：**基座为源，split/ 为镜像副本**。主仓库的提交（如 18ca355 等）在 split 各仓库
有对应提交（git log 同主题），说明历史上通过"代码复制 + 独立提交"方式同步。

建议的同步机制（记录，不实施）：

1. **改动走基座**：所有开发在 `D:\geo\geo-toolbox` 进行，cargo check/test 在基座验证。
2. **按目录复制到分仓库**：
   - core/ + plugins/ + examples/ → `split/geo-toolbox-core`
   - crates/geo-cli + crates/geo-server + crates/geo-wiring + adapters/ → `split/geo-toolbox-agent`
   - crates/geo-wasm → `split/geo-toolbox-web`
   - crates/geo-cli（裁剪 Cargo.toml，仅 CLI 成员）→ `split/geo-toolbox-edge`
3. **分仓库各自提交/发布**：每个分仓库独立 git 仓库，各自维护 Cargo.lock / feature flags，
   发布（npm / crates.io / PyPI）从分仓库执行。
4. **bindings/python 的归属**：目前无分仓库。后续可选：
   - (a) 新建 `split/geo-toolbox-python` 分仓库（PyPI 发布形态）；
   - (b) 并入 geo-toolbox-agent 分仓库。
   在决策落地前，bindings/python 仅在基座维护。
5. **同步纪律**：基座与分仓库不互相改写；若分仓库有修复需回灌，先合入基座再复制。
6. **可选自动化**：脚本（如 `scripts/sync_split.ps1`，当前不存在）按映射表做目录级 rsync/robocopy
   + 版本一致性校验，降低手工遗漏风险。

---

## 5. 约束与红线

- 基座边界内的所有 crate 必须在 `cargo check --workspace` 下 **0 error**。
- **禁止修改 split/ 下任何文件**（本任务及后续收口任务均适用）。
- 禁止删除/移动基座内任何文件（制品保留决策）。
- 未提交改动保留原样，不做无关重构。
