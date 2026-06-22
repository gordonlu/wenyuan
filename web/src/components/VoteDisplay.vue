<template>
  <section class="panel" v-if="votes.length">
    <h2>投票记录</h2>
    <div class="vote-grid">
      <div v-for="vote in votes" :key="`${vote.voter}-${vote.proposal_id}`" class="vote-item">
        <div class="vote-seat">{{ seatLabel(vote.voter) }}</div>
        <div class="vote-choice">
          <span :class="['badge', vote.final_choice ? 'ok' : 'warn']">
            {{ vote.final_choice ? '支持' : '反对' }}
          </span>
        </div>
        <div class="vote-target">
          <span class="muted">→</span>
          {{ proposalTitle(vote.proposal_id) || '未知策案' }}
        </div>
        <div class="vote-reason">{{ vote.reason }}</div>
        <div v-if="vote.key_evidence" class="vote-evidence muted">依据：{{ vote.key_evidence }}</div>
        <div v-if="vote.blocking_issue" class="vote-blocking"><span class="badge warn">阻塞：{{ vote.blocking_issue }}</span></div>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { seatLabels, type SeatKind } from '../domain/session'

const props = defineProps<{
  votes: Array<{
    voter: string
    proposal_id: string
    final_choice: boolean
    reason: string
    key_evidence?: string
    blocking_issue?: string
  }>
  proposals: Array<{ id: string; title: string }>
}>()

function seatLabel(key: string) {
  return seatLabels[key as SeatKind] || key
}

function proposalTitle(id: string) {
  return props.proposals.find((p) => p.id === id)?.title
}
</script>

<style scoped>
.vote-grid {
  display: grid;
  gap: var(--space-sm);
}
.vote-item {
  display: grid;
  grid-template-columns: 60px 50px 1fr;
  gap: var(--space-sm);
  align-items: start;
  padding: var(--space-md);
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
}
.vote-seat {
  font-weight: 600;
  font-size: 13px;
  font-family: var(--font-display);
  align-self: center;
}
.vote-choice {
  justify-self: center;
  margin-top: -5px;
}
.vote-target {
  font-size: 13px;
  line-height: 1.4;
  align-self: center;
}
.vote-reason {
  grid-column: 1 / -1;
  font-size: 13px;
  color: var(--color-text-muted);
  line-height: 1.5;
  padding-top: 4px;
  border-top: 1px solid var(--color-border-light);
}
.vote-evidence {
  grid-column: 1 / -1;
  font-size: 12px;
}
.vote-blocking {
  grid-column: 1 / -1;
  justify-self: start;
}
</style>
