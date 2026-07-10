<template>
  <section v-if="seatsWithVotes.length" class="panel">
    <h2>投票收敛</h2>

    <div class="vote-graph-wrap">
      <svg
        :viewBox="`0 0 ${vbW} ${vbH}`"
        class="vote-graph-svg"
        role="figure"
        aria-label="投票收敛图"
      >
        <!-- connections -->
        <line
          v-for="(line, i) in lines"
          :key="i"
          :x1="line.x1" :y1="line.y1" :x2="line.x2" :y2="line.y2"
          :stroke="line.color"
          :stroke-width="line.majority ? 2.5 : 1.5"
          stroke-linecap="round"
          :stroke-dasharray="line.blocked ? '6,3' : 'none'"
        />
        <!-- seat nodes -->
        <circle
          v-for="s in seatNodes"
          :key="s.seat"
          :cx="s.x" :cy="s.y"
          :r="18"
          :fill="s.bg"
          stroke="var(--color-border)"
          stroke-width="1"
        />
        <text
          v-for="s in seatNodes"
          :key="'t' + s.seat"
          :x="s.x" :y="s.y"
          text-anchor="middle"
          dominant-baseline="central"
          class="vg-seat-label"
          :fill="s.color"
        >{{ s.label }}</text>
        <!-- proposal nodes -->
        <rect
          v-for="p in propNodes"
          :key="p.id"
          :x="p.x - p.w / 2"
          :y="p.y - p.h / 2"
          :width="p.w"
          :height="p.h"
          :rx="6"
          :fill="p.majority ? '#e8f2ee' : 'var(--color-bg-subtle)'"
          :stroke="p.majority ? 'var(--color-accent)' : 'var(--color-border-light)'"
          :stroke-width="p.majority ? 2 : 1"
        />
        <text
          v-for="p in propNodes"
          :key="'t' + p.id"
          :x="p.x"
          :y="p.y"
          text-anchor="middle"
          dominant-baseline="central"
          class="vg-prop-label"
        >{{ p.label }}</text>
        <!-- blocking markers -->
        <g v-for="b in blockers" :key="'b' + b.seat">
          <circle
            :cx="b.x" :cy="b.y" r="6"
            fill="#d44" stroke="#fff" stroke-width="1.5"
          />
          <text
            :x="b.x" :y="b.y + 1"
            text-anchor="middle"
            dominant-baseline="central"
            font-size="9" font-weight="800" fill="#fff"
          >!</text>
          <text
            :x="b.x + 10" :y="b.y"
            font-size="9"
            fill="#a03030"
          >阻断</text>
        </g>
      </svg>

      <div class="vg-summary">
        <span v-if="majorityText" :class="['badge', majorityStatus]">{{ majorityText }}</span>
        <span v-if="hasBlockers" class="badge warn">有阻断问题</span>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { seatLabels, type SeatKind } from '../domain/session'

const props = defineProps<{
  votes: Array<{
    voter: SeatKind | string
    proposal_id: string
    final_choice?: boolean
    blocking_issue?: string
  }>
  proposals: Array<{
    id: string
    title: string
    proposed_by?: SeatKind | string
  }>
}>()

const seatColors: Record<string, string> = {
  mouyuan: '#1a6ba0',
  jingshi: '#9a6a30',
  chizheng: '#a03040',
}
const seatBgs: Record<string, string> = {
  mouyuan: '#e2eef9',
  jingshi: '#f0e6d3',
  chizheng: '#f5e8e8',
}

// ── Seat choices ──
const seatsWithVotes = computed(() => {
  const seatList: SeatKind[] = ['mouyuan', 'jingshi', 'chizheng']
  return seatList.filter((s) => props.votes.some((v) => v.voter === s))
    .map((s) => {
      const vote = props.votes.find((v) => v.voter === s && v.final_choice !== false)
        || props.votes.filter((v) => v.voter === s).pop()
      return {
        seat: s,
        proposalId: vote?.proposal_id ?? '',
        blocking: !!vote?.blocking_issue,
      }
    })
})

