<template>
  <section class="panel decision">
    <header class="section-head">
      <h2>决策结论</h2>
      <div class="phase-labels">
        <span v-if="votePolicy && mode !== 'single_agent'" class="badge flat">{{ voteStrategyLabel }}</span>
        <span v-if="decision?.has_risk_blocker" class="badge danger">有风险阻塞</span>
        <span :class="['badge', decision?.status === 'conditionally_adopted' ? 'warn' : decision?.status === 'majority_reached' ? 'ok' : 'warn']">
          {{ decision?.status === 'majority_reached' ? '形成多数' : decision?.status === 'conditionally_adopted' ? '有条件通过' : '未形成多数' }}
        </span>
      </div>
    </header>

    <div v-if="decision?.selected_proposal" class="result-block">
      <h3>{{ decision.selected_proposal.title }}</h3>
      <div class="formatted-summary" v-html="renderedSummary" />
      <p v-if="decision.status === 'conditionally_adopted'" class="muted" style="font-size: 13px">三席已有倾向，但还需要先处理采纳条件。</p>
      <dl>
        <dt>有效票数</dt>
        <dd>{{ decision.vote_count }}</dd>
        <dt>自投数</dt>
        <dd>{{ decision.self_vote_count }}</dd>
      </dl>
    </div>

    <div v-if="actionItems.length" class="result-block action-block">
      <h3>行动清单</h3>
      <ul class="action-list">
        <li v-for="item in actionItems" :key="item.text" :class="item.kind">
          <span v-if="item.kind === 'do'" class="action-marker">▶</span>
          <span v-else-if="item.kind === 'caution'" class="action-marker caution">⚠</span>
          <span v-else class="action-marker question">?</span>
          {{ item.text }}
        </li>
      </ul>
    </div>

    <div v-if="concerns.length" class="result-block">
      <h3>需要注意</h3>
      <ul>
        <li v-for="c in concerns" :key="c">{{ c }}</li>
      </ul>
    </div>

    <details v-if="hasDetail" class="detail-collapse">
      <summary>查看详细记录</summary>
      <div v-if="decision?.majority_reasons?.length" class="result-block">
        <h4>多数理由</h4>
        <ul>
          <li v-for="reason in decision.majority_reasons" :key="reason">{{ reason }}</li>
        </ul>
      </div>
      <div v-if="decision?.minority_choices?.length" class="minority-detail">
        <h4>少数方说明</h4>
        <div v-for="choice in decision.minority_choices" :key="choice.seat" class="minority-card">
          <p>
            <strong>{{ seatLabels[choice.seat] }}</strong>：{{ choice.reason }}
            <span v-if="choice.has_risk_warning" class="badge danger" style="margin-left: 8px">提示风险</span>
          </p>
          <p v-if="choice.reassessment_condition" class="muted">重新评估条件：{{ choice.reassessment_condition }}</p>
        </div>
      </div>
      <div v-if="decision?.adoption_conditions?.length" class="result-block">
        <h4>采纳条件</h4>
        <ul>
          <li v-for="item in decision.adoption_conditions" :key="item">{{ item }}</li>
        </ul>
      </div>
      <div v-if="decision?.reassessment_conditions?.length" class="result-block">
        <h4>重新评估条件</h4>
        <ul>
          <li v-for="condition in decision.reassessment_conditions" :key="condition">{{ condition }}</li>
        </ul>
      </div>
      <div v-if="decision?.unresolved_questions?.length" class="result-block">
        <h4>未决问题</h4>
        <ul>
          <li v-for="q in decision.unresolved_questions" :key="q">{{ q }}</li>
        </ul>
      </div>
    </details>
  </section>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { seatLabels, voteStrategyLabels } from '../domain/session'
import type { Decision, VotePolicy } from '../domain/session'
import { renderMarkdown } from '../utils/markdown'

const props = defineProps<{
  decision?: Decision | null
  votePolicy?: VotePolicy | null
  mode?: string
}>()

const voteStrategyLabel = computed(() =>
  props.votePolicy ? (voteStrategyLabels[props.votePolicy.strategy] ?? props.votePolicy.strategy) : ''
)

const renderedSummary = computed(() =>
  props.decision?.selected_proposal?.summary
    ? renderMarkdown(props.decision.selected_proposal.summary)
    : ''
)

interface ActionItem { text: string; kind: 'do' | 'caution' | 'question' }

const actionItems = computed<ActionItem[]>(() => {
  const items: ActionItem[] = []
  const d = props.decision
  if (!d) return items
  if (d.selected_proposal?.implementation_path?.trim()) {
    items.push({ text: d.selected_proposal.implementation_path, kind: 'do' })
  }
  for (const step of d.next_steps ?? []) {
    if (step.trim()) items.push({ text: step, kind: 'do' })
  }
  for (const condition of d.adoption_conditions ?? []) {
    if (condition.trim()) items.push({ text: condition, kind: 'caution' })
  }
  return items
})

const concerns = computed<string[]>(() => {
  const d = props.decision
  if (!d) return []
  const out: string[] = []
  for (const q of d.unresolved_questions ?? []) {
    if (q.trim()) out.push(q)
  }
  for (const choice of d.minority_choices ?? []) {
    if (choice.reason.trim()) {
      const prefix = seatLabels[choice.seat] ?? choice.seat
      out.push(`${prefix}：${choice.reason}`)
    }
  }
  return out
})

const hasDetail = computed(() => {
  const d = props.decision
  if (!d) return false
  return !!(
    d.majority_reasons?.length ||
    d.minority_choices?.length ||
    d.adoption_conditions?.length ||
    d.reassessment_conditions?.length ||
    d.unresolved_questions?.length
  )
})
</script>

<style scoped>
.formatted-summary {
  line-height: 1.7;
  font-size: 14px;
}
.formatted-summary p {
  margin: 0.5em 0;
}
.formatted-summary strong {
  color: var(--color-text);
  font-weight: 600;
}
.formatted-summary ul, .formatted-summary ol {
  padding-left: 1.5em;
  margin: 0.4em 0;
}
.formatted-summary li {
  margin: 0.2em 0;
}
.formatted-summary h2,
.formatted-summary h3,
.formatted-summary h4 {
  margin: 0.8em 0 0.3em;
  color: var(--color-text);
}
.action-block {
  background: #f8faf5;
  border: 1px solid #d8e0ce;
  border-radius: var(--radius-md);
  padding: var(--space-md);
  margin: var(--space-md) 0;
  color: var(--color-text);
}
.action-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.action-list li {
  padding: 6px 10px;
  background: #fff;
  border-radius: var(--radius-sm);
  border-left: 6px solid var(--color-accent);
  font-size: 13px;
  line-height: 1.5;
  color: var(--color-text);
}
.action-list li.caution {
  border-left-color: var(--color-warning-border);
}
.action-list li.question {
  border-left-color: var(--color-text-dim);
}
.action-marker {
  display: inline-block;
  width: 18px;
  font-size: 12px;
  color: var(--color-accent);
}
.action-marker.caution {
  color: var(--color-warning);
}
.action-marker.question {
  color: var(--color-text-dim);
}
.action-block ul li,
.action-block p,
.action-block dd,
.action-block h3 {
  color: var(--color-text);
}
.detail-collapse {
  margin-top: var(--space-md);
}
.detail-collapse summary {
  cursor: pointer;
  font-size: 13px;
  color: var(--color-text-muted);
  padding: 4px 0;
  user-select: none;
}
.detail-collapse summary:hover {
  color: var(--color-text);
}
.detail-collapse[open] {
  border-top: 1px solid var(--color-border-light);
  padding-top: var(--space-md);
}
</style>
