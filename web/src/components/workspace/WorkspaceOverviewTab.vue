<template>
  <div class="tab-content">
    <nav class="tab-nav" v-if="showNav">
      <a href="#overview-status">运行状态</a>
      <a href="#overview-conclusion">当前结论</a>
    </nav>

    <PhaseProgressBar id="overview-status" :phase="phase" :running="running" :events="events" />

    <section class="role-card-row" aria-label="三席状态">
      <SeatRoleCard
        v-for="(s, i) in seatList"
        :key="s.seat"
        :seat="s.seat"
        :phase="phase"
        :events="events"
        :running="running"
        :runs="s.runs"
        :tool-runs="s.toolRuns"
        :provider-ref="s.providerRef"
        :inactive="s.inactive"
      />
    </section>

    <ApiErrorState v-if="errorMessage" :message="errorMessage" />
    <ApiErrorState v-if="failureReason" :message="`失败原因：${failureReason}`" />

    <section v-if="failedRuns.length" class="panel">
      <h2>最近失败调用</h2>
      <ul class="failure-list">
        <li v-for="run in failedRuns" :key="run.id">
          <span :class="['seat-tag', run.seat]">{{ run.seatLabel }} · {{ run.phaseLabel }}</span>
          <strong>{{ run.error || '模型返回内容无法解析' }}</strong>
        </li>
      </ul>
    </section>

    <div v-if="retryRequired" class="status-bar status-bar-warn" role="status">
      <span class="status-bar-icon">&#9888;</span>
      <span>上次执行未正常完成，请使用重试继续。</span>
    </div>
    <div v-else-if="paused" class="status-bar status-bar-warn" role="status">
      <span class="status-bar-icon">&#9208;</span>
      <span>已暂停。你可以补充背景信息后继续。</span>
    </div>
    <div v-else-if="running" class="status-bar status-bar-live" role="status" aria-live="polite">
      <span class="status-bar-dot" />
      <span class="status-bar-phase">{{ phaseLabel }}</span>
      <span class="status-bar-sep">·</span>
      <span class="status-bar-seat">{{ runningSeatLabel }}</span>
      <span v-if="runningActivityLabel" class="status-bar-tool">{{ runningActivityLabel }}</span>
      <span v-if="lastEventTime" class="status-bar-time">{{ lastEventTime }}</span>
    </div>

    <div v-if="currentDigest" class="digest-row" id="overview-conclusion">
      <DecisionDigest :digest="currentDigest" />
      <EvidenceSummary
        v-if="evidenceSummary"
        element-id="quality"
        :summary="evidenceSummary"
        :donut-segments="donutData"
        :radar-axes="radarData"
        :quality-metrics="qualityMetrics"
      />
    </div>

    <DecisionSummary
      v-if="primaryDecision"
      :decision="primaryDecision"
      :vote-policy="votePolicy"
      :mode="mode"
    />

    <section v-if="recentEvents.length" class="panel">
      <h2>最近事件</h2>
      <div class="timeline-box">
        <ol class="timeline">
          <li v-for="ev in recentEvents" :key="ev.id">
            <time>{{ ev.time }}</time>
            <span :class="['badge', ev.badgeClass]">{{ ev.label }}</span>
          </li>
        </ol>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { SeatKind, SessionPhase, DecisionDigest as DecisionDigestType, VotePolicy, SessionEvent } from '../../domain/session'
import type { EvidenceSummary as EvidenceSummaryType } from '../../domain/session'
import type { DonutSegment } from '../charts/DonutChart.vue'
import type { RadarAxis } from '../charts/RadarChart.vue'
import ApiErrorState from '../ApiErrorState.vue'
import PhaseProgressBar from '../PhaseProgressBar.vue'
import SeatRoleCard from '../SeatRoleCard.vue'
import DecisionDigest from '../DecisionDigest.vue'
import EvidenceSummary from '../EvidenceSummary.vue'
import DecisionSummary from '../DecisionSummary.vue'
import { seatLabels, phaseLabels } from '../../domain/session'

interface SeatSlot {
  seat: SeatKind
  runs: any[]
  toolRuns: any[]
  providerRef: string
  inactive: boolean
}

interface FailedRun {
  id: string
  seat: SeatKind
  phase: SessionPhase
  seatLabel: string
  phaseLabel: string
  error?: string
}

interface RecentEvent {
  id: string
  time: string
  label: string
  badgeClass: string
}

const props = defineProps<{
  phase: SessionPhase
  running: boolean
  events: SessionEvent[]
  seatList: SeatSlot[]
  errorMessage: string
  failureReason?: string
  failedRuns: FailedRun[]
  retryRequired: boolean
  paused: boolean
  phaseLabel: string
  runningSeatLabel: string
  runningActivityLabel: string
  lastEventTime: string
  currentDigest?: DecisionDigestType | null
  evidenceSummary?: EvidenceSummaryType | null
  donutData: DonutSegment[]
  radarData: RadarAxis[]
  qualityMetrics: { label: string; value: string }[]
  primaryDecision?: any
  votePolicy?: VotePolicy | null
  mode: string
  recentEvents: RecentEvent[]
}>()

const showNav = computed(() => props.paused || props.running || props.retryRequired || props.currentDigest)
</script>
