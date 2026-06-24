# 文渊阁 Wenyuan

[![CI](https://github.com/gordonlu/wenyuan/actions/workflows/ci.yml/badge.svg)](https://github.com/gordonlu/wenyuan/actions/workflows/ci.yml)
[![Tauri](https://img.shields.io/badge/Tauri-2-FFC131?logo=tauri)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-2024-dea584?logo=rust)](https://www.rust-lang.org)
[![Vue](https://img.shields.io/badge/Vue-3-4FC08D?logo=vue.js)](https://vuejs.org)
[![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?logo=typescript)](https://www.typescriptlang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

文渊阁是一个本地 AI 合议工作台——把同一个问题交给三个不同立场的 AI 分别思考、互相批议、修订方案，然后投票得出结论。你可以看到每个席位说了什么、批评了什么、改了哪里、最终选了什么方案和少数意见是什么。

文渊阁不是一个 prompt，而是一个面向复杂决策的分阶段合议运行时。Prompt 只能模拟其方法，不能承载其状态、进度、校验、复盘和持续追问。

## 适合谁用

- 做决策前想听听不同角度的意见
- 想比较多个方案，而不是只要一个答案
- 需要可追溯的讨论记录，方便复盘

## 快速开始

**桌面版**（推荐）：

```bash
# 从 Releases 下载最新版，双击打开
# 首次进入配置页，填写 API Key，创建议题
```

**命令行版**（无需 API Key 可先体验）：

```bash
cargo run -p wenyuan-app
# 自动打开浏览器 → 进入配置页
# Mock 模式下可直接创建议题，无需配置模型
```

## 三个席位

| 席位 | 视角 | 关注点 |
|------|------|--------|
| 谋远席 | 未来 | 长期机会、替代路线、系统性思考 |
| 经世席 | 落地 | 资源约束、执行路径、成本收益 |
| 持正席 | 底线 | 风险、逻辑、伦理、边界条件 |

流程：独议 → 批议 → 复议 → 投票 → 结论。若三票分散，允许一次合案复议。

## 核心功能

- 三席独立思考+互相批议+修正方案+投票表决
- 实时查看进度、席位状态、事件时间线
- 证据池（区分事实/推断/偏好）
- 支持不同模型给不同席位（如 DeepSeek + GLM + Kimi）
- Single Agent 对照模式
- 暂停/继续/重试/手动触发复议
- 导出 Markdown / JSON / HTML

---

<details>
<summary><b>开发者专区</b>（点击展开）</summary>

## 开发运行

```bash
# 一键启动（内置前端）
cargo run -p wenyuan-app

# 分进程开发（需要 web/dist）
cargo run -p wenyuan-server
# 或前端 dev server
cd web && pnpm install && pnpm dev
```

## 配置模型

支持 OpenAI-compatible API。复制 `.env.example` 为 `.env`：

```bash
WENYUAN_LLM_BASE_URL=https://api.deepseek.com
WENYUAN_LLM_API_KEY=sk-xxxx
WENYUAN_LLM_MODEL=deepseek-chat
```

也可为三席分别指定不同模型和供应商：

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

```
crates/
├── wenyuan-agent     # 三席编排、书记官、搜索工具调用
├── wenyuan-core      # 领域模型（Session/Seat/Phase/Vote...）
├── wenyuan-provider  # LLM Provider 抽象（OpenAI/Mock/...）
├── wenyuan-store     # SQLite 持久化 + migration
├── wenyuan-server    # Axum API + SSE + embedded 前端
├── wenyuan-runtime   # 本地 server 启动逻辑（CLI + 桌面共用）
├── wenyuan-app       # 单二进制 CLI 入口
├── wenyuan-tools     # 文档解析、代码搜索、安全净化
web/                  # Vue 3 前端
desktop/              # Tauri 桌面壳
```

## 架构

```
[用户] → Tauri 桌面 / 浏览器
         ↓
    Axum API / SSE
         ↓
    Agent Runner (三席并发)
         ↓
    LlmProvider trait → OpenAI / Mock / ...
    SearchBackend     → Doubao / Tavily / Google / SearXNG
         ↓
    SQLite (本地持久化)
```

## 安全

- 只监听 `127.0.0.1`，不暴露到网络
- 所有写接口受 `X-Wenyuan-Token` 保护
- API Key 不返回前端，日志脱敏
- 外部来源（搜索/文档/代码）不可作为指令执行

## 技术栈

| 层 | 技术 |
|----|------|
| 后端 | Rust / Axum / SQLite |
| 前端 | Vue 3 / TypeScript / Vite |
| 桌面 | Tauri 2 |

</details>

## License

MIT
