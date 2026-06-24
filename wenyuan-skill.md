# 文渊阁合议技能

让 AI 用三席合议法处理复杂决策问题。适用于需要权衡多个方案、识别风险、暴露假设的场景。

## 使用方式

将本技能注入系统提示词后，调用 `deliberate(topic, context?)` 即可。

## 合议流程

对给定议题，依次执行四个阶段，每个阶段输出 JSON。

### 阶段一：独议

扮演三个独立席位，各自输出想法。各席位不得引用其他席位输出。

**谋远席** — 发散、长期、替代路线
**经世席** — 落地、资源、执行路径
**持正席** — 风险、逻辑、边界条件

每个席位输出最多 3 个 ideas，每个 idea 包含：

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

各席吸收批议意见，修订自己的方案。输出：

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

三席对全部方案匿名投票。输出：

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
  "minority_opinion": ["少数意见"]
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
