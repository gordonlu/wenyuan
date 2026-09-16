# 协作会议实现 Review

## 目标

这次改造针对一个非常具体的使用痛点：当用户想同时询问 2～3 个不同 AI 时，不应反复复制问题、粘贴回答、切换窗口，再人工把结果拼在一起。

文渊阁应该负责共享上下文、编排互动、收敛结果和处理失败；各 Agent 应围绕用户真正的目标寻找最佳实现，而不是因为“多 Agent”就被强制分配不同立场。

## 对现有实现的主要发现

### 1. 旧三席把“多 Agent”误绑定到“固定视角”

旧 prompt 将三个 Seat 固定成：

- 谋远：长期 / 替代路线
- 经世：落地 / 成本
- 持正：风险 / 边界

这种设计在部分战略讨论里有帮助，但作为默认协作机制会产生人为分歧：本来三个强模型可能都会选择同一个最佳实现，却被 prompt 推着分别“找长期机会 / 找执行路径 / 找风险”。

本分支把三个旧 Seat prompt 改成 **goal-driven peer**：席位名称只是兼容性 ID，所有参与者先解决同一个用户目标；只有真正有信息量的差异才互动。

### 2. 旧执行器实际上硬编码为 3 Agent

`AgentRunner::run_three` 遍历固定 `SeatKind::ALL`，并通过 `phase_barrier_completed` 要求三个 Seat 全部完成。

结果是：

- 无法自然表达 2 Agent 会议
- 一个 Provider 超时 / Agent 掉线会让整个 phase 失败
- 不能区分“3 人会议退化成仍然有效的 2 人会议”和“2 人会议只剩 1 人，已经失去多 Agent 意义”

为了避免对旧 Session / Store schema 做高风险大改，本分支没有强行把旧 Runner 改成动态成员，而是新增独立的 MCP Meeting Coordinator，专门处理 2～3 个外部 Agent。

### 3. 旧流程没有集中式用户追问仲裁

不同 Agent 在同一个议题中可能分别向用户问：

- 预算是多少？
- 预算上限？
- 你最多愿意花多少钱？

语义上是一个问题，但用户会被连续打断。

MCP Meeting 新增 Question Broker：

- `missing_key` 相同直接合并
- 文本高度重合自动合并
- 同一时间只有一个 active question
- 其他 distinct questions 排队
- 一个答案广播给所有 Agent

这样“多 Agent”不会变成“多倍追问”。

### 4. 旧流程缺少跨客户端协作总线

现有 Provider routing 能让内部三个 Seat 使用不同模型，但 Codex、Claude Code、OpenCode 等独立 Agent 客户端无法加入同一个文渊阁 Session。

新增 `wenyuan-mcp` crate，把文渊阁提升为本地协作总线。外部 Agent 不需要彼此直接通信，只需要连接同一个 localhost MCP endpoint。

### 5. 重试机制与“掉线恢复”不是一回事

旧系统有 session/seat retry，但多 Agent 实时协作需要额外语义：

- Agent 身份不能因为重连而重复创建
- 获取任务重复调用不能生成重复 work
- 提交因网络超时重试不能产生重复 turn
- 等待期间需要 liveness
- 3→2 和 2→1 应有不同策略

MCP Meeting 使用 `participant_id + resume_token`、幂等 `next_task`、幂等 `submit_turn` 和 heartbeat 实现这些语义。

## 新会议模型

```text
用户目标
   │
   ▼
Initial：2～3 Agent 独立给出最佳答案
   │
   ▼
Cross Review：只处理实质分歧 / 遗漏 / 更优实现
   │
   ▼
Synthesis：临时综合者选最佳实现并保留真实异议
   │
   ▼
最终结果
```

这个流程刻意不加入“投票必须选一个赢家”。对很多代码、产品和研究问题，最好的最终方案往往是吸收多个回答中最强的部分，而不是 2/3 多数投票。

旧内部 Session 的投票能力继续保留，用于确实需要表决的场景。

## Agent 数量

默认只支持 2～3 个参与者。

原因：

- 第 4 个 Agent 开始，延迟和 token 成本明显增加
- 交叉阅读量从线性快速变大
- 大部分实际问题 2～3 个强模型已经能暴露主要分歧
- 多于 3 个通常应该先做并行采样再摘要，而不是所有 Agent 相互对话

因此 MCP schema 对 `participant_limit` 直接限制为 2 或 3。

## 掉线策略

### 3 Agent → 2 Agent

允许。

一个 Agent 在 heartbeat timeout + grace 后仍未恢复，则当前未完成 turn 被标记 `skipped`，会议继续。最终结果带 degraded 信息，避免隐藏失败。

### 2 Agent → 1 Agent

不允许静默降级。

会议暂停，等待原 Agent 使用 `resume_token` 回来。否则“多 Agent 会议信息”会变成误导。

### Synthesis Agent 掉线

如果仍有至少两个健康 Agent，重新选择一个综合者继续同一个 synthesis turn，而不是重新跑整场会议。

## 交互去重

不仅去重用户问题，也避免 Agent 之间的低价值往返：

- Initial 完全独立
- Cross Review 明确禁止重复共识
- 可以直接同意对方，不要求制造异议
- Synthesis 不平均方案，而是选择最强实现
- `next_task` 在未 submit 前返回同一 turn
- `submit_turn` 重复调用返回 idempotent success

## 为什么 MCP Meeting 暂时与旧 Session 并行

直接把 `SeatKind`、SQLite schema、旧导出结构、前端 workspace、follow-up 和投票全部改成动态 participant，会让这次功能变成一次全项目迁移，风险和改动面远大于用户当前需求。

本分支选择：

1. 保留旧 Session，避免破坏已有历史、导出和 UI。
2. 先把旧 prompt 从固定角色改成平等协作语义。
3. 新建独立 `wenyuan-mcp` 协调层，解决跨 Agent、2～3 人、掉线、去重追问这些真正缺失的能力。
4. MCP 协议稳定后，再决定是否把旧 UI 的 Session 模型迁移到统一 Participant 模型。

## 后续建议

完成这一版并验证真实 Codex + Claude Code / OpenCode 互通后，下一步优先级应该是：

1. 在文渊阁 Workspace 中增加“外部 Agent 会议”视图。
2. 把 MCP meeting event 投影到现有时间线，而不是复制一套复杂 UI。
3. 允许用户在 3 Agent lobby 中以 2 Agent 提前开会（本分支协议已支持）。
4. 收集真实会议数据后，再决定是否淘汰固定 `SeatKind`。
5. 只有确有需求时再做 4+ Agent；不要预先泛化。
