# 🗺️ Geo-Toolbox 开发路线图

> 单仓库（monorepo）· 一个基座 + 三个方向：① AI 边缘计算 ② WASM 浏览器离线 ③ GeoAgent
> 状态快照更新时间：2026-08

---

## 📊 现状快照

| 维度 | 状态 | 说明 |
|------|------|------|
| 仓库形态 | ✅ 单仓 | split 分仓库已合并归档（D:\geo\split-archived） |
| 结构 | ✅ 15 core / 18 plugins / 5 adapters / 5 入口 | crates: cli·wasm·agent·server·wiring |
| 编译质量 | ✅ 0 error 0 warning | `cargo check --workspace` |
| Lint | ✅ clippy 0 deny | workspace 级 `-D warnings` |
| 安全 | ⚠️ 7 vulns（5 项已豁免） | pyo3 wait-0.29、rsa no-fix、rumqttc 锁 webpki，见 .cargo/audit.toml |
| 测试 | ✅ 90 test targets 全绿 | 含 MCP E2E、高风险函数覆盖 |
| 工具数量 | ✅ 240 tools / 89 MCP tools | `crates/geo-agent/tools_schema.json` |
| 归档插件 | 8 个 | contrib/archived-plugins（不再构建） |

---

## 三个方向

### ① AI 边缘计算 — geo-cli（14MB minimal）

- [x] minimal feature 裁剪 → 14MB 离线二进制
- [x] 子命令 + Unix 管道双模式
- [ ] 树莓派 / ARM 交叉编译验证（aarch64-unknown-linux-gnu）
- [ ] 边缘设备基准：典型管线（read→buffer→write）内存/耗时
- [ ] 增量更新机制（OTA 下发单个 crate 重编译）

### ② WASM 浏览器离线 — geo-wasm

- [x] wasm32 目标 + `_inner` + `GeoResult<T>` 模式（27/27 原生测试）
- [x] MapLibre GL JS 插件（10 ES 模块）+ ObservableHQ 示例
- [x] field-pwa 野外作业 PWA（IndexedDB 离线存储）
- [ ] `wasm-pack build` 产物体积优化（wasm-opt + 分 crate 懒加载）
- [ ] PWA Service Worker 缓存策略完善 + 离线队列
- [ ] Web Worker 计算池（大栅格不阻塞主线程）

### ③ GeoAgent — geo-agent

- [x] LLM 网关（OpenAI/Claude/DeepSeek）→ JSON tool_calls
- [x] 离线降级 KeywordRouter（keywords.yaml 关键词路由）
- [x] tools_schema.json 自动生成（scripts/generate_agent_schemas.py）
- [ ] 工具执行闭环：agent 输出直接驱动 geo-cli 执行并回传结果
- [ ] 多轮对话状态 + 工具链编排（一个查询拆多步调用）
- [ ] MCP 协议接入（作为 MCP client 调用外部工具）

---

## 🔧 技术债

| 项 | 说明 | 触发条件 |
|----|------|----------|
| pyo3 0.29 升级 | RUSTSEC-2026-0176/0177（wait-0.29） | pyo3 0.29 正式发布 |
| rumqttc webpki 修复 | RUSTSEC-2026-0049/0098/0099/0104 | 上游解锁 webpki 0.102.8 |
| rsa 替换 | RUSTSEC-2023-0071 no-fix | 迁移至 rustcrypto 系列 |

---

## 📐 里程碑回顾

- **Round 1-2**：五层洋葱架构落地，core 15 + plugins 18 crates
- **Round 3**：Registry 中心 + 适配器层 + 浏览器生态（MapLibre + ObservableHQ）
- **2026-06**：WASM 架构重构（_inner 模式）、MapLibre 绑定补齐、warning 清零
- **2026-08**：单仓合并（split → monorepo）、god-file 拆分（wmts/factor/rusle/dexing）、依赖安全加固、geo-agent 集成、文档整理

## 文档

- 使用指南：[WIKI.md](WIKI.md) · 仓库边界：[BOUNDARY.md](BOUNDARY.md) · 入口：[README.md](README.md)
