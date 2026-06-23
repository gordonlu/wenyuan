# 架构说明

## 当前定位

文渊阁是本地运行的 AI 合议工作台。它的核心不是聊天，而是把一个议题拆成可观察、可比较、可追溯的审议过程，并允许外部事实来源进入证据池。

当前架构已经可以冻结的行为契约包括：

- 三席合议主流程和 Single Agent 对照模式。
- SQLite 本地持久化、事件轨迹、失败重试和 stale execution recovery。
- 每席模型和每席 OpenAI-compatible Provider 路由。
- 投票策略、书记官、证据池、工具轨迹、用户偏好 JSON。
- 搜索、文档解析、代码搜索进入统一 external evidence/tool_runs。

还不应冻结的是 UI 呈现层：运行中动画、结果可信度摘要、工具轨迹摘要、空状态和错误修复提示仍在产品化。

## 分层

```text
Vue UI
  -> Axum HTTP/SSE
  -> Agent Runner
  -> Provider trait
  -> OpenAI-compatible 或 MockProvider

Axum HTTP
  -> Search backend pool
  -> wenyuan-tools 文档解析/代码搜索/安全净化
  -> 用户偏好 JSON

Agent Runner
  -> wenyuan-core 状态机、多数决、证据和工具轨迹模型
  -> wenyuan-store SQLite 持久化
```

## Crate 边界

### `wenyuan-core`

不依赖 Axum、SQLx 或具体模型 API。它定义稳定领域模型：

- `Session`
- `DeliberationMode`
- `SessionPhase`
- `SeatKind`
- `VotePolicy` / `VoteStrategy`
- `Evidence`
- `EvidenceSourceKind`
- `EvidenceTrustLevel`
- `SourceSafetyFlags`
- `ToolRun`

HTTP handler 不直接修改阶段字符串，只通过核心状态机推进阶段。

### `wenyuan-provider`

暴露 `LlmProvider` 和 `SearchBackend` 抽象。Agent Runner 不知道真实 HTTP API 细节，真实 Provider 和 MockProvider 可互换。

搜索 backend 支持：

- custom
- Doubao
- Tavily
- Google Custom Search
- SearXNG

搜索默认关闭。通过 `WENYUAN_SEARCH_PROVIDER` 环境变量启用（`doubao`、`tavily`、`google`、`searxng`、`custom`）。

### `wenyuan-tools`

负责外部来源工具和安全净化：

- `sanitize_untrusted_text`
- 搜索结果转 evidence
- 文档解析转 evidence
- 代码搜索转 evidence
- `ToolRun` 生成

当前文档解析支持：

- txt/md/json/log
- csv/tsv
- xlsx/xls/xlsm/xlsb/ods
- pdf
- docx

所有外部来源默认标记为 `trust_level=untrusted_external`，进入模型前只作为事实材料，不作为指令执行。

### `wenyuan-agent`

负责三席并发执行、阶段屏障、模型调用、JSON 修复、书记官、搜索注入和最终 artifacts 生成。

每一阶段必须相关席位全部完成并通过 JSON 解析后，才允许推进下一阶段。解析失败会保存 raw output，发起一次格式修复；再次失败则标记席位失败，不静默推进。

会话启用搜索时，Agent Runner 会在独议前运行搜索 backend，把搜索结果写入：

- `external_sources`
- `artifacts.evidence`
- `artifacts.tool_runs`

用户上传文档、代码搜索和外部 Agent 注入的材料也会通过 `external_evidence` / `external_tool_runs` 进入同一条链路。

### `wenyuan-store`

使用“当前状态业务表 + 轻量事件日志”。第一版不实现完整 Event Sourcing，但 `session_events` 会记录 UI 进度和排障信息。

SQLite 表包含：

- `sessions`
- `seats`
- `seat_runs`
- `rounds`
- `ideas`
- `critiques`
- `proposals`
- `votes`
- `session_events`

