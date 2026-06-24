# geo-toolbox 项目指令

Rust 地理空间工具链。Core → Plugin → Adapter 三层架构。
全功能 geo-toolbox-master skill 已安装（89 个 MCP 工具，Finder→Core→Plugin→Adapter→Core 五阶段流水线）。

## 工具使用
- 分析代码：tokensave_search / tokensave_context / tokensave_signature（禁止 raw grep/read）
- 调用链：tokensave_callers / tokensave_callees
- 影响分析：tokensave_impact / tokensave_affected
- 诊断：tokensave_diagnostics（解析 cargo check 输出）
- 代码编辑：tokensave_str_replace / tokensave_insert_at_symbol / tokensave_replace_symbol

## 三层架构
- **Core** — Rust 核心。性能敏感路径（批写、格式转换、消息分发、碳核算）。
  改前确认不影响 Plugin/Adapter 兼容性。
- **Plugin** — 插件层。遥感和空间分析委托 Python 生态（GEE SDK、PyQGIS、GDAL CLI、brightway2）。
- **Adapter** — 外部适配器。PostGIS、GEE、QGIS、GDAL 等 GIS 工具适配。

## geo-toolbox-master Skill
当用户提及 GIS/遥感/碳核算/流域等任务时自动激活。
五阶段：stage0(搜索)→stage1(入库)→stage2(分析)→stage3(碳汇)→stage4(成果)
参数：workspace、domain、boundary、aoi_name、year、skip_stages

## 编译 & 测试
- Rust: cargo build / cargo test / cargo check
- 需要时：cargo clippy / cargo fmt
- 改完必跑 cargo check 验证

## 环境
- 默认 provider: router（DeepSeek v4 模型自动路由）
- 思考级别: medium
- Caveman lite 模式活跃

## 项目管理
- 复杂拆分：to-issues（转 issue）、to-prd（转 PRD）、triage（分类处理）
- 需要时用 subagents 并行分析或审查
- 用 evo 自动优化关键性能路径
