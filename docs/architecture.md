# 架构说明

## 分层

文渊阁采用 Rust workspace 和独立 Vue 前端。

```text
Vue UI
  -> Axum HTTP/SSE
  -> Agent Runner
  -> Provider trait
  -> OpenAI-compatible 或 MockProvider

Agent Runner
  -> wenyuan-core 状态机与多数决
  -> wenyuan-store SQLite 持久化
```

## 核心边界

`wenyuan-core` 不依赖 Axum、SQLx 或具体模型 API。HTTP handler 不直接修改阶段字符串，只通过核心 `Session::transition_to` 推进状态。

`wenyuan-provider` 只暴露 `LlmProvider`。Agent Runner 不知道真实 HTTP API 的细节，真实 Provider 和 MockProvider 可互换。

`wenyuan-agent` 负责三席并发执行和阶段屏障。每一阶段必须三席全部完成并通过 JSON 解析后，才允许推进下一阶段。

`wenyuan-store` 使用“当前状态业务表 + 轻量事件日志”。第一版不实现完整 Event Sourcing，但 `session_events` 会记录 UI 进度和排障信息。

## 并发选择

第一版同时使用进程内 `HashMap<Uuid, CancellationFlag>` 和 SQLite execution lease 防止同一 Session 被重复启动。进程内锁负责当前服务实例内的快速互斥，SQLite 的 `execution_token`、`lease_expires_at` 和 `recovery_state` 负责跨请求、服务异常退出后的可诊断状态。

服务启动时会扫描过期 lease，把未完成执行标记为 `retry_required` 并释放 token。取消 Session 会清理 lease；后台任务完成前会再次检查 token，避免取消后的结果覆盖取消状态。

后续若要支持多进程或云部署，应把并发保护下沉为数据库条件更新或租约表。

## 持久化

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

当前实现会把完整 `DiscussionArtifacts` 作为 JSON 快照写入 `sessions.artifacts_json`，同时把 Idea、Critique、Proposal、Vote 分表保存，兼顾恢复和排障。

## 配置

没有配置真实模型时默认使用 MockProvider。配置真实模型需要：

```env
WENYUAN_LLM_BASE_URL=
WENYUAN_LLM_API_KEY=
WENYUAN_LLM_MODEL=
```

配置状态接口不返回 API Key。
