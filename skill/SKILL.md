---
name: wenyuan-deliberation
description: Use the Wenyuan three-seat deliberation method to analyze complex decisions, expose assumptions, compare options, and identify risks.
version: 1.0.0
metadata:
  openclaw:
    skillKey: wenyuan
    emoji: "🏛️"
---

# 文渊阁合议技能

Use this skill when the user faces a complex decision that requires structured deliberation, competing perspectives, risk identification, and assumption tracking.

文渊阁 is a deliberation protocol. It uses three distinct seats to examine a question before forming a final council recommendation.

This skill does not require external APIs, credentials, or tools. It runs using the current agent's own reasoning model.

---

## When to Use

Use this skill when the user asks for help with:

* strategic decisions
* product direction
* architecture choices
* trade-off analysis
* planning under uncertainty
* risk review
* choosing between multiple options
* exposing hidden assumptions
* avoiding premature convergence

Examples:

* "帮我判断这个项目方向值不值得做"
* "这几个方案怎么选"
* "帮我从风险和长期收益角度分析"
* "用文渊阁合议一下"
* "帮我做三席合议"
* "What are the trade-offs between these options?"

---

## When Not to Use

Do not use this skill for:

* simple factual questions
* direct code edits without strategic trade-offs
* pure translation or rewriting
* straightforward debugging with one clear fix
* tasks where the user explicitly wants a short answer
* cases where a normal concise answer is enough

If the user asks for a quick answer, answer directly unless they explicitly request deliberation.

---

## Core Principle

The goal is not to make the answer look more elaborate.

The goal is to make the decision more:

* explicit
* balanced
* falsifiable
* risk-aware
* assumption-aware
* easier to revisit later

Do not force consensus. Real disagreement is useful.

---

## The Three Seats

### 谋远席 — Long-Range Seat

Focus on:

* long-term consequences
* alternative routes
* second-order effects
* non-obvious opportunities
* future optionality
* unconventional paths

Avoid:

* merely repeating practical constraints
* converging too early
* giving the same answer as 经世席

---

### 经世席 — Practical Seat

Focus on:

* feasibility
* execution path
* cost and benefit
* resources required
* sequencing
* operational constraints
* implementation risk

Avoid:

* vague vision without steps
* overengineering
* ignoring current constraints

---

### 持正席 — Integrity Seat

Focus on:

* logical gaps
* hidden assumptions
* failure modes
* boundary conditions
* safety
* compliance
* ethics
* reversibility
* evidence quality

Avoid:

* generic pessimism
* rejecting everything without concrete reasons
* listing risks without explaining how to verify or reduce them

---

## Internal Deliberation Process

Run four internal phases:

1. Independent Deliberation
2. Critique
3. Revision
4. Final Council

By default, do not expose the full internal JSON to the user. Use it as an internal reasoning protocol, then present a clear final answer.

Only show the full JSON if the user explicitly asks for:

* "完整 JSON"
* "展开四阶段"
* "显示三席过程"
* "show the full deliberation"
* "give me the raw process"

---

## Phase 1: Independent Deliberation

Each seat independently proposes up to three ideas.

Rules:

* Each idea must have a stable `proposal_id`.
* `proposal_id` format: `{seat}_{index}`.
* Valid seat IDs: `mouyuan`, `jingshi`, `chizheng`.
* Seats must not reference other seats during this phase.
* At least one idea from 谋远席 should consider a non-obvious or unconventional path when appropriate.

Internal schema:

```json
{
  "phase": "independent",
  "seats": [
    {
      "seat": "mouyuan",
      "ideas": [
        {
          "proposal_id": "mouyuan_1",
          "title": "",
          "summary": "",
          "rationale": "",
          "unconventional": false,
          "assumptions": [],
          "risks": []
        }
      ]
    },
    {
      "seat": "jingshi",
      "ideas": []
    },
    {
      "seat": "chizheng",
      "ideas": []
    }
  ]
}
```

---

## Phase 2: Critique

Each seat reviews proposals from the other seats.

Rules:

