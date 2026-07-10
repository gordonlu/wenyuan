<template>
  <div class="tab-content">
    <nav class="tab-nav">
      <a href="#decision-summary">结论</a>
      <a v-if="votes.length" href="#decision-votes">投票</a>
      <a v-if="hasDecisionObjects" href="#decision-objects">决策对象</a>
    </nav>

    <div id="decision-summary">
      <DecisionSummary
        v-if="primaryDecision"
        :decision="primaryDecision"
        :vote-policy="votePolicy"
        :mode="mode"
      />
      <p v-else class="muted" style="padding: var(--space-md)">讨论进行中，结论将在合议完成后生成。</p>
    </div>

    <div id="decision-votes">
      <VoteConvergenceGraph v-if="votes.length" :votes="votes" :proposals="proposals" />
      <VoteDisplay v-if="votes.length" :votes="votes" :proposals="proposals" />
      <p v-else class="muted" style="padding: var(--space-md)">暂无投票数据。</p>
    </div>

    <VoteChanges
      v-if="votes.length"
      :votes="votes"
      :proposals="proposals"
    />

    <DecisionObjectsPanel
      id="decision-objects"
      v-if="hasDecisionObjects"
      :objects="decisionObjects"
      @resolve="$emit('resolveObject', $event)"
      @dismiss="$emit('dismissObject', $event)"
    />
    <p v-else class="muted" style="padding: var(--space-md)">暂无决策对象。合议完成后将自动生成。</p>
  </div>
</template>

<script setup lang="ts">
import type { SeatKind } from '../../domain/session'
import DecisionSummary from '../DecisionSummary.vue'
import VoteConvergenceGraph from '../VoteConvergenceGraph.vue'
import VoteDisplay from '../VoteDisplay.vue'
import VoteChanges from '../VoteChanges.vue'
import DecisionObjectsPanel from '../DecisionObjectsPanel.vue'

defineProps<{
  primaryDecision?: any
  votePolicy?: any
  mode: string
  votes: any[]
  proposals: any[]
  hasDecisionObjects: boolean
  decisionObjects: any[]
}>()

defineEmits<{
  resolveObject: [id: string]
  dismissObject: [id: string]
}>()
</script>
