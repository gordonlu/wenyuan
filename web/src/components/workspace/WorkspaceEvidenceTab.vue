<template>
  <div class="tab-content">
    <nav class="tab-nav">
      <a href="#evidence-summary">总览</a>
      <a v-if="externalEvidence.length" href="#evidence-sources">证据</a>
      <a v-if="toolRuns.length" href="#evidence-tools">工具</a>
      <a v-if="supportedClaims.length" href="#evidence-claims">主张</a>
    </nav>

    <div id="evidence-summary">
      <EvidenceSummary
        v-if="evidenceSummary"
        element-id="quality"
        :summary="evidenceSummary"
        :donut-segments="donutData"
        :radar-axes="radarData"
        :quality-metrics="qualityMetrics"
      />
    </div>

    <section id="evidence-sources" v-if="externalEvidence.length" class="panel evidence-source-panel">
      <div class="row-head">
        <h2>来源证据</h2>
        <span class="badge flat">{{ externalEvidence.length }} 条</span>
      </div>
      <div class="item-grid evidence-source-grid">
        <article v-for="ev in externalEvidence.slice(0, 12)" :key="ev.id" class="item evidence-source-item">
          <div class="item-head">
            <span>{{ ev.kindLabel }}</span>
            <span :class="['badge', ev.trustClass]">{{ ev.trustLabel }}</span>
          </div>
          <div class="formatted-text" v-html="ev.contentHtml" />
          <p class="muted evidence-source-url">{{ ev.source }}</p>
          <div v-if="ev.safetyLabels.length" class="evidence-safety-row">
            <span v-for="label in ev.safetyLabels" :key="label" class="badge warn">{{ label }}</span>
          </div>
        </article>
      </div>
    </section>

    <section id="evidence-tools" v-if="toolRuns.length" class="panel tool-run-panel">
      <div class="row-head">
        <h2>工具轨迹</h2>
        <button class="icon" title="展开详情" @click="showDetail = !showDetail">
          <ChevronDown v-if="!showDetail" :size="16" />
          <ChevronUp v-else :size="16" />
        </button>
      </div>
      <div v-if="!showDetail" class="tool-run-summary">
        <span v-for="(count, name) in toolRunSummary" :key="name" class="tool-run-chip">
          {{ name }} {{ count }} 次
        </span>
        <span class="tool-run-meta">{{ toolRunDuration }} 秒 · {{ toolRunFailed }} 次失败</span>
      </div>
      <div v-else class="item-grid tool-run-grid">
        <article v-for="run in toolRuns" :key="run.id" class="item tool-run-item">
          <div class="item-head">
            <span>{{ run.toolName }}</span>
            <span :class="['badge', run.status === 'completed' ? 'ok' : 'warn']">{{ run.status }}</span>
          </div>
          <p>{{ run.summary }}</p>
          <p class="muted">{{ run.duration }} 秒 · {{ run.evidenceCount }} 条证据</p>
          <p v-if="run.error" class="muted tool-run-error">{{ run.error }}</p>
        </article>
      </div>
    </section>

    <div id="evidence-claims">
      <section v-if="supportedClaims.length" class="panel">
        <h2>有证据的主张</h2>
        <div class="item-grid">
          <article v-for="claim in supportedClaims" :key="claim.id" class="item">
            <div class="item-head">
              <span :class="['seat-tag', claim.seat]">{{ claim.seatLabel }}</span>
              <span class="badge ok">有证据</span>
            </div>
            <div class="formatted-text" v-html="claim.contentHtml" />
            <p class="muted">来源：{{ claim.contextText }}</p>
            <p v-if="claim.evidenceDetail" class="muted">证据：{{ claim.evidenceDetail }}</p>
          </article>
        </div>
      </section>
      <section v-if="unsupportedClaims.length" class="panel">
        <h2>未验证的主张</h2>
        <div class="item-grid">
          <article v-for="claim in unsupportedClaims" :key="claim.id" class="item">
            <div class="item-head">
              <span :class="['seat-tag', claim.seat]">{{ claim.seatLabel }}</span>
              <span class="badge warn">未验证</span>
            </div>
            <div class="formatted-text" v-html="claim.contentHtml" />
            <p class="muted">来源：{{ claim.contextText }}</p>
            <p v-if="claim.evidenceDetail" class="muted">证据：{{ claim.evidenceDetail }}</p>
          </article>
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { ChevronDown, ChevronUp } from '@lucide/vue'
import type { EvidenceSummary as EvidenceSummaryType } from '../../domain/session'
import type { DonutSegment } from '../charts/DonutChart.vue'
import type { RadarAxis } from '../charts/RadarChart.vue'
import EvidenceSummary from '../EvidenceSummary.vue'

defineProps<{
  evidenceSummary?: EvidenceSummaryType | null
  donutData: DonutSegment[]
  radarData: RadarAxis[]
  qualityMetrics: { label: string; value: string }[]
  externalEvidence: {
    id: string
    kindLabel: string
    trustClass: string
    trustLabel: string
    contentHtml: string
    source: string
    safetyLabels: string[]
  }[]
  toolRuns: {
    id: string
    toolName: string
    status: string
    summary: string
    duration: string
    evidenceCount: number
    error?: string
  }[]
  toolRunSummary: Record<string, number>
  toolRunDuration: string
  toolRunFailed: number
  supportedClaims: {
    id: string
    seat: string
    seatLabel: string
    contentHtml: string
    contextText: string
    evidenceDetail?: string
  }[]
  unsupportedClaims: {
    id: string
    seat: string
    seatLabel: string
    contentHtml: string
    contextText: string
    evidenceDetail?: string
  }[]
}>()

const showDetail = ref(false)
</script>