当前实现会把完整 `DiscussionArtifacts` 作为 JSON 快照写入 `sessions.artifacts_json`，同时把 Idea、Critique、Proposal、Vote 分表保存，兼顾恢复、查询和排障。

新增外部来源字段存储在 session/artifacts 层：

- `external_evidence_json`
- `external_tool_runs_json`
- `artifacts.evidence`
- `artifacts.tool_runs`

## HTTP 边界

稳定接口：

- `GET /api/health`
- `GET /api/config/status`
- `GET /api/preferences`
- `PUT /api/preferences`
- `POST /api/sessions`
- `GET /api/sessions`
- `GET /api/sessions/{id}`
- `POST /api/sessions/{id}/start`
- `POST /api/sessions/{id}/pause`
- `POST /api/sessions/{id}/resume`
- `POST /api/sessions/{id}/cancel`
- `POST /api/sessions/{id}/retry`
- `POST /api/sessions/{id}/retry-phase`
- `POST /api/sessions/{id}/retry-seat/{seat}`
- `GET /api/sessions/{id}/events`
- `POST /api/tools/documents/parse`
- `POST /api/tools/code/search`

`/api/sessions/{id}/events` 是 Server-Sent Events，用于 UI 和外部 Agent 观察阶段变化。

## 并发和恢复

第一版同时使用进程内 `HashMap<Uuid, CancellationFlag>` 和 SQLite execution lease 防止同一 Session 被重复启动。

进程内锁负责当前服务实例内的快速互斥。SQLite 的 `execution_token`、`lease_expires_at` 和 `recovery_state` 负责跨请求、服务异常退出后的可诊断状态。

服务启动时会扫描过期 lease，把未完成执行标记为 `retry_required` 并释放 token。取消 Session 会清理 lease；后台任务完成前会再次检查 token，避免取消后的结果覆盖取消状态。

后续若要支持多进程或云部署，应把并发保护下沉为数据库条件更新或租约表。

## 配置

没有配置真实模型时默认使用 MockProvider。配置真实模型需要：

```env
WENYUAN_LLM_BASE_URL=
WENYUAN_LLM_API_KEY=
WENYUAN_LLM_MODEL=
```

每席独立 Provider：

```env
WENYUAN_LLM_BASE_URL_MOUYUAN=
WENYUAN_LLM_API_KEY_MOUYUAN=
WENYUAN_LLM_MODEL_MOUYUAN=

WENYUAN_LLM_BASE_URL_JINGSHI=
WENYUAN_LLM_API_KEY_JINGSHI=
WENYUAN_LLM_MODEL_JINGSHI=

WENYUAN_LLM_BASE_URL_CHIZHENG=
WENYUAN_LLM_API_KEY_CHIZHENG=
WENYUAN_LLM_MODEL_CHIZHENG=
```

搜索配置：

```env
WENYUAN_SEARCH_PROVIDER=doubao
WENYUAN_SEARCH_PROVIDER=none
WENYUAN_SEARCH_API_URL=
WENYUAN_SEARCH_API_KEY=
WENYUAN_SEARCH_TAVILY_KEY=
WENYUAN_SEARCH_SEARXNG_URL=
```

工具和偏好配置：

```env
WENYUAN_CODE_SEARCH_ROOT=.
WENYUAN_PREFERENCES_PATH=wenyuan-preferences.json
```

配置状态接口不返回 API Key。

## 安全边界

外部来源包括搜索结果、上传文档、代码搜索结果、外部 Agent 注入内容。它们都不能成为模型的指令来源。

实现约束：

- 去除控制字符。
- 限制文本长度。
- 标记疑似 prompt injection。
- 记录来源 hash。
- 记录 `source_kind`、`trust_level`、`safety_flags`。
- prompt 明确要求模型只把外部来源当作事实材料。

这层是产品行为契约，应继续保持稳定。
