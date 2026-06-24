---
name: wenyuan-deliberation
description: 三席合议决策法 — 把复杂问题交给三个不同立场的 AI 席位分别思考、互相批议、修订方案并投票。
version: 1.0.0
metadata:
  openclaw:
    emoji: "\u2696\uFE0F"
    homepage: https://github.com/gordonlu/wenyuan
    skillKey: deliberate
---

让 AI 用三席合议法处理复杂决策问题。适用于需要权衡多个方案、识别风险、暴露假设的场景。

## 使用方式

加载本技能后，调用 `deliberate` 并传入议题：

```
deliberate(topic: string, context?: string)
```

技能会自动执行四阶段合议，返回最终结论。

## 合议流程

对给定议题，依次执行四个阶段，每个阶段输出 JSON。

### 阶段一：独议

扮演三个独立席位，各自输出想法。各席位不得引用其他席位输出。

| 席位 | 视角 | 关注点 |
|------|------|--------|
| 谋远席 | 未来 | 长期机会、替代路线、系统性思考 |
| 经世席 | 落地 | 资源约束、执行路径、成本收益 |
| 持正席 | 底线 | 风险、伦理、边界条件、反例 |

每个席位输出最多 3 个 ideas，结构：

```json
{
  "title": "方案名称",
  "summary": "一句话概括",
  "rationale": "为什么可行",
  "unconventional": false,
  "assumptions": ["依赖条件"],
  "risks": ["潜在风险"]
}
```

### 阶段二：批议

每席阅读另外两席的方案，对每个其他席位输出一条 review：

```json
{
  "target_seat": "mouyuan",
  "strongest_point": "最大亮点",
  "weakest_point": "最大弱点",
  "challenge": "核心质疑",
  "suggested_improvement": "改进建议"
}
```

### 阶段三：复议

各席吸收批议意见，修订自己的方案：

```json
{
  "title": "方案名称",
  "summary": "修订后摘要",
  "adopted_points": ["采纳了哪些批议"],
  "rejection_reasons": ["拒绝了哪些及理由"],
  "risks": ["风险清单"],
  "confidence": 0.0
}
```

### 阶段四：投票

三席对全部方案匿名投票：

```json
{
  "votes": [
    {
      "proposal_ref": "方策一",
      "reason": "投票理由",
      "confidence": 0.0,
      "final_choice": false
    }
  ],
  "majority_reached": true,
  "majority_proposal": "方策一",
  "majority_reason": "多数方理由",
  "minority_opinion": ["少数意见"],
  "unresolved_issues": ["未解决的分歧"]
}
```

## 输出规则

1. 每个阶段只输出要求的 JSON，不要输出解释文字
2. 缺少信息时使用空字符串或空数组，不要省略字段
3. 字段名与 schema 完全一致
4. 字符串中不要包含未转义的双引号

## 注意事项

- 三席应保持视角差异，不要趋同
- 批议应提出实质性质疑，避免"总体认同"
- 投票应基于方案本身，不因席位身份投票
- 如果三票分散、无多数，说明分歧过大，应在结果中突出未解决问题
- 确保 assumptions 和 risks 不重复，每个 assumption 应可验证

## 与文渊阁完整版的区别

本技能提供的是"脑内三席合议"——由单一模型扮演三个角色。文渊阁完整版（wenyuan-server）支持：

- 真实多模型：三个席位可接入不同 LLM Provider
- 持久化：SQLite 存储全部议题记录
- 证据池：管理外部来源、验证状态
- 审计：完整决策轨迹回溯
- 桌面应用：Tauri 桌面壳 + 系统托盘
