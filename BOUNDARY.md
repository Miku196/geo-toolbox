# Geo-Toolbox 仓库边界（BOUNDARY）

> 记录时间：2026-08-14 合并决策 · 状态：**单仓库（monorepo）**

## 决策：合并为单仓库

历史上有过 multi-repo 拆分（D:\geo\split\ 四个分仓库），实践中产生了代码漂移
（同一修复要同步多份）、重复构建与依赖配置复杂度。经评估合并回单仓库：

- **唯一仓库**：D:\geo\geo-toolbox（git 仓库，master 分支）
- 旧分仓库归档于 D:\geo\split-archived\（保留 git 历史，不再维护）

## 单仓库布局（三个方向 = 三个目录）

```
geo-toolbox/
├── core/         基座算法（15 core crates）
├── plugins/      领域插件（18 crates）
├── crates/
│   ├── geo-cli/       方向一：AI 边缘计算（minimal feature 裁剪 14MB）
│   ├── geo-wasm/      方向二：WASM 浏览器离线
│   ├── geo-agent/     方向三：AI 网关（LLM→工具调用）
│   ├── geo-server/    HTTP/WMS 服务
│   └── geo-wiring/    DI 组合根
├── adapters/     外部桥接（PostGIS/GEE/QGIS/GDAL/…）
├── bindings/     Python / MapLibre / Jupyter / QGIS
└── examples/     案例（dexing-copper / chengdu-carbon / china-risk-assessment）
```

## 依赖原则

- 方向严格单向：Core ← Facade ← Plugin ← Wiring ← Adapter；crates/ 与 bindings/ 是消费端
- 全 path 依赖、单一 Cargo.lock、单一 target/
- 方向间的差异（轻量/浏览器/服务端）由 **feature flags** 控制，不由仓库隔离控制

## 发布（按产物，不拆仓库）

- CLI 二进制：cargo build --release -p geo-cli
- WASM npm 包：wasm-pack build crates/geo-wasm + bindings/maplibre npm publish
- AI 网关：cargo build -p geo-agent
- crates.io：cargo publish --manifest-path <crate>/Cargo.toml 逐个发布

## 质量线（守住即可）

cargo check --workspace --all-targets 0 error 0 warning
cargo clippy --workspace --all-targets 0 deny
cargo audit 0 vulnerabilities（例外见 .cargo/audit.toml）
cargo test --workspace 全绿

## 工程约定（合并自 context.md）

### 架构原则

1. **Core 层不依赖 Plugin/Adapter 层** — Core 是纯基础能力
2. **通过 PluginRegistry 发现工具** — 插件注册后在 CLI / Server / WASM 中统一调用
3. **插件使用 trait 接口隔离** — 每个 plugin 通过 Plugin trait 暴露能力
4. **Adapter 使用 trait 接口** — ExternalAdapter 统一 push/pull/execute
5. **深度优先于广度** — 优先让一个模块做深做透，而非分散的浅封装

### 可观测性

- 全项目使用 tokio tracing（非 `log` 宏）；结构化字段：`tracing::info!(field = value, "msg")`
- 统一 key 命名：path / table / count / latency_ms / error / bbox / crs / source / bytes
- 关键入口 `#[tracing::instrument]`；`GEO_LOG_FORMAT=json` 切 JSON；geo-server 响应头返回 X-Trace-Id

### 系统韧性

- **ResourceGuard**（`geo-core::guard`）— 输入 50MB / 100 万要素 / 10k² 栅格上限
- **CachedHealth**（`geo-core::health`）— TTL 缓存健康探针，is_ok() O(1)
- **BlockingPool** — CPU 密集算法（STL 分解、Mann-Kendall、栅格卷积）经 `spawn_blocking` 隔离
- **ScenarioMatrix** — 4 EcoZone × 4 LandUseScenario 单选查找 IPCC 参数，禁止枚举

