---
name: wenyuan-deliberation
description: Use the Wenyuan three-seat deliberation method to analyze complex decisions, expose assumptions, compare options, and identify risks.
version: 1.0.1
metadata:
  openclaw:
    skillKey: wenyuan
    emoji: "🏛️"
---

# 文渊阁合议技能

Use this skill for complex decisions that need structured trade-off analysis, risk identification, and assumption tracking.

This is a lightweight prompt skill. It uses the current agent's own reasoning model and does not require external APIs or credentials.

---

## When to Use

Use this skill when the user asks for:

- complex decision analysis
- product or architecture direction
- risk review
- trade-off comparison
- strategic planning
- assumption exposure
- "文渊阁合议"
- "三席合议"

Do not use this skill for simple factual questions, direct code edits, translation, or short answers unless the user explicitly requests deliberation.

---

## Core Idea

Analyze the topic through three distinct seats:

### 谋远席

Focus on long-term direction, alternatives, optionality, second-order effects, and unconventional opportunities.

### 经世席

Focus on execution, cost, feasibility, resources, sequencing, and practical constraints.

### 持正席

Focus on risks, logic gaps, hidden assumptions, failure modes, boundaries, and what must be verified.

The seats should stay meaningfully different. Do not make them all agree too early.

---

## Default Lightweight Process

By default, run a compact internal deliberation.

Do not generate or output full internal JSON.

Use this internal process:

1. **独议**  
   Each seat forms one concise position.

2. **批议**  
   Each seat identifies the strongest weakness in the other perspectives.

3. **复议**  
   Revise the recommendation by preserving useful disagreement.

4. **阁议**  
   Produce a final recommendation, key assumptions, risks, minority opinion, and next actions.

Keep the process compact.

Avoid long lists.

Avoid exhaustive schemas.

Avoid pretending that uncertainty has disappeared.

---

## Progress Feedback

When starting a deliberation, show a short visible status line before the final answer if the host supports streaming:

`🏛️ 文渊阁合议开始：将从谋远、经世、持正三席进行压缩合议。`

Do not emit long intermediate reasoning.

Do not output full internal phase content unless the user explicitly asks.

If the host only displays the final response, include a compact progress summary in the final answer:

```md
## 合议进度

- 独议：已完成
- 批议：已压缩吸收
- 复议：已完成
- 阁议：已形成建议
