<template>
  <section class="panel">
    <div class="row-head">
      <h2>决策对象（{{ objects.length }}）</h2>
      <span class="badge flat" v-if="openCount > 0">{{ openCount }} 待处理</span>
    </div>
    <div v-if="objects.length === 0" class="muted" style="padding: 12px 0">暂无决策对象。</div>
    <div class="item-grid">
      <article
        v-for="obj in sortedObjects"
        :key="obj.id"
        :class="['item', 'decision-object-card', `status-${obj.status}`, `priority-${obj.priority}`]"
      >
        <div class="item-head">
          <span :class="['badge', kindBadge(obj.kind)]">{{ decisionObjectKindLabels[obj.kind] }}</span>
          <span :class="['badge', statusBadge(obj.status)]">{{ decisionObjectStatusLabels[obj.status] }}</span>
          <span class="badge flat">{{ decisionObjectPriorityLabels[obj.priority] }}</span>
        </div>
        <h3>{{ obj.title }}</h3>
        <p>{{ obj.summary }}</p>
        <div class="item-meta">
          <span v-if="obj.seat" :class="['seat-tag', obj.seat]">{{ seatLabels[obj.seat as SeatKind] || obj.seat }}</span>
          <span v-if="obj.source_phase" class="muted">{{ phaseLabels[obj.source_phase as SessionPhase] || obj.source_phase }}</span>
        </div>
        <div class="item-actions" v-if="obj.status === 'open'">
          <button class="small-btn" @click="$emit('resolve', obj.id)">标记已解决</button>
          <button class="small-btn muted" @click="$emit('dismiss', obj.id)">忽略</button>
        </div>
      </article>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { decisionObjectKindLabels, decisionObjectStatusLabels, decisionObjectPriorityLabels, seatLabels, phaseLabels, type DecisionObject, type SeatKind, type SessionPhase } from '../domain/session'

const props = defineProps<{
  objects: DecisionObject[]
}>()

defineEmits<{
  resolve: [id: string]
  dismiss: [id: string]
}>()

const openCount = computed(() => props.objects.filter((o) => o.status === 'open').length)

const sortedObjects = computed(() =>
  [...props.objects].sort((a, b) => {
    const order: Record<string, number> = { critical: 0, high: 1, medium: 2, low: 3 }
    return (order[a.priority] ?? 2) - (order[b.priority] ?? 2)
  })
)

function kindBadge(kind: string) {
  if (kind === 'risk' || kind === 'minority_concern') return 'warn'
  if (kind === 'opportunity') return 'ok'
  return ''
}

function statusBadge(status: string) {
  if (status === 'open') return 'warn'
  if (status === 'resolved' || status === 'expanded') return 'ok'
  if (status === 'dismissed' || status === 'superseded') return 'flat'
  return ''
}
</script>

<style scoped>
.decision-object-card.status-resolved { opacity: 0.7; }
.decision-object-card.status-dismissed { opacity: 0.5; }
.decision-object-card.status-superseded { opacity: 0.5; text-decoration: line-through; }
.item-meta { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 8px; font-size: 12px; }
.item-actions { display: flex; gap: 8px; margin-top: 10px; }
.small-btn { padding: 3px 10px; font-size: 12px; border: 1px solid var(--color-border-light); border-radius: var(--radius-sm); background: var(--color-bg); cursor: pointer; }
.small-btn:hover { background: var(--color-accent-light); }
.small-btn.muted { color: var(--color-text-dim); }
</style>
