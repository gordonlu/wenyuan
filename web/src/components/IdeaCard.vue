<template>
  <article class="item idea-card" :class="{ unconventional: idea.unconventional }">
    <div class="item-head">
      <span>{{ seatLabel(idea.proposed_by) }}</span>
      <span v-if="idea.status" :class="['badge', `idea-status ${idea.status}`]">
        {{ statusLabel(idea.status) }}
      </span>
    </div>
    <h3>{{ idea.title }}</h3>
    <p>{{ idea.summary }}</p>
    <p v-if="idea.value" class="muted">价值：{{ idea.value }}</p>
    <p v-if="idea.mechanism" class="muted">机制：{{ idea.mechanism }}</p>
    <div v-if="idea.source_seats?.length && idea.source_seats.length > 1" class="idea-tags">
      <span class="badge ok">合并来源：{{ idea.source_seats.map((s) => seatLabel(s)).join('、') }}</span>
    </div>
    <div v-if="idea.unconventional" class="idea-tags">
      <span class="badge warn">非主流方向</span>
    </div>
    <div v-if="idea.assumptions?.length" class="idea-tags">
      <span v-for="a in idea.assumptions" :key="a" class="badge">假设：{{ a }}</span>
    </div>
    <div v-if="idea.risks?.length" class="idea-tags">
      <span v-for="r in idea.risks" :key="r" class="badge warn">风险：{{ r }}</span>
    </div>
    <div v-if="idea.referenced_by_proposals?.length" class="idea-tags">
      <span class="badge ok">被 {{ idea.referenced_by_proposals.length }} 个策案引用</span>
    </div>
    <div v-if="idea.challenged_by?.length" class="idea-tags">
      <span class="badge warn">被 {{ idea.challenged_by.length }} 条批议质疑</span>
    </div>
    <div v-if="idea.merged_into" class="idea-tags">
      <span class="badge">已合并</span>
    </div>
  </article>
</template>

<script setup lang="ts">
import { seatLabels, ideaStatusLabels, type SeatKind } from '../domain/session'

defineProps<{
  idea: {
    proposed_by: string
    status?: string
    title: string
    summary: string
    value?: string
    mechanism?: string
    source_seats?: string[]
    unconventional?: boolean
    assumptions?: string[]
    risks?: string[]
    referenced_by_proposals?: string[]
    challenged_by?: string[]
    merged_into?: string | null
  }
}>()

function seatLabel(key: string) {
  return seatLabels[key as SeatKind] || key
}

function statusLabel(key?: string) {
  return key ? (ideaStatusLabels[key] || key) : ''
}
</script>

<style scoped>
.idea-card.unconventional {
  border-left: 6px solid var(--color-warning-border);
}
.idea-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 10px;
}
.idea-tags .badge {
  font-size: 11px;
  white-space: normal;
  max-width: 100%;
  overflow-wrap: break-word;
}
</style>
