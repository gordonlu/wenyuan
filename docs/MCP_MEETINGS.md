# MCP 多 Agent 协作会议

文渊阁现在可以作为一个本地 MCP 协作总线，让 2～3 个外部 Agent（例如 Codex、Claude Code、OpenCode 或其他支持 HTTP MCP 的客户端）进入同一个会议。

核心目标不是让 Agent 扮演预设立场，而是让它们围绕**同一个用户目标**独立给出最佳方案、只对实质差异进行交叉校验，最后由临时综合者形成结果。

## 启动

通过文渊阁桌面版或：

```bash
cargo run -p wenyuan-app
```

默认同时启动 MCP 端点：

```text
http://127.0.0.1:3847/mcp
```

可在 `.env` 调整：

```env
WENYUAN_MCP_ENABLED=true
WENYUAN_MCP_PORT=3847
```

MCP 仅监听 `127.0.0.1`。如果端口被占用，主文渊阁仍会正常启动，只会记录 MCP 启动失败。

## 客户端配置

不同 Agent 的 MCP 配置文件格式不同，核心信息只有同一个 HTTP 地址。例如支持 URL 型 MCP 配置的客户端通常类似：

```json
{
  "mcpServers": {
    "wenyuan": {
      "url": "http://127.0.0.1:3847/mcp"
    }
  }
}
```

所有需要参会的 Agent 都指向同一个本地文渊阁 MCP 端点。

## 推荐流程

### 1. 创建会议

由一个 Agent（通常是用户当前正在对话的 Agent）调用：

`wenyuan_create_meeting`

参数：

- `title`
- `goal`：用户真正要解决的问题，而不是给不同 Agent 分配的角色
- `context`
- `participant_limit`：2 或 3
- `max_rounds`：1 或 2，默认 2

返回：

- `meeting_id`
- `join_code`
- `owner_token`

`owner_token` 只由主持侧保存，用于回答会议过程中唯一的用户追问。

### 2. Agent 加入

每个 Agent 调用 `wenyuan_join_meeting`，使用相同 `meeting_id` / `join_code`，并给自己一个可识别的 `display_name`。

加入成功后会得到：

- `participant_id`
- `resume_token`
- 建议 heartbeat 间隔

**掉线重连必须复用 `resume_token`**，否则文渊阁会拒绝创建同名重复参与者。

如果创建了 3 人会议但最终只需要 2 个 Agent，可由主持者调用 `wenyuan_start_meeting` 提前开始。

### 3. 拉取任务并提交

参会 Agent 循环：

1. `wenyuan_next_task`
2. 完成返回的任务
3. `wenyuan_submit_turn`
4. 如果返回 `waiting_for_peers`，等待期间调用 `wenyuan_heartbeat`

`next_task` 是幂等的：同一 turn 未提交前重复调用不会生成新任务。

`submit_turn` 同样支持幂等重试：网络抖动导致重复提交不会重复计入。

## 会议逻辑

### Initial

所有 Agent 收到**完全相同的用户目标和上下文**，独立给出自己认为最好的方案。

没有“战略席 / 落地席 / 风险席”之类的强制分工。不同答案应该来自模型本身的判断差异，而不是 prompt 人为制造差异。

### Cross Review

只有在以下情况才值得互动：

- 实质结论冲突
- 关键条件遗漏
- 错误假设
- 明显更好的实现路径

已经一致的内容不要求换一种说法重复。Agent 可以直接承认另一个方案更好，不要求“每个 Agent 都必须有不同意见”。

### Synthesis

文渊阁从在线参与者中选择一个临时综合者。它只是编辑者，不拥有更高权威。

综合时要求：

- 选择最适合用户目标的实现，而不是机械平均
- 保留真实分歧和不确定性
- 不伪造共识
- 不重复询问已回答的问题

## 避免重复追问用户

Agent 只有在“缺失信息会实质改变结论，并且不能通过明确假设继续推进”时才应调用 `wenyuan_ask_user`。

文渊阁有一个集中 Question Broker：

1. 相同 `missing_key` 的问题自动合并。
2. 完全相同或高度重合的问题自动合并。
3. 同一时刻只允许一个 distinct user question 处于 `open`。
4. 后续不同问题进入 `deferred` 队列。
5. `wenyuan_answer_user` 回答后，答案会共享给所有 Agent。
6. 已回答问题不会再次进入 active queue。

例如两个 Agent 分别问：

- “用户的预算上限是多少？”
- “预算上限是多少”

若都使用 `missing_key=budget`，用户只会看到一次问题。

## Agent 掉线处理

默认：

- heartbeat timeout：45 秒
- offline grace：60 秒

### 三 Agent 会议

若一个 Agent 超过 timeout + grace 仍未恢复：

- 它尚未完成的当前阶段 turn 标记为 `skipped`
- 会议允许以剩余 2 个 Agent 继续
- 最终结果标记 `degraded=true`
- 如果掉线的是 synthesis Agent，会把 synthesis turn 重新分配给在线 Agent

### 两 Agent 会议

若一个 Agent 掉线：

**不会自动退化成单 Agent。**

会议进入 `paused`，等待掉线 Agent 使用原 `resume_token` 重连。这样避免用户以为自己得到的是“多 Agent 共识”，实际上只有一个模型完成了回答。

## 状态恢复

MCP meeting state 默认保存到文渊阁 data directory：

```text
mcp-meetings.json
```

应用重启后会恢复会议、turn、问题队列、participant resume token 等状态。

## MCP tools

| Tool | 用途 |
|---|---|
| `wenyuan_create_meeting` | 创建 2～3 Agent 会议 |
| `wenyuan_start_meeting` | 主持者提前以已加入的 2 个 Agent 开始 |
| `wenyuan_join_meeting` | 加入 / 断线重连 |
| `wenyuan_next_task` | 获取当前任务，幂等 |
| `wenyuan_submit_turn` | 提交当前 turn，幂等 |
| `wenyuan_ask_user` | 请求必要用户信息，自动去重排队 |
| `wenyuan_answer_user` | 主持者一次回答并广播给所有 Agent |
| `wenyuan_heartbeat` | 等待时刷新在线状态 |
| `wenyuan_meeting_status` | 查看参与者、阶段、问题、退化状态与最终结果 |

## 给外部 Agent 的最短指令

可以直接告诉一个已经连接文渊阁 MCP 的 Agent：

> 使用文渊阁 MCP 参加这次协作会议。围绕用户目标给出你认为整体最好的方案，不扮演固定立场；只对实质分歧或遗漏互动。使用 next_task / submit_turn 完成你的 turn，等待期间 heartbeat。只有真正会改变结论的缺失信息才 ask_user，并复用 resume_token 处理断线。

## 当前边界

这一版 MCP meeting 与旧的内部 `three_seat` session 并行存在，目的是先把“跨客户端的 2～3 Agent 真协作”跑通，而不是一次性重写旧 Session / Store / UI。

旧三席的 prompt 已改成 goal-driven peer semantics；席位名称继续保留只是为了数据库与 API 兼容。

后续如果要在桌面 UI 里直接显示 MCP 会议房间、参会 Agent 和问答队列，可以在不改变 MCP 协议的前提下把 `MeetingHub` 接入现有工作区视图。
