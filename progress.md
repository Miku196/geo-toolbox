# Progress

## Status
Phase 4.2 (MVT/PMTiles) + 死代码清理 (2026-06-19) — ✅

## 2026-06-19

### WMTS MVT 瓦片 + PMTiles 归档 (Phase 4.2)
- [x] `geo-ogc::mvt_source` 模块 — `MvtFeatureProvider` trait, `JsonFeatureProvider`, `render_mvt_tile()`
- [x] `WmtsLayer` 新增 `mvt_source` 字段 — 支持 MVT 格式
- [x] `handle_get_tile()` 根据 format 分发 MVT/栅格渲染
- [x] `handle_mvt_tile()` — 使用 `geo-tile::MvtEncoder` 生成 protobuf 瓦片
- [x] `WmtsService::build_pmtiles_archive()` — z0-10 全量 MVT 瓦片 → PMTiles v3
- [x] `WmtsService::estimate_mvt_tile_count()` — MVT 瓦片数量预估
- [x] `PmtilesWriter::finish()` 返回 `GeoResult<W>` 而非 `()`
- [x] `geo-server` 新增 `china-cities` MVT 示例图层
- [x] `geo-server` 新增 `GET /pmtiles/{layer}` 端点 → PMTiles 文件下载
- [x] +14 tests (geo-ogc: 25→39)

### 死代码清理
- [x] 移除 3 个确实未用的 Rust 导入 (Point, DebrisFlowRunout, GeeMq)
- [x] 保留 trait 导入 (Row/Column/ExternalAdapter/Plugin/Area/Centroid/BoundingRect/ConvexHull) — 方法解析必需
- [x] ROADMAP.md 更新 MVT 瓦片状态 [ ]→[x]

### CI 覆盖率门禁 (Phase 0.3)
- [x] PR 覆盖率对比门禁 — head 低于 base 时禁止合并
- [x] 主分支门禁 25%→45%
- [x] 定时任务：自动创建/更新覆盖率报告 issue
- [x] ROADMAP.md 更新 [ ]→[x]

### 测试统计
| 包 | 通过 | 新增 |
|----|------|------|
| geo-ogc | 39 | +14 (MVT/PMTiles) |
| geo-tile | 11 | — (finish() 返回类型变更) |
| geo-vector | 14 | — |
| geo-server | 编译通过 | — |
| workspace | ✅ 0 error | — |

### 剩余 [ ] 项 (10 处)
- USTC 镜像网络阻断: WASM npm发布, TypeScript类型, Python bindings (maturin)
- 架构待定: Jupyter Kernel, %%geo magic, matplotlib可视化, pandas↔GeoJSON
- CI: 失败测试自动issue
- 低优先级: QGIS工具箱,