* Every critique must target a specific `proposal_id`.
* Critique must be substantive.
* Avoid empty praise such as "总体认同" or "可以参考".
* Critique should improve the final decision, not merely attack.

Internal schema:

```json
{
  "phase": "critique",
  "reviews": [
    {
      "reviewer_seat": "mouyuan",
      "target_seat": "jingshi",
      "target_proposal_id": "jingshi_1",
      "strongest_point": "",
      "weakest_point": "",
      "challenge": "",
      "suggested_improvement": ""
    }
  ]
}
```

---

## Phase 3: Revision

Each seat revises its own proposal after considering critique.

Rules:

* Revised proposals must retain or reference their original `proposal_id`.
* A seat may merge, narrow, discard, or strengthen its own proposal.
* The revision must explain what was adopted and what was rejected.
* `confidence` must be between 0 and 1.
* Do not simply repeat the independent proposal.

Internal schema:

```json
{
  "phase": "revision",
  "revised_proposals": [
    {
      "seat": "mouyuan",
      "proposal_id": "mouyuan_1",
      "title": "",
      "summary": "",
      "adopted_points": [],
      "rejection_reasons": [],
      "assumptions": [],
      "risks": [],
      "confidence": 0.0
    }
  ]
}
```

---

## Phase 4: Final Council

The three seats vote anonymously on the revised proposals.

Rules:

* Each seat casts one vote.
* Vote based on the proposal, not the identity of the seat that proposed it.
* A proposal with at least two votes becomes the majority proposal.
* If all three votes are split, set `majority_reached` to `false`.
* Do not invent consensus when disagreement remains.
* Preserve meaningful minority opinions.

Internal schema:

```json
{
  "phase": "final_council",
  "votes": [
    {
      "proposal_id": "",
      "reason": "",
      "confidence": 0.0
    }
  ],
  "majority_reached": true,
  "majority_proposal_id": "",
  "majority_reason": "",
  "minority_opinions": [],
  "unresolved_issues": [],
  "recommended_next_actions": []
}
```

---

## Default User-Facing Output

Unless the user explicitly requests the full JSON, present the final result in this format:

```markdown
## 阁议结论

推荐方案：...

核心理由：...

## 关键假设

1. ...
2. ...
3. ...

## 主要风险

1. ...
2. ...
3. ...

## 少数意见

...

## 仍未验证的问题

1. ...
2. ...

## 建议下一步

1. ...
2. ...
3. ...
```

Use concise but complete explanations.

The final answer should help the user decide what to do next.

---

## Full JSON Output

If the user asks for the complete deliberation process, output:

```json
{
  "topic": "",
  "context_summary": "",
  "independent": {},
  "critique": {},
  "revision": {},
  "final_council": {}
}
```

When outputting JSON:

1. Output valid JSON only.
2. Do not include explanatory text outside JSON.
3. Do not omit required fields.
4. Use empty strings, empty arrays, `false`, or `null` when information is missing.
5. Do not include unescaped double quotes inside string values.
6. Keep field names stable.

---

## Quality Rules

Maintain clear differences between the three seats.

批议 must contain real challenges, not decorative comments.

复议 must show real changes or explain why no change was made.

阁议 must explain:

* why the recommended proposal is preferred
* which assumptions it depends on
* what could make it fail
* whether minority opinions should be preserved
* what should be verified next

Do not overstate certainty.

Do not describe a single-model role-play as true multi-model independent deliberation.

Do not present the result as fact. It is a structured decision recommendation based on the available context.

If important information is missing, say what is missing and how it affects the recommendation.

---

## Completion Standard

A good 文渊阁 answer should make the decision more inspectable.

Before finalizing, check:

* Did each seat contribute a distinct perspective?
* Did the critiques challenge real weaknesses?
* Did the revised proposals change or sharpen the original ideas?
* Did the final recommendation preserve unresolved risks?
* Did the answer identify assumptions that should be verified?
* Did the answer avoid pretending that uncertainty has disappeared?

The purpose of this skill is not to produce a longer answer.

The purpose is to produce a better decision.
