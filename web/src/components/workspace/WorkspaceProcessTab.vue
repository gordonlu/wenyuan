<template>
  <div class="tab-content">
    <nav class="tab-nav">
      <a href="#process-topic">议题</a>
      <a href="#process-ideas">创意</a>
      <a href="#process-critiques">批议</a>
      <a href="#process-revisions">差异</a>
      <a href="#process-proposals">策案</a>
    </nav>

    <section id="process-topic" class="panel">
      <div class="row-head">
        <h2>议题</h2>
        <button v-if="!editingContext" class="icon" title="补充背景" @click="$emit('startEditContext')">
          <Pen :size="16" />
        </button>
      </div>
      <div class="formatted-text" v-html="topicHtml" />
      <template v-if="editingContext">
        <textarea v-model="editText" rows="4" placeholder="补充背景信息…" style="margin-top: 12px" />
        <div class="actions" style="margin-top: 8px">
          <button @click="saveContext">保存</button>
          <button @click="$emit('cancelEditContext')">取消</button>
        </div>
      </template>
      <div v-else-if="contextHtml" class="formatted-text muted" style="margin-top: 8px" v-html="contextHtml" />
    </section>

    <section id="process-ideas" class="panel">
      <h2>创意池（{{ ideas.length }}）</h2>
      <div class="item-grid">
        <IdeaCard v-for="idea in ideas" :key="idea.id" :idea="idea" />
      </div>
    </section>

    <section id="process-critiques" class="panel">
      <h2>批议摘要</h2>
      <div class="item-grid">
        <article v-for="critique in critiques" :key="`${(critique.reviewer as SeatKind)}-${(critique.target_seat as SeatKind)}`" class="item">
          <div class="item-head">
            <span :class="['seat-tag', critique.reviewer]">{{ seatLabels[(critique.reviewer as SeatKind)] }}</span> → <span :class="['seat-tag', critique.target_seat]">{{ seatLabels[(critique.target_seat as SeatKind)] }}</span>
          </div>
          <p class="muted" v-if="critique.strongest_point">强点：{{ critique.strongest_point }}</p>
          <p class="muted" v-if="critique.weakest_point">弱点：{{ critique.weakest_point }}</p>
          <p>{{ critique.challenge }}</p>
          <p v-if="critique.counterexample" class="muted">反例：{{ critique.counterexample }}</p>
          <p class="muted">{{ critique.suggested_improvement }}</p>
          <p v-if="critique.evidence_question" class="muted">补证：{{ critique.evidence_question }}</p>
        </article>
      </div>
    </section>

    <CritiqueGraph
      v-if="critiques.length"
      :ideas="ideas"
      :critiques="critiques"
      :proposals="proposals"
    />

    <section id="process-revisions" class="panel">
      <h2>独议 / 复议差异</h2>
      <div class="item-grid">
        <article v-for="diff in revisionDiffs" :key="diff.seat" class="item">
          <div class="item-head">
            <span :class="['seat-tag', diff.seat]">{{ seatLabels[(diff.seat as SeatKind)] }}</span>
            <span v-if="diff.titleChanged || diff.summaryChanged" class="badge ok">已调整</span>
            <span v-else class="badge">延续</span>
          </div>
          <h3>{{ diff.proposalTitle || '暂无复议策案' }}</h3>
          <p class="muted">采纳独议：{{ diff.ideaTitles.join('、') || '暂无' }}</p>
          <p v-if="diff.initialSummary" class="muted">独议：{{ diff.initialSummary }}</p>
          <p v-if="diff.revisedSummary">复议：{{ diff.revisedSummary }}</p>
          <p v-if="diff.addedImplementationPath" class="muted">落地：{{ diff.addedImplementationPath }}</p>
          <p v-if="diff.addedSuccessMetrics.length" class="muted">指标：{{ diff.addedSuccessMetrics.join('、') }}</p>
        </article>
      </div>
    </section>

    <ProposalCompare id="process-proposals" :proposals="proposals" :selected-id="selectedProposalId" />
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { Pen } from '@lucide/vue'
import { seatLabels, type SeatKind } from '../../domain/session'
import IdeaCard from '../IdeaCard.vue'
import CritiqueGraph from '../CritiqueGraph.vue'
import ProposalCompare from '../ProposalCompare.vue'

const props = defineProps<{
  topicHtml: string
  contextHtml: string
  editingContext: boolean
  editContextText: string
  ideas: any[]
  critiques: any[]
  proposals: any[]
  revisionDiffs: any[]
  selectedProposalId?: string
}>()

const emit = defineEmits<{
  startEditContext: []
  saveContext: [text: string]
  cancelEditContext: []
}>()

const editText = ref(props.editContextText)

watch(() => props.editingContext, (val) => {
  if (val) editText.value = props.editContextText
})

function saveContext() {
  emit('saveContext', editText.value)
}
</script>
