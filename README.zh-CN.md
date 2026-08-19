# geo-toolbox

[English](README.md) | 简体中文

用于构建可复现 GIS 自动化管线的 Rust 地理空间工具箱。项目将纯 Rust 空间基础能力与可按 feature 启用的 PostGIS、GDAL、PDAL、Google Earth Engine、QGIS、DuckDB、STAC、OSM、CAD 和仿真工具适配器结合。

## 提供的能力

- 由 45 个 Rust package 组成的 workspace：15 个核心 crate、18 个领域插件、5 个适配器 crate，以及 CLI/WASM/Agent/Server 入口。
- 明确的分层依赖：Core -> Facade -> Plugin -> Wiring -> Adapter。只有 `geo-wiring` 可以组合插件与适配器。
- 可离线运行的 CRS、矢量、栅格、瓦片、时序、索引、统计、碳核算、GeoParquet 和 OGC 空间能力。
- 面向 AI Agent 的 CLI、MCP 端点、HTTP GeoAgent，以及 Python、MapLibre、QGIS、Jupyter 和 field PWA 集成。

## 仓库结构

```text
geo-toolbox/
|- core/       # 纯 Rust 地理空间算法与共享抽象
|- plugins/    # 碳核算、生态、水文、遥感等领域工作流
|- adapters/   # 可按 feature 启用的外部 GIS 系统桥接层
|- crates/     # CLI、WASM、Agent、Server 与组合根
|- bindings/   # Python、MapLibre GL JS、Jupyter、QGIS 集成
|- apps/       # Field PWA
|- examples/   # 可复现的地理空间示例与测试数据
|- docs/       # 补充文档
|- fuzz/       # cargo-fuzz 目标
```

依赖边界见 [BOUNDARY.md](BOUNDARY.md)，使用指南见 [WIKI.md](WIKI.md)，当前能力与可靠性工作见 [ROADMAP.md](ROADMAP.md)。

## 快速开始

### 前置条件

- Rust stable，项目 MSRV 目标为 Rust 1.80。
- 仅在启用相应适配器时需要安装外部工具：GDAL、QGIS、PostgreSQL/PostGIS、PDAL 或 Python。

### 构建和验证可移植 workspace

```bash
cargo check --workspace
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace -- -D warnings \
  -A clippy::too_many_arguments \
  -A clippy::manual_clamp \
  -A clippy::needless_range_loop \
  -A clippy::should_implement_trait
```

CI 会在 Linux 与 Windows 运行同一套可移植检查。GDAL native bindings 在安装 `libgdal-dev` 的 Ubuntu 专用 job 中验证，因此除非显式启用该 feature，本地构建不需要 GDAL SDK。

### 运行 CLI

```bash
cargo run -p geo-cli -- carbon assess input.geojson
cargo run -p geo-cli -- hydro basin dem.tif
cargo run -p geo-cli -- mcp-serve
```

构建最小 CLI：

```bash
cargo build --release --no-default-features --features minimal -p geo-cli
```

### 运行 GeoAgent

```bash
cargo run -p geo-agent
curl -X POST http://127.0.0.1:3000/agent \
  -H "Content-Type: application/json" \
  -d '{"query":"calculate NDVI for this area"}'
```

Provider 与 endpoint 配置见 [crates/geo-agent/README.md](crates/geo-agent/README.md)。

## 配置

将 `config.example.json` 复制为 `config.json`，或通过 `GEO_CONFIG_PATH` 指定仓库外的配置文件。`config.json` 被刻意忽略，因为适配器路径、服务端点和凭据均与机器有关。

```bash
cp config.example.json config.json
# 配置存放在其他位置时设置 GEO_CONFIG_PATH。
```

不要提交凭据、实际服务账号路径或工作站专用的可执行文件路径。

## 外部适配器约定

适配器会在可能的情况下于启动外部进程前校验失败条件。仓库默认测试套件保持可复现且不依赖外部环境；真实服务和二进制集成测试应在安装相应工具的可选环境中执行。

内部 `GT v1` 瓦片归档不是 PMTiles v3。在实现真正的 PMTiles v3 并通过官方 fixture 测试前，不应将其描述为可与 MapLibre 或 Protomaps PMTiles reader 互操作。

## 质量与安全

- CI 检查格式、Clippy、workspace 构建与测试、依赖审计、覆盖率、fuzz 目标以及 release/WASM 工作流。
- `cargo audit` 会阻止未获准的安全公告。已有上游例外记录在 [`.cargo/audit.toml`](.cargo/audit.toml)，应在上游依赖变动时复查。
- 当前主开发分支为 `master`。

## 绑定与示例

- [MapLibre GL JS 绑定](bindings/maplibre-gl-geo-toolbox/README.md)
- [QGIS 插件](bindings/qgis/geo_toolbox_qgis/README.md)
- [Jupyter 集成](bindings/jupyter/README.md)
- [ObservableHQ 示例](docs/observablehq/README.md)
- [中国自然灾害风险评估示例](examples/china-risk-assessment/README.md)

## 许可证

MIT
