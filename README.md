# 文渊阁 Wenyuan

[![CI](https://github.com/gordonlu/wenyuan/actions/workflows/ci.yml/badge.svg)](https://github.com/gordonlu/wenyuan/actions/workflows/ci.yml)
[![Tauri](https://img.shields.io/badge/Tauri-2-FFC131?logo=tauri)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-2024-dea584?logo=rust)](https://www.rust-lang.org)
[![Vue](https://img.shields.io/badge/Vue-3-4FC08D?logo=vue.js)](https://vuejs.org)
[![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?logo=typescript)](https://www.typescriptlang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

文渊阁是一个本地 AI 协作合议工作台：把同一个用户目标交给多个 AI 独立判断，只对真正的分歧、遗漏和更优实现进行互动，再收敛成最终结果。

多 Agent 不等于人为分配不同立场。默认原则是**目标一致、判断独立、差异自然产生**；如果多个模型认为同一个方案最好，它们可以直接形成共识，不需要为了“多角度”而故意唱反调。

文渊阁不是一个 prompt，而是一个面向复杂决策的分阶段合议运行时。它负责状态、进度、交叉校验、恢复、用户追问去重和最终收敛。

如需在 AI 对话中快速体验合议方法，可加载 `skill/SKILL.md`（Lite 版），无需安装即可在当前对话中运行。

## 两种协作方式

### 内置三席

兼容现有工作区、历史记录、证据池和投票流程：

```text
独议 → 批议 → 复议 → 投票 → 结论
```

“谋远席 / 经世席 / 持正席”名称继续保留用于数据库与 UI 兼容，但在当前版本中只是参与者标识，**不再强制分别承担长期 / 落地 / 风险视角**。

### MCP 外部 Agent 会议

文渊阁可以作为本地 MCP 协作总线，让 2～3 个独立 Agent 客户端进入同一场会议，例如 Codex、Claude Code、OpenCode 或其他支持 HTTP MCP 的 Agent。

```text
用户目标
  ↓
2～3 Agent 独立回答
  ↓
只处理实质分歧 / 遗漏
  ↓
临时综合者收敛最佳实现
```

MCP 默认地址：

```text
http://127.0.0.1:3847/mcp
```

详细说明见 [`docs/MCP_MEETINGS.md`](docs/MCP_MEETINGS.md)。

## 适合谁用

- 一个问题想同时听 2～3 个强模型的判断，但不想反复复制粘贴
- 做决策前需要独立意见与交叉校验，而不是单模型一次回答
- 技术 / 产品 / 研究方案需要多个 Agent 共同找最佳实现
- 需要可追溯的讨论记录和失败恢复

## 快速开始

**桌面版**（推荐）：

```bash
# 从 Releases 下载最新版，双击打开
# 首次进入配置页，填写 API Key，创建议题
```

**命令行版**：

```bash
cargo run -p wenyuan-app
```

启动后：

- 文渊阁工作区使用随机 localhost 端口打开
- MCP meeting 默认监听 `127.0.0.1:3847`
- Mock 模式仍可在没有 API Key 时体验内置流程

MCP 可通过环境变量控制：

```env
WENYUAN_MCP_ENABLED=true
WENYUAN_MCP_PORT=3847
```

## MCP 会议的关键交互规则

- 只支持 2～3 Agent，避免参与者过多导致延迟和交叉阅读成本快速上升
- 所有 Agent 面对同一个用户目标，不预设角色
- Initial 阶段独立回答
- Cross Review 只回应实质分歧 / 遗漏；已经一致的内容不重复
- Synthesis 由临时综合者完成，它不是“裁判”或更高权威
- 相同用户问题自动合并，同一时间只暴露一个 distinct question
- `next_task` / `submit_turn` 都支持幂等重试
- Agent 用 `resume_token` 断线重连，不创建重复参与者
- 3 Agent 掉 1 个超过 grace 后可降级为 2 Agent
- 2 Agent 掉 1 个会暂停，不会悄悄变成单 Agent

## 核心功能

- 多 Agent 独立思考、交叉校验和结果收敛
- 三席内置合议 + 2～3 外部 Agent MCP 会议
- 实时查看进度、席位状态、事件时间线
- 证据池（区分事实 / 推断 / 偏好）
- 支持不同模型与不同 Provider
- Single Agent 对照模式
- 暂停 / 继续 / 重试 / 手动触发复议
- MCP 用户追问去重、掉线重连和退化策略
- 导出 Markdown / JSON / HTML

---

<details>
<summary><b>开发者专区</b>（点击展开）</summary>

## 开发运行

```bash
# 一键启动（内置前端 + MCP）
cargo run -p wenyuan-app

# 分进程开发（主 server；MCP 当前由 wenyuan-runtime 启动）
cargo run -p wenyuan-server

# 前端 dev server
cd web && pnpm install && pnpm dev
```

## 配置模型

支持 OpenAI-compatible API。复制 `.env.example` 为 `.env`：

```bash
WENYUAN_LLM_BASE_URL=https://api.deepseek.com
WENYUAN_LLM_API_KEY=sk-xxxx
WENYUAN_LLM_MODEL=deepseek-chat
```

也可给不同内部参与者指定不同模型和供应商；这只改变模型来源，不再意味着固定思考职责：

```bash
WENYUAN_LLM_BASE_URL_MOUYUAN=https://api.deepseek.com
WENYUAN_LLM_MODEL_MOUYUAN=deepseek-chat
WENYUAN_LLM_BASE_URL_JINGSHI=https://ark.cn-beijing.volces.com/api/v3
WENYUAN_LLM_MODEL_JINGSHI=doubao-seed-2.0-pro
```

## 桌面版构建

```bash
cd desktop && npm install && npx tauri build
```

## 项目结构

```text
crates/
├── wenyuan-agent     # 内置合议编排、书记官、搜索工具调用
├── wenyuan-core      # 领域模型（Session/Seat/Phase/Vote...）
├── wenyuan-provider  # LLM Provider 抽象（OpenAI/Mock/...）
├── wenyuan-store     # SQLite 持久化 + migration
├── wenyuan-server    # Axum API + SSE + embedded 前端
├── wenyuan-runtime   # 本地 server + MCP server 启动逻辑
├── wenyuan-mcp       # 2～3 外部 Agent MCP Meeting Coordinator
├── wenyuan-app       # 单二进制 CLI 入口
├── wenyuan-tools     # 文档解析、代码搜索、安全净化
web/                  # Vue 3 前端
desktop/              # Tauri 桌面壳
```

## 架构

```text
                 ┌─ 内置 Agent Runner ─→ LLM Providers
[用户] → Wenyuan ┤
                 └─ MCP Meeting Hub ───→ 外部 Agent A/B/C
                         │
                         ├─ turn coordination
                         ├─ question dedupe
                         ├─ heartbeat / reconnect
                         └─ synthesis
```

现有内置 Session 与 MCP Meeting 暂时并行，以避免为了动态参与者一次性迁移历史数据、投票、follow-up 和前端工作区。设计 review 见 [`docs/COLLABORATIVE_MEETING_REVIEW.md`](docs/COLLABORATIVE_MEETING_REVIEW.md)。

## 安全

- 主服务和 MCP 都只监听 `127.0.0.1`
- 主服务所有写接口受 `X-Wenyuan-Token` 保护
- API Key 不返回前端，日志脱敏
- 外部来源（搜索 / 文档 / 代码）不可作为指令执行
- MCP meeting token 只用于本地会议身份恢复和主持操作

## 技术栈

| 层 | 技术 |
|----|------|
| 后端 | Rust / Axum / SQLite |
| MCP Meeting | Rust / Axum / JSON-RPC |
| 前端 | Vue 3 / TypeScript / Vite |
| 桌面 | Tauri 2 |

</details>

## License

MIT
