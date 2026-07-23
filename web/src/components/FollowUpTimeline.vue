<template>
  <section class="panel">
    <div class="row-head">
      <h2>续议演化链</h2>
      <span class="badge flat" v-if="turns.length">{{ turns.length }} 轮</span>
    </div>

    <div v-if="turns.length === 0" class="muted" style="padding: 12px 0">暂无续议记录。</div>

    <div v-else class="impact-chain">
      <div v-for="(turn, i) in sortedTurns" :key="turn.id" class="chain-step">
        <div class="chain-connector" v-if="i > 0">
          <span class="chain-arrow">↓</span>
        </div>

        <div class="chain-card" :class="`impact-${impactClass(turn.impact)}`">
          <!-- step 1: mode -->
          <div class="chain-row">
            <span class="chain-badge mode-badge">{{ followUpModeLabels[turn.mode] }}</span>
            <time class="muted">{{ formatTime(turn.created_at) }}</time>
          </div>

          <!-- step 2: user input (if any) -->
          <div v-if="turn.user_input" class="chain-row">
            <span class="chain-label">输入</span>
            <p class="chain-text">{{ turn.user_input }}</p>
          </div>

          <!-- step 3: result -->
          <div class="chain-row">
            <span class="chain-label">结果</span>
            <pre class="chain-pre">{{ formatResult(turn.result_json) }}</pre>
          </div>

          <!-- step 4: impact -->
          <div class="chain-impact" :class="`impact-bar-${impactClass(turn.impact)}`">
            <span class="chain-impact-label">对原结论的影响</span>
            <span :class="['badge', impactBadge(turn.impact)]">{{ followUpImpactLabels[turn.impact] }}</span>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { followUpModeLabels, followUpImpactLabels, type FollowUpTurn } from '../domain/session'

const props = defineProps<{ turns: FollowUpTurn[] }>()

const sortedTurns = computed(() =>
  [...props.turns].sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime()),
)

function impactBadge(impact: string) {
  const map: Record<string, string> = {
    changes_decision: 'danger',
    suggests_re_deliberation: 'warn',
    raises_new_risk: 'warn',
    adds_action_item: 'ok',
    adds_adoption_condition: 'ok',
    clarifies_decision: 'ok',
    no_change: 'flat',
  }
  return map[impact] ?? 'flat'
}

function impactClass(impact: string) {
  return impactBadge(impact).replace('danger', 'danger').replace('warn', 'warn')
}

function formatTime(iso: string) {
  try {
    return new Date(iso).toLocaleString('zh-CN', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
  } catch { return iso }
}

function formatResult(value: unknown): string {
  if (typeof value === 'string') return value
  try { return JSON.stringify(value, null, 2) } catch { return String(value) }
}
</script>

<style scoped>
.impact-chain {
  display: flex;
  flex-direction: column;
  gap: 0;
}
.chain-step {
  display: flex;
  flex-direction: column;
}
.chain-connector {
  text-align: center;
  padding: 2px 0;
}
.chain-arrow {
  font-size: 16px;
  color: var(--color-text-dim);
}
.chain-card {
  border: 1px solid var(--color-border-light);
  border-radius: var(--radius-md);
  background: var(--color-surface);
  overflow: hidden;
}
.chain-card.impact-danger {
  border-left: 3px solid var(--color-danger);
}
.chain-card.impact-warn {
  border-left: 3px solid var(--color-warning-text);
}
.chain-card.impact-ok {
  border-left: 3px solid var(--color-accent);
}

.chain-row {
  padding: 8px 12px;
  border-bottom: 1px solid var(--color-border-light);
  display: flex;
  gap: 10px;
  align-items: flex-start;
}
.chain-row:last-of-type {
  border-bottom: none;
}
.chain-row time {
  margin-left: auto;
  font-size: 12px;
  white-space: nowrap;
}

.chain-badge {
  display: inline-block;
  padding: 2px 10px;
  border-radius: 10px;
  font-size: 12px;
  font-weight: 700;
}
.mode-badge {
  background: var(--color-bg-subtle);
  color: var(--color-text);
  border: 1px solid var(--color-border-light);
}

.chain-label {
  font-size: 11px;
  font-weight: 700;
  color: var(--color-text-dim);
  text-transform: uppercase;
  letter-spacing: 0.04em;
  min-width: 32px;
  flex-shrink: 0;
  padding-top: 1px;
}

.chain-text {
  font-size: 13px;
  margin: 0;
  white-space: pre-wrap;
}

.chain-pre {
  font-size: 12px;
  background: var(--color-bg);
  border: 1px solid var(--color-border-light);
  border-radius: var(--radius-sm);
  padding: 6px 10px;
  margin: 0;
  overflow-x: auto;
  white-space: pre-wrap;
  max-height: 160px;
  overflow-y: auto;
  font-family: var(--font-mono);
  flex: 1;
}

.chain-impact {
  padding: 8px 12px;
  display: flex;
  align-items: center;
  gap: 8px;
}
.chain-impact.impact-bar-danger {
  background: rgba(232, 132, 122, 0.1);
}
.chain-impact.impact-bar-warn {
  background: rgba(217, 168, 63, 0.1);
}
.chain-impact.impact-bar-ok {
  background: rgba(95, 206, 155, 0.1);
}
.chain-impact-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-muted);
}
</style>