const voteCounts = computed(() => {
  const counts: Record<string, number> = {}
  for (const sw of seatsWithVotes.value) {
    if (!sw.proposalId) continue
    counts[sw.proposalId] = (counts[sw.proposalId] || 0) + 1
  }
  return counts
})

const majorityProposal = computed(() => {
  let max = 0
  let best = ''
  for (const [pid, count] of Object.entries(voteCounts.value)) {
    if (count > max) { max = count; best = pid }
  }
  return best
})

const majorityCount = computed(() => voteCounts.value[majorityProposal.value] || 0)
const totalVoters = computed(() => seatsWithVotes.value.length)

const majorityText = computed(() => {
  if (!totalVoters.value) return ''
  const mc = majorityCount.value
  if (mc === totalVoters.value) return `${mc}/${totalVoters.value} 一致通过`
  if (mc >= 2) return `${mc}/${totalVoters.value} 多数`
  return '未形成多数'
})

const majorityStatus = computed(() => {
  if (majorityCount.value >= 2 && majorityCount.value === totalVoters.value) return 'ok'
  if (majorityCount.value >= 2) return 'ok'
  return 'warn'
})

const hasBlockers = computed(() => seatsWithVotes.value.some((s) => s.blocking))

// ── SVG layout ──
const seatX = 40
const propX = 240
const vbW = 360
const vbH = computed(() => Math.max(200, seatsWithVotes.value.length * 70 + 30))

const seatNodes = computed(() =>
  seatsWithVotes.value.map((sw, i) => ({
    seat: sw.seat,
    label: seatLabels[sw.seat as SeatKind] ?? sw.seat,
    x: seatX,
    y: 50 + i * 70,
    color: seatColors[sw.seat] ?? '#666',
    bg: seatBgs[sw.seat] ?? '#f0f0f0',
    proposalId: sw.proposalId,
    blocking: sw.blocking,
  })),
)

const propNodes = computed(() => {
  const unique = new Map<string, string>()
  for (const sw of seatsWithVotes.value) {
    if (sw.proposalId && !unique.has(sw.proposalId)) {
      const p = props.proposals.find((pp) => pp.id === sw.proposalId)
      unique.set(sw.proposalId, p?.title ?? sw.proposalId)
    }
  }
  const entries = Array.from(unique.entries())
  return entries.map(([id, title], i) => ({
    id,
    label: title.length > 6 ? title.slice(0, 6) + '…' : title,
    x: propX,
    y: 50 + i * (vbH.value / entries.length || 70),
    w: Math.min(title.length * 14 + 40, 100),
    h: 32,
    majority: id === majorityProposal.value,
  }))
})

const lines = computed(() => {
  const result: Array<{
    x1: number; y1: number; x2: number; y2: number
    color: string; majority: boolean; blocked: boolean
  }> = []
  const pMap = new Map(propNodes.value.map((p) => [p.id, p]))
  for (const sn of seatNodes.value) {
    const pn = sn.proposalId ? pMap.get(sn.proposalId) : null
    if (!pn) continue
    result.push({
      x1: sn.x + 18, y1: sn.y,
      x2: pn.x - pn.w / 2, y2: pn.y,
      color: seatColors[sn.seat] ?? '#999',
      majority: sn.proposalId === majorityProposal.value,
      blocked: sn.blocking,
    })
  }
  return result
})

const blockers = computed(() =>
  seatNodes.value
    .filter((sn) => sn.blocking)
    .map((sn) => ({ seat: sn.seat, x: sn.x - 22, y: sn.y - 22 })),
)
</script>

<style scoped>
.vote-graph-wrap {
  text-align: center;
}
.vote-graph-svg {
  width: 100%;
  max-width: 360px;
  height: auto;
}
.vg-seat-label {
  font-size: 11px;
  font-weight: 700;
  pointer-events: none;
}
.vg-prop-label {
  font-size: 11px;
  pointer-events: none;
}
.vg-summary {
  display: flex;
  gap: 8px;
  justify-content: center;
  margin-top: 12px;
}
</style>
