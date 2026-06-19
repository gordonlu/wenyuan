<template>
  <section class="panel" v-if="proposals.length">
    <h2>策案对比</h2>
    <div class="compare-grid">
      <div class="compare-header">维度</div>
      <template v-for="proposal in proposals" :key="proposal.id">
        <div class="compare-header seat-col">
          <span>{{ seatLabel(proposal.proposed_by) }}</span>
          <span v-if="proposal.confidence !== undefined" class="badge">{{ Math.round(proposal.confidence * 100) }}%</span>
        </div>
      </template>

      <div class="compare-label">标题</div>
      <div v-for="p in proposals" :key="p.id" class="compare-cell">{{ p.title }}</div>

      <div class="compare-label">摘要</div>
      <div v-for="p in proposals" :key="p.id" class="compare-cell">{{ p.summary }}</div>

      <div class="compare-label">落地路径</div>
      <div v-for="p in proposals" :key="p.id" class="compare-cell muted">{{ p.implementation_path }}</div>

      <div class="compare-label">采纳观点</div>
      <div v-for="p in proposals" :key="p.id" class="compare-cell">
        <span v-if="!p.adopted_points?.length" class="muted">无</span>
        <span v-else>{{ p.adopted_points.join('、') }}</span>
      </div>

      <div class="compare-label">拒绝观点</div>
      <div v-for="p in proposals" :key="p.id" class="compare-cell">
        <span v-if="!p.rejected_points?.length" class="muted">无</span>
        <span v-else>{{ p.rejected_points.join('、') }}</span>
      </div>

      <div class="compare-label">风险</div>
      <div v-for="p in proposals" :key="p.id" class="compare-cell">
        <span v-if="!p.risks?.length" class="muted">无</span>
        <span v-else>{{ p.risks.join('、') }}</span>
      </div>

      <div class="compare-label">成功指标</div>
      <div v-for="p in proposals" :key="p.id" class="compare-cell">
        <span v-if="!p.success_metrics?.length" class="muted">无</span>
        <span v-else>{{ p.success_metrics.join('、') }}</span>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { seatLabels, type SeatKind } from '../domain/session'

defineProps<{
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
}>()

function seatLabel(key: string) {
  return seatLabels[key as SeatKind] || key
}
</script>

<style scoped>
.compare-grid {
  display: grid;
  grid-template-columns: 100px repeat(auto-fit, minmax(200px, 1fr));
  gap: 1px;
  background: #d8dfd9;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  overflow: hidden;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.62);
}
.compare-header {
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.88), rgba(238, 243, 239, 0.88)),
    #eef3ef;
  padding: 12px;
  font-weight: 600;
  font-size: 13px;
  font-family: var(--font-display);
  display: flex;
  align-items: center;
  gap: 8px;
}
.compare-header.seat-col {
  justify-content: center;
}
.compare-label {
  background: #f3f6f2;
  padding: 10px 12px;
  font-size: 12px;
  color: var(--color-text-muted);
  font-weight: 700;
  white-space: nowrap;
}
.compare-cell {
  background: rgba(255, 255, 255, 0.72);
  padding: 10px 12px;
  font-size: 13px;
  line-height: 1.5;
  color: var(--color-text);
}

.compare-cell:nth-child(odd) {
  background: rgba(250, 252, 249, 0.78);
}
</style>
