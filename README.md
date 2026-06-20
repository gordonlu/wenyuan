# 文渊阁 Wenyuan

文渊阁是一个本地运行的 AI 合议工作台。它把同一个问题交给三个不同立场的席位分别思考、互相批议、修订方案，并通过投票形成最终结论。

项目目标不是让一个模型给出更长的回答，而是把复杂决策拆成可观察、可比较、可追溯的过程：你可以看到每个席位提出了什么、批评了什么、如何修改，以及最终多数方案和少数意见分别是什么。

## 适合做什么

- 比较多个方案，而不是只要一个即时答案。
- 处理需要权衡长期收益、现实约束和风险底线的问题。
- 让不同模型或不同提示词角色对同一议题给出独立意见。
- 观察 AI 协作过程，评估三席合议是否比单 Agent 更稳定。
- 本地保存和导出议题记录，方便复盘。

## 三个席位

- 谋远席：偏长期、系统性、机会空间和战略方向。
- 经世席：偏落地、资源约束、执行路径和成本收益。
- 持正席：偏风险、伦理、边界条件和反方检查。

默认流程会经历独议、批议、复议、阁议等阶段。若三席无法形成多数，系统会允许一次合案复议，再输出可追溯的结果。

## 核心功能

- 三席独立会话：每个席位保留自己的提示词、模型配置、会话记录和执行状态。
- 实时进度：通过页面实时查看阶段进度、席位状态和事件时间线。
- 方案比较：横向查看创意、批议关系、修订方案、投票过程和最终结果。
- 证据池：区分事实、推断和偏好，让结论中的依据更清楚。
- 多模型路由：可为三个席位分别配置不同模型或不同 OpenAI-compatible Provider。
- Single Agent 对照模式：用单 Agent 流程跑同一议题，便于和三席模式比较。
- 人工介入：支持暂停、继续、取消、补充背景、重试阶段、重试失败席位和手动触发复议。
- 导出：支持 Markdown、JSON 和 HTML。

## 快速开始

后端默认使用 MockProvider，无需 API Key，适合先体验界面和流程：

```bash
cargo run -p wenyuan-server
```

访问：

```text
http://127.0.0.1:3210
```

如果需要前端开发服务器：

```bash
cd web
pnpm install
pnpm dev
```

Vite 默认地址：

```text
http://127.0.0.1:5173
```

## 接入真实模型

文渊阁支持 OpenAI-compatible API。复制 `.env.example` 为 `.env`，或直接设置环境变量：

```bash
WENYUAN_LLM_BASE_URL=https://api.example.com/v1
WENYUAN_LLM_API_KEY=your-api-key
WENYUAN_LLM_MODEL=your-model
WENYUAN_LLM_TIMEOUT_SECS=120
```

三个席位默认共用全局模型。也可以分别指定：

```bash
WENYUAN_LLM_MODEL_MOUYUAN=model-for-mouyuan
WENYUAN_LLM_MODEL_JINGSHI=model-for-jingshi
WENYUAN_LLM_MODEL_CHIZHENG=model-for-chizheng
```

如果三个席位需要走不同供应商，也可以分别设置：

```bash
WENYUAN_LLM_BASE_URL_MOUYUAN=https://provider-a.example/v1
WENYUAN_LLM_API_KEY_MOUYUAN=key-a

WENYUAN_LLM_BASE_URL_JINGSHI=https://provider-b.example/v1
WENYUAN_LLM_API_KEY_JINGSHI=key-b

WENYUAN_LLM_BASE_URL_CHIZHENG=https://provider-c.example/v1
WENYUAN_LLM_API_KEY_CHIZHENG=key-c
```

页面里的模型下拉框可通过 `WENYUAN_AVAILABLE_MODELS` 或 `WENYUAN_AVAILABLE_MODELS_MOUYUAN/JINGSHI/CHIZHENG` 配置，格式为逗号分隔。也可以用 `value:标签` 给同一个模型值展示不同名称。

## 基本使用

1. 新建议题，填写问题、背景和期望输出。
2. 选择三席模式或 Single Agent 模式。
3. 如有需要，为不同席位选择不同模型。
4. 启动会话，在工作台查看实时阶段、席位进度、方案和投票。
5. 对结果补充背景、重试阶段或导出记录。

## 数据与隐私

文渊阁默认本地运行，议题、事件、席位记录和结果保存在本地 SQLite 数据库中。使用真实模型时，提示词和会话内容会发送到你配置的模型供应商。

## 当前边界

当前版本聚焦三席合议、单 Agent 对照、本地持久化、可视化工作台和可选联网搜索。暂不包含用户系统、云端部署、通用工具调用、向量数据库、Tauri 桌面端或任意数量 Agent 编排。

## 联网搜索

会话勾选“启用搜索”后，系统会在第一轮独议前检索网络来源，并把结果作为 `external_sources` 提供给席位，同时写入最终证据池。未设置 `WENYUAN_SEARCH_PROVIDER` 时默认使用免 key fallback：`bing,duckduckgo,wikipedia`。如需显式关闭搜索后端，可设置 `WENYUAN_SEARCH_PROVIDER=none`；也可混合配置 `custom,doubao,tavily,google,searxng` 等 provider。

## 外部来源安全

搜索结果、文档解析结果和代码搜索结果都被视为不可信证据，而不是指令。进入三席输入前会移除控制字符、限制长度、记录来源 hash，并用 `safety_flags` 标记疑似 prompt injection 内容。模型提示中会明确要求只把这些内容作为事实材料使用，不执行来源文本里的指令。

## 文档解析

后端提供 `POST /api/tools/documents/parse`，请求体包含 `filename`、可选 `mime_type` 和 `content_base64`。当前支持纯文本、Markdown、JSON、日志、CSV/TSV、Excel/OpenDocument 表格、PDF 文本抽取和 DOCX 正文抽取；外部文档内容同样会进入安全净化层并标记为不可信来源。

## 工具调用轨迹

文档解析、代码搜索和会话内网页搜索都会生成 `tool_runs`，记录工具名、输入摘要 hash、状态、耗时、关联 evidence IDs 和错误信息。新建议题页可上传文件或执行代码搜索，结果会作为外部来源进入背景、证据池和最终报告的 Audit 导出。

代码搜索提供 `POST /api/tools/code/search`，默认只扫描服务启动目录；可用 `WENYUAN_CODE_SEARCH_ROOT` 指定允许搜索的项目根。搜索会跳过 `.git`、`target`、`node_modules`、`dist` 等目录，并只读取常见源码/配置文本文件。

## 用户偏好配置

后端提供 `GET/PUT /api/preferences`，默认保存到 `wenyuan-preferences.json`，也可用 `WENYUAN_PREFERENCES_PATH` 指定路径。偏好只保存本地默认配置：默认合议模式、书记官/搜索开关、投票策略、每席默认模型、代码搜索根目录和文件大小上限；不保存隐藏推理，不抽取历史决策，也不做向量记忆。
