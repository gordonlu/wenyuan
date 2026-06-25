<template>
  <section class="panel">
    <div class="row-head">
      <h2>续议时间线</h2>
      <span class="badge flat" v-if="turns.length">{{ turns.length }} 轮</span>
    </div>

    <div v-if="turns.length === 0" class="muted" style="padding: 12px 0">暂无续议记录。</div>

    <div v-else class="followup-timeline">
      <div v-for="turn in sortedTurns" :key="turn.id" class="turn-entry">
        <div class="turn-head">
          <span class="badge">{{ followUpModeLabels[turn.mode] }}</span>
          <span :class="['badge', impactBadge(turn.impact)]">{{ followUpImpactLabels[turn.impact] }}</span>
          <time class="muted">{{ formatTime(turn.created_at) }}</time>
        </div>
        <div v-if="turn.user_input" class="turn-user-input">
          <span class="turn-label">用户输入：</span>
          <p>{{ turn.user_input }}</p>
        </div>
        <div class="turn-result">
          <span class="turn-label">执行结果：</span>
          <pre class="turn-json">{{ formatResult(turn.result_json) }}</pre>
        </div>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { followUpModeLabels, followUpImpactLabels, type FollowUpTurn } from '../domain/session'

const props = defineProps<{
  turns: FollowUpTurn[]
}>()

const sortedTurns = computed(() =>
  [...props.turns].sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
)

function impactBadge(impact: string) {
  if (impact === 'changes_decision') return 'danger'
  if (impact === 'raises_new_risk' || impact === 'suggests_re_deliberation') return 'warn'
  if (impact === 'no_change') return 'flat'
  return 'ok'
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
.followup-timeline { display: flex; flex-direction: column; gap: 12px; }
.turn-entry { padding: 12px; border: 1px solid var(--color-border-light); border-radius: var(--radius-sm); background: var(--color-bg-subtle); }
.turn-head { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; margin-bottom: 8px; }
.turn-head time { margin-left: auto; font-size: 12px; }
.turn-label { font-weight: 600; font-size: 12px; color: var(--color-text-dim); }
.turn-user-input p { margin: 4px 0 0; font-size: 13px; }
.turn-result { margin-top: 6px; }
.turn-json { margin: 4px 0 0; font-size: 12px; background: var(--color-bg); padding: 8px; border: 1px solid var(--color-border-light); border-radius: var(--radius-sm); overflow-x: auto; white-space: pre-wrap; max-height: 200px; overflow-y: auto; font-family: var(--font-mono); }
</style>
