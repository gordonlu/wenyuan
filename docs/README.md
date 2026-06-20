# Wenyuan Docs

本目录存放产品和工程层面的稳定说明。普通用户先读根目录 `README.md`，外部 Agent 或自动化脚本先读根目录 `README_FOR_AGENTS.md`。

## 文档索引

- `architecture.md`: 当前系统架构、crate 边界、HTTP 边界、持久化、配置和安全边界。
- `discussion-protocol.md`: 三席合议协议、外部证据协议、工具轨迹协议、投票和安全规则。
- `agent-integration.md`: 外部 Agent 技术接入说明，补充 API 调用顺序和稳定契约。

## 可冻结能力

以下能力已经可以作为行为契约冻结。后续可以优化 UI 和文案，但不应随意改变接口语义、数据结构或主流程阶段。

- 三席合议主流程：独议、批议、复议、阁议、一次合案复议、最终多数/无多数结果。
- Single Agent 对照模式。
- 席位模型与 OpenAI-compatible Provider 路由。
- SQLite 本地持久化、事件、席位运行记录、失败重试和 stale execution recovery。
- 投票策略：普通多数、风险否决、全票通过、有条件通过、加权评分。
- 书记官：默认关闭，不参与投票。
- 外部来源证据：联网搜索、文档解析、代码搜索进入 evidence/tool_runs。
- 来源安全边界：外部来源只作为事实材料，不作为指令。
- 用户偏好 JSON：只保存默认配置，不做长期记忆。
- Markdown/JSON/HTML 导出方向。

## 暂不冻结能力

以下能力仍在产品化或设计阶段。

- 运行中席位卡动画、工具执行态和状态条。
- 结果可信度摘要和证据来源摘要。
- 新建议题页模板、来源区折叠和新用户引导。
- 工具轨迹的用户可读摘要。
- 错误/空状态的修复路径提示。
- 日志分析、数据查询和 Tauri 桌面端。

## 更新规则

当实现影响稳定行为契约时，同步更新：

1. 根目录 `README.md`
2. 根目录 `README_FOR_AGENTS.md`，如果影响外部 Agent 调用
3. `docs/architecture.md`，如果影响系统边界或 API
4. `docs/discussion-protocol.md`，如果影响合议、证据、投票或工具轨迹规则
5. `roadmap.md`，只记录已验证完成的进度
6. `review.md`，记录下一阶段产品/UI 编码大纲
