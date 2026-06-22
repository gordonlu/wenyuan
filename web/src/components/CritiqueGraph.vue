<template>
  <section class="panel critique-graph">
    <h2>批议关系图</h2>
    <div class="graph">
      <div class="graph-layer">
        <h3>三席批议流向</h3>
        <div class="flow-grid">
          <div v-for="(row, ri) in flowMatrix" :key="ri" class="flow-row">
            <span class="seat-label">{{ seatLabels[row.from] }}</span>
            <div class="flow-cell">
              <template v-for="(cell, ci) in row.cells" :key="ci">
                <span v-if="cell.count" :class="['flow-arrow', cell.count > 1 ? 'active' : '']">
                  {{ '→' }} {{ seatLabels[cell.to] }} ×{{ cell.count }}
                </span>
              </template>
            </div>
          </div>
        </div>
      </div>

      <div class="graph-layer">
        <h3>创意 → 策案引用</h3>
        <div class="ref-list">
          <p v-for="item in ideaProposalLinks" :key="item.ideaId" class="ref-row">
            <span :class="['seat-tag', item.seat]">{{ seatLabels[item.seat] }}</span>
            <span class="sep">「{{ item.ideaTitle }}」</span>
            <span class="arrow">→</span>
            <span class="sep">「{{ item.proposalTitle }}」</span>
          </p>
          <p v-if="!ideaProposalLinks.length" class="muted">暂无引用关系</p>
        </div>
      </div>

      <div class="graph-layer">
        <h3>批议补证请求</h3>
        <div class="ref-list">
          <p v-for="c in critiquesWithEvidence" :key="c.reviewer + c.target_seat" class="ref-row">
            <span class="tag">{{ seatLabels[c.reviewer] }}</span>
            <span class="arrow">要求</span>
            <span class="tag">{{ seatLabels[c.target_seat] }}</span>
            <span class="sep">补证：{{ c.evidence_question }}</span>
          </p>
          <p v-if="!critiquesWithEvidence.length" class="muted">暂无补证请求</p>
        </div>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { seatLabels, type Critique, type IdeaCard, type Proposal, type SeatKind } from '../domain/session'

const props = defineProps<{
  ideas: IdeaCard[]
  critiques: Critique[]
  proposals: Proposal[]
}>()

const seats: SeatKind[] = ['mouyuan', 'jingshi', 'chizheng']

const flowMatrix = computed(() => {
  return seats.map((from) => ({
    from,
    cells: seats
      .filter((to) => to !== from)
      .map((to) => {
        const count = props.critiques.filter((c) => c.reviewer === from && c.target_seat === to).length
        return { to, count }
      }),
  }))
})

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

const critiquesWithEvidence = computed(() => {
  return props.critiques.filter((c) => c.evidence_question?.trim())
})
</script>

<style scoped>
.graph { display: flex; flex-direction: column; gap: 20px; }
.graph-layer { background: var(--color-bg); border: 1px solid var(--color-border); border-radius: var(--radius); padding: 16px; }
.graph-layer h3 { font-size: 14px; margin-bottom: 12px; color: var(--color-text-muted); }
.flow-grid { display: flex; flex-direction: column; gap: 8px; }
.flow-row { display: flex; align-items: center; gap: 12px; }
.seat-label { min-width: 56px; font-weight: 600; font-size: 13px; }
.flow-cell { display: flex; gap: 12px; flex-wrap: wrap; }
.flow-arrow { font-size: 13px; color: var(--color-text-muted); }
.flow-arrow.active { color: var(--color-accent); font-weight: 600; }
.ref-list { display: flex; flex-direction: column; gap: 6px; }
.ref-row { display: flex; align-items: center; gap: 6px; font-size: 13px; flex-wrap: wrap; }
.tag { background: var(--color-bg-dim); border: 1px solid var(--color-border); border-radius: 4px; padding: 1px 8px; font-size: 12px; white-space: nowrap; }
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
.arrow { color: var(--color-text-muted); margin: 0 2px; }
.sep { color: var(--color-text); }
</style>
