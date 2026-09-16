<template>
  <section class="panel" v-if="proposals.length">
    <h2>策案对比</h2>

    <!-- Summary rows -->
    <div class="compare-summary-grid" :style="{ '--col-count': proposals.length }">
      <div class="compare-header">席位</div>
      <div v-for="p in proposals" :key="p.id" class="compare-header seat-col" :class="{ selected: p.id === selectedId }">
        {{ seatLabel(p.proposed_by) }}
      </div>

      <div class="compare-label">策案</div>
      <div v-for="p in proposals" :key="p.id" class="compare-cell" :class="{ selected: p.id === selectedId }">
        <strong>{{ p.title }}</strong>
      </div>

      <div class="compare-label">置信度</div>
      <div v-for="p in proposals" :key="p.id" class="compare-cell" :class="{ selected: p.id === selectedId }">
        <span v-if="p.confidence !== undefined" class="badge"
          :class="p.confidence >= 0.7 ? 'ok' : p.confidence >= 0.5 ? '' : 'warn'"
        >{{ Math.round(p.confidence * 100) }}%</span>
      </div>

      <div class="compare-label">摘要</div>
      <div v-for="p in proposals" :key="p.id" class="compare-cell formatted" :class="{ selected: p.id === selectedId }"
        v-html="renderMarkdown(p.summary)"
      />

      <div class="compare-label">主要风险</div>
      <div v-for="p in proposals" :key="p.id" class="compare-cell" :class="{ selected: p.id === selectedId }">
        <span v-if="!p.risks?.length" class="muted">无</span>
        <span v-else>{{ p.risks.slice(0, 3).join('、') }}</span>
      </div>

      <div class="compare-label">选择状态</div>
      <div v-for="p in proposals" :key="p.id" class="compare-cell" :class="{ selected: p.id === selectedId }">
        <span v-if="p.id === selectedId" class="badge ok">最终方案</span>
        <span v-else class="muted">未采纳</span>
      </div>
    </div>

    <!-- Dimension toggle -->
    <div class="detail-toggle">
      <button
        v-for="dim in dimensions"
        :key="dim.key"
        type="button"
        :class="['tab-toggle', { active: activeDimension === dim.key }]"
        @click="activeDimension = dim.key === activeDimension ? null : dim.key"
      >
        {{ dim.label }}
      </button>
    </div>

    <!-- Dimension detail -->
    <div v-if="activeDimension" class="compare-dim-grid" :style="{ '--col-count': proposals.length }">
      <div class="compare-header">席位</div>
      <div v-for="p in proposals" :key="p.id" class="compare-header seat-col"
        :class="{ selected: p.id === selectedId }">
        {{ seatLabel(p.proposed_by) }}
      </div>

      <div class="compare-label">{{ dimensionLabel }}</div>
      <div
        v-for="p in proposals"
        :key="p.id"
        class="compare-cell formatted"
        :class="{ selected: p.id === selectedId }"
        v-html="dimensionContent(p)"
      />
    </div>
  </section>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { seatLabels, type SeatKind } from '../domain/session'
import { renderMarkdown } from '../utils/markdown'

const props = defineProps<{
  proposals: Array<{
    id: string
    proposed_by: string
    title: string
    summary: string
    implementation_path: string
    adopted_points?: string[]
    rejected_points?: string[]
    risks?: string[]
    success_metrics?: string[]
    confidence?: number
  }>
  selectedId?: string
}>()

function seatLabel(key: string) {
  return seatLabels[key as SeatKind] || key
}

const activeDimension = ref<string | null>(null)

const dimensions = [
  { key: 'implementation', label: '落地路径' },
  { key: 'adopted', label: '采纳与拒绝' },
  { key: 'metrics', label: '成功指标' },
]

const dimensionLabel = computed(() => {
  const d = dimensions.find((d) => d.key === activeDimension.value)
  return d?.label ?? ''
})

import { computed } from 'vue'

function dimensionContent(p: typeof props.proposals[0]) {
  if (activeDimension.value === 'implementation') {
    return renderMarkdown(p.implementation_path) || '<span class="muted">无</span>'
  }
  if (activeDimension.value === 'adopted') {
    let html = ''
    if (p.adopted_points?.length) {
      html += `<p><strong>采纳：</strong>${p.adopted_points.join('、')}</p>`
    }
    if (p.rejected_points?.length) {
      html += `<p><strong>拒绝：</strong>${p.rejected_points.join('、')}</p>`
    }
    return html || '<span class="muted">无</span>'
  }
  if (activeDimension.value === 'metrics') {
    if (p.success_metrics?.length) return p.success_metrics.join('、')
    return '<span class="muted">无</span>'
  }
  return ''
}
</script>

<style scoped>
.compare-summary-grid {
  display: grid;
  grid-template-columns: 80px repeat(var(--col-count), 1fr);
  gap: 1px;
  background: var(--color-border-light);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  overflow: hidden;
  margin-bottom: 12px;
}
.compare-dim-grid {
  display: grid;
  grid-template-columns: 80px repeat(var(--col-count), 1fr);
  gap: 1px;
  background: var(--color-border-light);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  overflow: hidden;
}
.compare-header {
  background: var(--color-surface-alt);
  padding: 10px 12px;
  font-weight: 600;
  font-size: 13px;
  font-family: var(--font-display);
}
.compare-header.seat-col {
  text-align: center;
}
.compare-header.selected {
  background: var(--color-accent-light);
  color: var(--color-accent-text);
}
.compare-label {
  background: var(--color-surface-alt);
  padding: 8px 10px;
  font-size: 11px;
  color: var(--color-text-muted);
  font-weight: 700;
  white-space: nowrap;
}
.compare-cell {
  background: var(--color-surface);
  padding: 8px 10px;
  font-size: 13px;
  line-height: 1.5;
  color: var(--color-text);
}
.compare-cell.selected {
  background: rgba(95, 206, 155, 0.1);
}
.compare-cell.formatted :deep(p) {
  margin: 0.3em 0;
}
.compare-cell.formatted :deep(strong) {
  font-weight: 600;
}
.compare-cell.formatted :deep(ul),
.compare-cell.formatted :deep(ol) {
  padding-left: 1.3em;
  margin: 0.2em 0;
}
.detail-toggle {
  display: flex;
  gap: 4px;
  margin-bottom: 12px;
  flex-wrap: wrap;
}
.tab-toggle {
  padding: 5px 14px;
  border: 1px solid var(--color-border-light);
  border-radius: 14px;
  background: var(--color-bg-subtle);
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-muted);
  cursor: pointer;
  transition: all 0.15s ease;
}
.tab-toggle:hover {
  background: var(--color-bg);
  color: var(--color-text);
}
.tab-toggle.active {
  background: var(--color-accent);
  color: #04222a;
  border-color: var(--color-accent);
}
</style>
