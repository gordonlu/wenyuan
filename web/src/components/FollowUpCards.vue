<template>
  <section class="panel">
    <div class="row-head">
      <h2>续议建议</h2>
      <div class="actions">
        <button
          v-if="!loading && suggestions.length > 0"
          class="icon"
          title="重新生成续议建议"
          @click="$emit('regenerate')"
        >
          <RotateCw :size="15" />
        </button>
      </div>
    </div>

    <div v-if="loading" class="muted" style="padding: 12px 0">加载中…</div>
    <div v-else-if="suggestions.length === 0" class="muted" style="padding: 12px 0">
      暂无续议建议。完成合议后系统自动生成，也可手动生成。
    </div>

    <div v-else class="item-grid">
      <article
        v-for="s in suggestions"
        :key="s.id"
        class="item followup-card"
      >
        <div class="item-head">
          <span :class="['badge', kindBadge(s.kind)]">{{ followUpKindLabels[s.kind] }}</span>
          <span class="badge flat">{{ followUpModeLabels[s.suggested_mode] }}</span>
          <span class="badge flat">{{ followUpSeatLabel(s.kind) }}</span>
        </div>
        <h3>{{ s.title }}</h3>
        <p>{{ s.message }}</p>
        <div class="item-actions">
          <button
            v-if="s.suggested_mode === 'single_seat'"
            class="small-btn"
            @click="$emit('start', { suggestion: s, mode: 'single_seat' })"
          >
            {{ s.action_label }}
          </button>
          <button
            v-if="s.suggested_mode === 'mini_deliberation'"
            class="small-btn"
            @click="$emit('start', { suggestion: s, mode: 'mini_deliberation' })"
          >
            小合议
          </button>
          <button
            v-if="s.suggested_mode === 're_deliberation'"
            class="small-btn"
            @click="$emit('start', { suggestion: s, mode: 're_deliberation' })"
          >
            新事实复议
          </button>
        </div>
      </article>
    </div>
  </section>
</template>

<script setup lang="ts">
import { RotateCw } from '@lucide/vue'
import { followUpKindLabels, followUpModeLabels, followUpSeatLabel, type FollowUpSuggestion } from '../domain/session'

defineProps<{
  suggestions: FollowUpSuggestion[]
  loading: boolean
}>()

defineEmits<{
  regenerate: []
  start: [payload: { suggestion: FollowUpSuggestion; mode: string }]
}>()

function kindBadge(kind: string) {
  if (kind === 'mitigate_risk' || kind === 'discuss_minority_concern') return 'warn'
  if (kind === 'expand_opportunity') return 'ok'
  return ''
}
</script>

<style scoped>
.followup-card .item-actions { display: flex; gap: 8px; margin-top: 10px; }
.small-btn { padding: 3px 10px; font-size: 12px; border: 1px solid var(--color-border-light); border-radius: var(--radius-sm); background: var(--color-bg); cursor: pointer; }
.small-btn:hover { background: var(--color-accent-light); }
</style>
