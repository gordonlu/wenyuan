# 文渊阁 Wenyuan

文渊阁是一个本地运行的三席 AI 合议系统。用户创建议题后，谋远席、经世席、持正席分别独议、批议、复议、阁议，并通过 2/3 多数形成结果；若三票分散，只允许一次合案复议。

## 模块

- `crates/wenyuan-core`：领域模型、状态机、阶段转换、屏障、多数决和 Convergence 规则。
- `crates/wenyuan-provider`：统一 LLM Provider trait，包含 `MockProvider` 和 OpenAI-compatible Provider。
- `crates/wenyuan-agent`：三席提示词、并发阶段执行、JSON 解析、格式修复重试、取消检查。
- `crates/wenyuan-store`：SQLite migrations、Session/Artifacts 持久化、事件轨迹和恢复查询。
- `crates/wenyuan-server`：Axum API、SSE、后台执行、配置状态和静态前端托管。
- `web`：Vue 3 + TypeScript + Pinia + Vue Router 前端。

## 运行

后端默认使用 MockProvider，无需 API Key：

```powershell
cargo run -p wenyuan-server
```

前端开发服务器：

```powershell
cd web
pnpm install
pnpm dev
```

访问：

```text
http://127.0.0.1:3210
```

开发时也可以访问 Vite：

```text
http://127.0.0.1:5173
```

## 配置

复制 `.env.example` 为 `.env` 或直接设置环境变量。配置了 `WENYUAN_LLM_BASE_URL` 和 `WENYUAN_LLM_API_KEY` 后会使用 OpenAI-compatible Provider，否则使用 MockProvider。

Mock 场景通过 `WENYUAN_MOCK_SCENARIO` 控制：

- `success_majority`
- `timeout`
- `malformed_then_repair`
- `always_malformed`
- `single_seat_failure`
- `split_then_convergence`

## API

- `POST /api/sessions`
- `GET /api/sessions`
- `GET /api/sessions/:id`
- `POST /api/sessions/:id/start`
- `POST /api/sessions/:id/retry`
- `POST /api/sessions/:id/cancel`
- `GET /api/sessions/:id/events`
- `GET /api/health`
- `GET /api/config/status`

## 验证

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test

cd web
pnpm test
pnpm build
```

## 第一版边界

第一版不包含裁判 AI、第四 Agent、向量数据库、联网搜索、工具调用、用户系统、云部署、Tauri、完整 Event Sourcing 或任意数量 Agent。
