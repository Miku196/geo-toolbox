# geo-toolbox-agent 🤖

AI Agent API 网关 — 将自然语言地理分析请求转换为 JSON 工具调用指令。

## 架构

```
用户 HTTP 请求 ({"query": "分析德兴铜矿 NDVI"})
         ↓
   /agent 端点 (axum)
         ↓
   ┌─────────────────────┐
   │  LLM Provider       │ ← OpenAI / Claude / DeepSeek
   │  (云端大模型)        │
   └─────────┬───────────┘
             ↓ 或 (断网降级)
   ┌─────────────────────┐
   │  KeywordRouter      │ ← keywords.yaml 关键词匹配
   └─────────┬───────────┘
             ↓
   {"tool": "ndvi", "params": {"input": "path.tif", "red_band": 3, "nir_band": 4}}
         ↓
   返回给调用方 → 由 geo-cli 或前端执行
```

## 快速开始

### 1. 配置环境

```bash
cp .env.example .env
# 编辑 .env 填入 API Key
```

### 2. 生成工具 Schema（从 geo-toolbox 主项目）

```bash
python ../scripts/generate_agent_schemas.py
```

### 3. 启动

```bash
cargo run --release
```

### 4. 调用

```bash
curl -X POST http://127.0.0.1:3000/agent \
  -H "Content-Type: application/json" \
  -d '{"query": "计算这个区域的 NDVI，红色波段是第3波段，近红波段是第4波段"}'
```

响应：

```json
{
  "fallback": false,
  "provider": "openai",
  "model": "gpt-4o",
  "tool_calls": [
    {
      "tool": "ndvi",
      "params": {
        "red_band": 3,
        "nir_band": 4
      }
    }
  ],
  "usage": {
    "prompt_tokens": 234,
    "completion_tokens": 45,
    "total_tokens": 279
  }
}
```

## API 端点

| 端点 | 方法 | 说明 |
|------|------|------|
| `/agent` | POST | 自然语言 → 工具调用 JSON |
| `/metrics` | GET | Token 用量/成本统计 |
| `/health` | GET | 健康检查 |

## 嵌入式宿主与测试

`geo_agent::build_app()` 保持独立启动程序的环境变量兼容行为。嵌入其他 Rust 宿主或编写 HTTP 集成测试时，使用 `AgentAppConfig` 与 `build_app_with_config(config)` 显式传入 provider、日志目录和 fallback 开关，避免测试修改进程环境或向仓库目录写运行日志。

`run()` 只负责 dotenv、tracing 与 TCP listener 生命周期；router 构造不绑定端口，可直接用 Axum/Tower 的 in-process service 测试 `/health`、`/agent` 和 `/metrics`。

## 多提供商切换

环境变量 `AI_PROVIDER=openai|claude|deepseek`

- **OpenAI**: 默认，使用 `gpt-4o`，支持 function calling
- **Claude**: Anthropic Messages API，使用 `claude-sonnet-4-20250514`
- **DeepSeek**: 国产替代，使用 `deepseek-chat`，性价比高

## 断网降级

设置 `FALLBACK_ENABLED=true` 启用。当 API 调用超时或网络不可达时，自动切换至 `keywords.yaml` 关键词匹配。降级日志标注 `[FALLBACK]`。

```json
{
  "fallback": true,
  "provider": "fallback",
  "model": "keyword-router",
  "tool_calls": [{"tool": "ndvi", "params": {}}],
  "usage": null
}
```

## 工具 Schema

`tools_schema.json` 包含 238 个地理空间分析工具的 OpenAI 函数调用格式定义。通过 `scripts/generate_agent_schemas.py` 从 `geo-toolbox` 主项目自动生成。

## 三方向架构

```
geo-toolbox/           ← 基座（核心算法 + 89+ 工具实现）
geo-toolbox-agent/     ← 方向一：AI Agent 网关（本仓库）
geo-toolbox/cli        ← 方向三：AI 边缘计算 CLI
geo-toolbox/wasm       ← 方向二：离线浏览器 WASM
```
