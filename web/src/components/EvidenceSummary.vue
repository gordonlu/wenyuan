<template>
  <section class="evidence-summary" :class="{ 'has-warnings': summary.has_safety_warnings }">
    <header class="evidence-summary-head">
      <h3>来源证据</h3>
      <span v-if="summary.total" class="badge ok">{{ summary.total }} 项</span>
      <span v-else class="badge">无</span>
    </header>

    <div v-if="summary.total" class="evidence-summary-body">
      <div class="evidence-grid">
        <div class="evidence-stat">
          <span class="evidence-stat-value">{{ bySourceText }}</span>
          <span class="evidence-stat-label">来源分布</span>
        </div>
        <div class="evidence-stat">
          <span class="evidence-stat-value" :class="{ 'text-warn': summary.untrusted_count > 0 }">
            {{ summary.untrusted_count }}
          </span>
          <span class="evidence-stat-label">不可信外部</span>
        </div>
        <div class="evidence-stat">
          <span class="evidence-stat-value" :class="{ 'text-warn': summary.unverified_claims > 0 }">
            {{ summary.unverified_claims }}
          </span>
          <span class="evidence-stat-label">未验证主张</span>
        </div>
        <div class="evidence-stat">
          <span v-if="summary.injection_risk_count > 0" class="evidence-stat-value text-danger">
            {{ summary.injection_risk_count }}
          </span>
          <span v-else class="evidence-stat-value">0</span>
          <span class="evidence-stat-label">注入风险</span>
        </div>
      </div>

      <div v-if="summary.has_safety_warnings" class="evidence-summary-note">
        部分外部来源存在安全标记，详细内容请在证据池查看
      </div>
    </div>

    <div v-else class="evidence-summary-empty">
      <p>暂无来源证据。启用联网搜索或上传文件可在讨论中获得外部事实支撑。</p>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { evidenceSourceKindLabels, type EvidenceSummary } from '../domain/session'

const props = defineProps<{
  summary: EvidenceSummary
}>()

const sourceKindOrder = ['web_search', 'file', 'code', 'log', 'data', 'internal']

const bySourceText = computed(() => {
  const parts: string[] = []
  for (const kind of sourceKindOrder) {
    const count = props.summary.by_source[kind]
    if (count) {
      parts.push(`${evidenceSourceKindLabels[kind] ?? kind} ${count}`)
    }
  }
  return parts.join(' · ') || '仅内部'
})
</script>

<style scoped>
.evidence-summary {
  padding: 16px;
  border: 1px solid var(--color-border-light);
  border-radius: var(--radius-md);
  background: var(--color-surface);
}

.evidence-summary.has-warnings {
  border-color: var(--color-warning-border);
  background: var(--color-warning-bg);
}

.evidence-summary-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.evidence-summary-head h3 {
  margin: 0;
  font-size: 14px;
}

.evidence-summary-body {
  display: grid;
  gap: 12px;
}

.evidence-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 1px;
  overflow: hidden;
  border: 1px solid var(--color-border-light);
  border-radius: var(--radius-sm);
  background: var(--color-border-light);
}

.evidence-stat {
  padding: 10px 12px;
  background: var(--color-surface);
  text-align: center;
}

.evidence-stat-value {
  display: block;
  font-size: 20px;
  font-weight: 800;
  line-height: 1.2;
  color: var(--color-text);
}

.evidence-stat-value.text-warn {
  color: var(--color-warning-text);
}

.evidence-stat-value.text-danger {
  color: var(--color-danger);
}

.evidence-stat-label {
  display: block;
  margin-top: 2px;
  font-size: 11px;
  color: var(--color-text-muted);
}

.evidence-summary-note {
  padding: 8px 12px;
  font-size: 12px;
  color: var(--color-warning-text);
  background: rgba(212, 184, 106, 0.15);
  border-radius: var(--radius-sm);
}

.evidence-summary-empty {
  padding: 12px 0;
}

.evidence-summary-empty p {
  margin: 0;
  font-size: 13px;
  color: var(--color-text-muted);
  line-height: 1.5;
}
</style>
