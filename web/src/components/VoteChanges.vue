<template>
  <section v-if="hasTwoRounds" class="panel vote-changes">
    <h2>投票变化（合案前后对比）</h2>
    <div class="rounds">
      <div class="round">
        <h3>第一轮（原始策案）</h3>
        <div v-for="(round, ri) in roundVotes" :key="ri" class="vote-block">
          <p v-for="v in round.first" :key="v.voter" class="vote-row">
            <span :class="['seat-tag', v.voter]">{{ seatLabels[v.voter] }}</span>
            <span :class="['badge', v.final_choice ? 'ok' : 'warn']">{{ v.final_choice ? '支持' : '反对' }}</span>
            <span class="sep">{{ proposalTitle(v.proposal_id) }}</span>
          </p>
        </div>
      </div>
      <div class="round" v-if="roundVotes.length">
        <h3>第二轮（含合案）</h3>
        <div v-for="(round, ri) in roundVotes" :key="ri" class="vote-block">
          <p v-for="v in round.second" :key="v.voter" class="vote-row">
            <span :class="['seat-tag', v.voter]">{{ seatLabels[v.voter] }}</span>
            <span :class="['badge', v.final_choice ? 'ok' : 'warn']">{{ v.final_choice ? '支持' : '反对' }}</span>
            <span class="sep">{{ proposalTitle(v.proposal_id) }}</span>
          </p>
        </div>
      </div>
    </div>
  </section>
  <section v-else-if="votes.length" class="panel vote-changes">
    <h2>投票记录</h2>
    <p class="muted">未经历合案复议，仅一轮投票。</p>
  </section>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { seatLabels, type Proposal, type Vote } from '../domain/session'

const props = defineProps<{
  votes: Vote[]
  proposals: Proposal[]
}>()

const totalVotes = props.votes.length
const half = Math.floor(totalVotes / 2)

const roundVotes = computed(() => {
  // If total votes > number of seats (3), we likely have two rounds
  if (totalVotes <= 3) return []
  const first = props.votes.slice(0, half)
  const second = props.votes.slice(half)
  return [{ first, second }]
})

const hasTwoRounds = computed(() => roundVotes.value.length > 0 && roundVotes.value[0].second.length > 0)

function proposalTitle(id: string) {
  return props.proposals.find((p) => p.id === id)?.title ?? '未知策案'
}
</script>

<style scoped>
.rounds { display: flex; gap: 24px; flex-wrap: wrap; }
.round { flex: 1; min-width: 260px; background: var(--color-bg); border: 1px solid var(--color-border); border-radius: var(--radius); padding: 16px; }
.round h3 { font-size: 14px; margin-bottom: 12px; color: var(--color-text-muted); }
.vote-block { display: flex; flex-direction: column; gap: 8px; }
.vote-row { display: flex; align-items: center; gap: 8px; font-size: 13px; }
.tag { background: var(--color-bg-dim); border: 1px solid var(--color-border); border-radius: 4px; padding: 1px 8px; font-size: 12px; white-space: nowrap; }
.sep { color: var(--color-text); }
.seat-tag {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 5px;
  font-size: 12px;
  font-weight: 600;
}
.seat-tag.mouyuan { background: #e2eef9; color: #1a5a8c; }
.seat-tag.jingshi { background: #f0e6d3; color: #7a5a2e; }
.seat-tag.chizheng { background: #f5e8e8; color: #8c3a3a; }
</style>
