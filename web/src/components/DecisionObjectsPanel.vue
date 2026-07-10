<template>
  <section class="panel">
    <div class="row-head">
      <h2>决策对象（{{ objects.length }}）</h2>
      <span v-if="openCount > 0" class="badge warn">{{ openCount }} 待处理</span>
    </div>
    <div v-if="objects.length === 0" class="muted" style="padding: 12px 0">暂无决策对象。</div>

    <div v-for="group in groupedObjects" :key="group.kind" class="obj-group">
      <div class="obj-group-head">
        <span :class="['badge', kindBadge(group.kind)]">{{ group.label }}</span>
        <span class="badge flat">{{ group.items.length }}</span>
      </div>
      <div class="obj-group-grid">
        <article
          v-for="obj in group.items"
          :key="obj.id"
          :class="['obj-card', `status-${obj.status}`]"
        >
          <div :class="['obj-status-line', `line-${obj.status}`]" />
          <div class="obj-card-body">
            <div class="obj-card-head">
              <h3>{{ obj.title }}</h3>
              <div class="obj-badges">
                <span :class="['badge', priorityBadge(obj.priority)]">{{ decisionObjectPriorityLabels[obj.priority] }}</span>
                <span :class="['badge', statusBadge(obj.status)]">{{ decisionObjectStatusLabels[obj.status] }}</span>
              </div>
            </div>
            <p v-if="obj.summary" class="obj-summary">{{ obj.summary }}</p>
            <div class="obj-meta">
              <span v-if="obj.seat" :class="['seat-tag', obj.seat]">{{ seatLabels[obj.seat as SeatKind] || obj.seat }}</span>
              <span v-if="obj.source_phase" class="muted">{{ phaseLabels[obj.source_phase as SessionPhase] || obj.source_phase }}</span>
            </div>
            <div v-if="obj.status === 'open'" class="obj-actions">
              <button class="small-btn" @click="$emit('resolve', obj.id)">已解决</button>
              <button class="small-btn muted" @click="$emit('dismiss', obj.id)">忽略</button>
            </div>
          </div>
        </article>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import {
  decisionObjectKindLabels, decisionObjectStatusLabels, decisionObjectPriorityLabels,
  seatLabels, phaseLabels, type DecisionObject, type SeatKind, type SessionPhase,
} from '../domain/session'

const props = defineProps<{ objects: DecisionObject[] }>()
defineEmits<{ resolve: [id: string]; dismiss: [id: string] }>()

const openCount = computed(() => props.objects.filter((o) => o.status === 'open').length)

const groupedObjects = computed(() => {
  const kindOrder = ['hypothesis', 'risk', 'action', 'opportunity', 'minority_concern']
  const groups: Array<{ kind: string; label: string; items: DecisionObject[] }> = []
  const priorityOrder: Record<string, number> = { critical: 0, high: 1, medium: 2, low: 3 }
  for (const kind of kindOrder) {
    const items = props.objects
      .filter((o) => o.kind === kind)
      .sort((a, b) => (priorityOrder[a.priority] ?? 2) - (priorityOrder[b.priority] ?? 2))
    if (items.length) {
      groups.push({ kind, label: (decisionObjectKindLabels as Record<string, string>)[kind] ?? kind, items })
    }
  }
  return groups
})

function kindBadge(kind: string) {
  if (kind === 'risk' || kind === 'minority_concern') return 'warn'
  if (kind === 'opportunity') return 'ok'
  return ''
}

function priorityBadge(priority: string) {
  if (priority === 'critical') return 'warn'
  if (priority === 'high') return ''
  return 'flat'
}

function statusBadge(status: string) {
  if (status === 'open') return 'warn'
  if (status === 'resolved' || status === 'expanded') return 'ok'
  return 'flat'
}
</script>

<style scoped>
.obj-group {
  margin-bottom: 20px;
}
.obj-group-head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
}
.obj-group-grid {
  display: grid;
  gap: 8px;
}
.obj-card {
  display: flex;
  gap: 0;
  background: var(--color-surface);
  border: 1px solid var(--color-border-light);
  border-radius: var(--radius-md);
  overflow: hidden;
}
.obj-status-line {
  width: 4px;
  flex-shrink: 0;
  background: var(--color-border-light);
}
.line-open { background: var(--color-warning-text); }
.line-resolved, .line-expanded { background: var(--color-accent); }
.line-dismissed, .line-superseded { background: var(--color-text-dim); }

.obj-card-body {
  padding: 10px 12px;
  flex: 1;
  min-width: 0;
}
.status-dismissed .obj-card-body,
.status-superseded .obj-card-body {
  opacity: 0.5;
}

.obj-card-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 4px;
}
.obj-card-head h3 {
  margin: 0;
  font-size: 14px;
  font-weight: 700;
}
.obj-badges {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
}
.obj-summary {
  font-size: 13px;
  color: var(--color-text-muted);
  margin: 4px 0;
  line-height: 1.4;
}
.obj-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 6px;
  font-size: 12px;
}
.obj-actions {
  display: flex;
  gap: 6px;
  margin-top: 8px;
}
.small-btn {
  padding: 3px 10px;
  font-size: 12px;
  border: 1px solid var(--color-border-light);
  border-radius: var(--radius-sm);
  background: var(--color-bg);
  cursor: pointer;
}
.small-btn:hover { background: var(--color-accent-light); }
.small-btn.muted { color: var(--color-text-dim); }

@media (min-width: 640px) {
  .obj-group-grid {
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  }
}
</style>
