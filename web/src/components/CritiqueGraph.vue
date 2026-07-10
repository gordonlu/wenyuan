<template>
  <section class="panel critique-graph">
    <h2>批议关系图</h2>

    <div class="graph-main">
      <svg
        :viewBox="`0 0 ${vbWidth} ${vbHeight}`"
        class="tri-svg"
        role="figure"
        aria-label="三席批议三角关系图"
      >
        <!-- edges -->
        <g v-for="edge in visibleEdges" :key="`${edge.from}-${edge.to}`">
          <path
            :d="edge.path"
            fill="none"
            :stroke="edge.active ? edge.color : 'var(--color-border-light)'"
            :stroke-width="edge.active ? 2 : 1.2"
            :stroke-dasharray="edge.count === 0 ? '4,4' : 'none'"
            class="tri-edge"
            :class="{ 'edge-active': edge.active, 'edge-hover': edge.hover }"
            @click="selectEdge(edge)"
            @mouseenter="edge.hover = true"
            @mouseleave="edge.hover = false"
          />
          <polygon
            :points="edge.arrowHead"
            :fill="edge.active ? edge.color : 'var(--color-text-dim)'"
          />
          <rect
            v-if="edge.count > 0"
            :x="edge.labelX - 16"
            :y="edge.labelY - 10"
            width="32"
            height="20"
            rx="10"
            :fill="edge.active ? edge.color : 'var(--color-bg-subtle)'"
            :stroke="edge.active ? 'transparent' : 'var(--color-border-light)'"
            stroke-width="1"
          />
          <text
            v-if="edge.count > 0"
            :x="edge.labelX"
            :y="edge.labelY"
            text-anchor="middle"
            dominant-baseline="central"
            class="tri-edge-label"
            :fill="edge.active ? '#fff' : 'var(--color-text-muted)'"
          >{{ edge.count }}</text>
        </g>

        <!-- nodes -->
        <g
          v-for="node in triNodes"
          :key="node.seat"
          class="tri-node"
          :class="{ 'node-hover': hoveredNode === node.seat }"
          tabindex="0"
          role="button"
          :aria-label="`${node.label}席`"
          @mouseenter="hoveredNode = node.seat"
          @mouseleave="hoveredNode = null"
          @focus="hoveredNode = node.seat"
          @blur="hoveredNode = null"
        >
          <circle
            :cx="node.x"
            :cy="node.y"
            :r="nodeR"
            :fill="node.bg"
            stroke="var(--color-border)"
            stroke-width="1"
          />
          <text
            :x="node.x"
            :y="node.y"
            text-anchor="middle"
            dominant-baseline="central"
            class="tri-node-label"
            :fill="node.color"
          >{{ node.label }}</text>
        </g>
      </svg>
    </div>

    <div v-if="filteredCritiques.length" class="critique-detail">
      <div class="row-head">
        <h3 v-if="selectedEdge">{{ seatLabels[selectedEdge.from] }} → {{ seatLabels[selectedEdge.to] }} 批议</h3>
        <h3 v-else>详细批议</h3>
        <span class="badge flat">{{ filteredCritiques.length }} 条</span>
      </div>
      <div class="critique-list">
        <article
          v-for="c in filteredCritiques"
          :key="`${c.reviewer}-${c.target_seat}`"
          class="critique-item"
        >
          <p class="muted" v-if="c.strongest_point">强点：{{ c.strongest_point }}</p>
          <p class="muted" v-if="c.weakest_point">弱点：{{ c.weakest_point }}</p>
          <p>{{ c.challenge }}</p>
          <p v-if="c.counterexample" class="muted">反例：{{ c.counterexample }}</p>
          <p class="muted">{{ c.suggested_improvement }}</p>
          <p v-if="c.evidence_question" class="muted">补证：{{ c.evidence_question }}</p>
        </article>
      </div>
    </div>

    <div v-if="ideaProposalLinks.length" class="ref-section">
      <h3>创意 → 策案引用</h3>
      <div class="ref-list">
        <p v-for="item in ideaProposalLinks" :key="item.ideaId" class="ref-row">
          <span :class="['seat-tag', item.seat]">{{ seatLabels[item.seat] }}</span>
          <span>「{{ item.ideaTitle }}」→「{{ item.proposalTitle }}」</span>
        </p>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { seatLabels, type Critique, type IdeaCard, type Proposal, type SeatKind } from '../domain/session'

const props = defineProps<{
  ideas: IdeaCard[]
  critiques: Critique[]
  proposals: Proposal[]
}>()

// ── Seat configs ──
const seats: SeatKind[] = ['mouyuan', 'jingshi', 'chizheng']
const seatColors: Record<SeatKind, string> = {
  mouyuan: '#1a6ba0',
  jingshi: '#9a6a30',
  chizheng: '#a03040',
}
const seatBgs: Record<SeatKind, string> = {
  mouyuan: '#e2eef9',
  jingshi: '#f0e6d3',
  chizheng: '#f5e8e8',
}

// ── SVG layout ──
const vbWidth = 320
const vbHeight = 260
const nodeR = 28
const margin = nodeR + 16

const triNodes = computed(() => [
  { seat: 'mouyuan' as SeatKind, label: '谋远', x: 160, y: 50, color: seatColors.mouyuan, bg: seatBgs.mouyuan },
  { seat: 'jingshi' as SeatKind, label: '经世', x: 290, y: 230, color: seatColors.jingshi, bg: seatBgs.jingshi },
  { seat: 'chizheng' as SeatKind, label: '持正', x: 30, y: 230, color: seatColors.chizheng, bg: seatBgs.chizheng },
])

function nodePos(seat: SeatKind) {
  const n = triNodes.value.find((t) => t.seat === seat)
  return n ? { x: n.x, y: n.y } : { x: 0, y: 0 }
}

