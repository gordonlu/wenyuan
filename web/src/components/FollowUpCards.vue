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

    <div v-else class="fu-grid">
      <article
        v-for="s in suggestions"
        :key="s.id"
        :class="['fu-card', `fu-${s.kind}`]"
      >
        <div class="fu-card-head">
          <span :class="['badge', kindBadge(s.kind)]">{{ followUpKindLabels[s.kind] }}</span>
          <span class="badge flat">{{ followUpModeLabels[s.suggested_mode] }}</span>
        </div>
        <h3>{{ s.title }}</h3>
        <p class="fu-why">{{ s.message }}</p>
        <div class="fu-meta">
          <span class="fu-seat">{{ followUpSeatLabel(s.kind) }}</span>
        </div>
        <div class="fu-action">
          <span class="fu-effect-label">将执行</span>
          <button
            v-if="s.suggested_mode === 'single_seat'"
            class="fu-btn"
            @click="$emit('start', { suggestion: s, mode: 'single_seat' })"
          >
            {{ s.action_label }}
          </button>
          <button
            v-else-if="s.suggested_mode === 'mini_deliberation'"
            class="fu-btn"
            @click="$emit('start', { suggestion: s, mode: 'mini_deliberation' })"
          >
            小合议
          </button>
          <button
            v-else-if="s.suggested_mode === 're_deliberation'"
            class="fu-btn fu-btn-warn"
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

defineProps<{ suggestions: FollowUpSuggestion[]; loading: boolean }>()
defineEmits<{ regenerate: []; start: [payload: { suggestion: FollowUpSuggestion; mode: string }] }>()

function kindBadge(kind: string) {
  if (kind === 'mitigate_risk' || kind === 'discuss_minority_concern') return 'warn'
  if (kind === 'expand_opportunity') return 'ok'
  return ''
}
</script>

<style scoped>
.fu-grid {
  display: grid;
  gap: 10px;
}
.fu-card {
  border: 1px solid var(--color-border-light);
  border-radius: var(--radius-md);
  background: var(--color-surface);
  padding: 12px 14px;
}
.fu-card.fu-mitigate_risk { border-left: 3px solid var(--color-warning-text); }
.fu-card.fu-discuss_minority_concern { border-left: 3px solid var(--color-warning-text); }
.fu-card.fu-expand_opportunity { border-left: 3px solid var(--color-accent); }

.fu-card-head {
  display: flex;
  gap: 6px;
  margin-bottom: 6px;
}
.fu-card h3 {
  margin: 0 0 4px;
  font-size: 14px;
}
.fu-why {
  font-size: 13px;
  color: var(--color-text-muted);
  margin: 0 0 10px;
  line-height: 1.5;
}
.fu-meta {
  margin-bottom: 8px;
}
.fu-seat {
  font-size: 12px;
  color: var(--color-text-dim);
  font-weight: 600;
}
.fu-action {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-top: 8px;
  border-top: 1px solid var(--color-border-light);
}
.fu-effect-label {
  font-size: 12px;
  color: var(--color-text-dim);
}
.fu-btn {
  padding: 5px 14px;
  font-size: 12px;
  font-weight: 600;
  border: 1px solid var(--color-border-light);
  border-radius: var(--radius-sm);
  background: var(--color-bg);
  cursor: pointer;
  transition: background 0.15s;
}
.fu-btn:hover { background: var(--color-accent-light); }
.fu-btn-warn { color: var(--color-warning-text); border-color: var(--color-warning-border); }
.fu-btn-warn:hover { background: var(--color-warning-bg); }
</style>