// ── Edge computation ──
interface TriEdge {
  from: SeatKind
  to: SeatKind
  count: number
  color: string
  path: string
  arrowHead: string
  labelX: number
  labelY: number
  active: boolean
  hover: boolean
}

const selectedEdge = ref<{ from: SeatKind; to: SeatKind } | null>(null)
const hoveredNode = ref<SeatKind | null>(null)

const flowMatrix = computed(() => {
  return seats.flatMap((from) =>
    seats.filter((to) => to !== from).map((to) => ({
      from,
      to,
      count: props.critiques.filter((c) => c.reviewer === from && c.target_seat === to).length,
    })),
  )
})

const visibleEdges = computed(() => {
  return flowMatrix.value.map(({ from, to, count }) => {
    const p1 = nodePos(from)
    const p2 = nodePos(to)
    const angle = Math.atan2(p2.y - p1.y, p2.x - p1.x)
    const gap = nodeR + 4

    // curve offset to separate bidirectional edges
    const perpAngle = angle + Math.PI / 2
    const curveOffset = count > 0 ? 18 : 8

    const sx = p1.x + gap * Math.cos(angle)
    const sy = p1.y + gap * Math.sin(angle)
    const ex = p2.x - gap * Math.cos(angle)
    const ey = p2.y - gap * Math.sin(angle)
    const cx = (sx + ex) / 2 + curveOffset * Math.cos(perpAngle)
    const cy = (sy + ey) / 2 + curveOffset * Math.sin(perpAngle)
    const path = `M ${sx.toFixed(1)} ${sy.toFixed(1)} Q ${cx.toFixed(1)} ${cy.toFixed(1)} ${ex.toFixed(1)} ${ey.toFixed(1)}`

    // arrow head at end
    const arrowLen = 8
    const ah1x = ex - arrowLen * Math.cos(angle - 0.5)
    const ah1y = ey - arrowLen * Math.sin(angle - 0.5)
    const ah2x = ex - arrowLen * Math.cos(angle + 0.5)
    const ah2y = ey - arrowLen * Math.sin(angle + 0.5)
    const arrowHead = `${ex.toFixed(1)},${ey.toFixed(1)} ${ah1x.toFixed(1)},${ah1y.toFixed(1)} ${ah2x.toFixed(1)},${ah2y.toFixed(1)}`

    // label at midpoint on curve
    const midT = 0.5
    const lx = (1 - midT) ** 2 * sx + 2 * (1 - midT) * midT * cx + midT ** 2 * ex
    const ly = (1 - midT) ** 2 * sy + 2 * (1 - midT) * midT * cy + midT ** 2 * ey

    const active = selectedEdge.value?.from === from && selectedEdge.value?.to === to

    return {
      from,
      to,
      count,
      color: seatColors[from],
      path,
      arrowHead,
      labelX: Math.round(lx),
      labelY: Math.round(ly),
      active,
      hover: false,
    } as TriEdge
  })
})

function selectEdge(edge: TriEdge) {
  if (selectedEdge.value?.from === edge.from && selectedEdge.value?.to === edge.to) {
    selectedEdge.value = null
  } else {
    selectedEdge.value = { from: edge.from, to: edge.to }
  }
}

const filteredCritiques = computed(() => {
  if (selectedEdge.value) {
    return props.critiques.filter(
      (c) => c.reviewer === selectedEdge.value!.from && c.target_seat === selectedEdge.value!.to,
    )
  }
  return props.critiques
})

// ── Idea→Proposal links ──
const ideaProposalLinks = computed(() => {
  const links: Array<{ ideaId: string; seat: SeatKind; ideaTitle: string; proposalTitle: string }> = []
  for (const proposal of props.proposals) {
    if (!proposal.source_idea_ids?.length) continue
    for (const ideaId of proposal.source_idea_ids) {
      const idea = props.ideas.find((i) => i.id === ideaId)
      if (idea) {
        links.push({
          ideaId: idea.id,
          seat: idea.proposed_by,
          ideaTitle: idea.title,
          proposalTitle: proposal.title,
        })
      }
    }
  }
  return links
})
</script>

<style scoped>
.graph-main {
  display: flex;
  justify-content: center;
  margin-bottom: 16px;
}
.tri-svg {
  width: 100%;
  max-width: 320px;
  height: auto;
}
.tri-node {
  cursor: pointer;
  outline: none;
}
.tri-node-label {
  font-size: 13px;
  font-weight: 700;
  pointer-events: none;
}
.tri-edge {
  cursor: pointer;
  transition: stroke 0.15s ease, stroke-width 0.15s ease;
}
.tri-edge-label {
  font-size: 12px;
  font-weight: 700;
  pointer-events: none;
}
.tri-edge.edge-active {
  stroke-width: 2.5px;
}
.tri-edge.edge-hover {
  opacity: 0.8;
}
.node-hover circle {
  filter: brightness(0.94);
}

.critique-detail {
  margin-bottom: 16px;
}
.critique-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.critique-item {
  padding: 10px 12px;
  background: var(--color-bg-subtle);
  border-radius: var(--radius-sm);
  font-size: 13px;
}
.critique-item p {
  margin: 2px 0;
}

.ref-section h3 {
  font-size: 13px;
  color: var(--color-text-muted);
  margin-bottom: 8px;
}
.ref-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 13px;
}

.seat-tag {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 5px;
  font-size: 12px;
  font-weight: 600;
  margin-right: 4px;
}
.seat-tag.mouyuan { background: #e2eef9; color: #1a5a8c; }
.seat-tag.jingshi { background: #f0e6d3; color: #7a5a2e; }
.seat-tag.chizheng { background: #f5e8e8; color: #8c3a3a; }
</style>
