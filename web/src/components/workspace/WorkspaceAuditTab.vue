<template>
  <div class="tab-content">
    <nav class="tab-nav">
      <a v-if="failedRuns.length" href="#audit-failures">失败</a>
      <a href="#audit-stats">统计</a>
      <a href="#audit-timeline">时间线</a>
      <a v-if="scribeReport" href="#audit-report">深度报告</a>
    </nav>

    <section id="audit-failures" v-if="failedRuns.length" class="panel">
      <h2>最近失败调用</h2>
      <ul class="failure-list">
        <li v-for="run in failedRuns" :key="run.id">
          <span :class="['seat-tag', run.seat]">{{ run.seatLabel }} · {{ run.phaseLabel }}</span>
          <strong>{{ run.error || '模型返回内容无法解析' }}</strong>
        </li>
      </ul>
    </section>

    <section id="audit-stats" class="panel">
      <h2>运行统计</h2>
      <p v-if="seatRuns.length && !hasTokenUsage" class="muted usage-note">
        当前 Provider 未返回 token usage；费用和额度请以供应商按调用次数或控制台账单为准。
      </p>
      <div class="stat-grid">
        <article v-for="stat in seatStats" :key="stat.seat" class="stat">
          <span :class="['seat-tag', stat.seat]">{{ stat.seatLabel }}</span>
          <strong>{{ stat.calls }} 次调用</strong>
          <p>
            {{ stat.duration }} · {{ stat.failed }} 次失败 · {{ stat.repaired }} 次修复
            <template v-if="stat.hasUsage"> · {{ stat.tokens }} tokens</template>
          </p>
          <p class="muted">{{ stat.promptVersions || '暂无 Prompt 版本' }}</p>
        </article>
      </div>
    </section>

    <section id="audit-timeline" class="panel">
      <div class="row-head timeline-head">
        <h2>事件时间线</h2>
        <button v-if="!showTrajectory" class="stat-action" title="查看阶段轨迹" @click="$emit('loadTrajectory')">
          <RotateCw :size="14" /> 查看阶段轨迹
        </button>
      </div>
      <div class="timeline-box">
        <ol class="timeline">
          <li v-for="event in timelineEvents" :key="event.id">
            <time v-if="event.time">{{ event.time }}</time>
            <span :class="['badge', event.badgeClass]">{{ event.label }}</span>
          </li>
        </ol>
      </div>
      <div v-if="showTrajectory && trajectoryEvents.length" class="trajectory-block">
        <h3>阶段轨迹</h3>
        <div class="timeline-box compact">
          <ol class="timeline">
            <li v-for="ev in trajectoryEvents" :key="ev.id">
              <time>{{ ev.time }}</time>
              <span class="badge ok">{{ ev.label }}</span>
            </li>
          </ol>
        </div>
      </div>
    </section>

    <section id="audit-report" v-if="scribeReport" class="panel">
      <h2>深度研究报告</h2>
      <div class="scribe-report">
        <h3>共识总结</h3>
        <div class="formatted-text" v-html="scribeReport.consensusHtml" />
        <div v-if="scribeReport.gaps.length">
          <h3>结构缺失</h3>
          <ul>
            <li v-for="gap in scribeReport.gaps" :key="gap">{{ gap }}</li>
          </ul>
        </div>
        <div v-if="scribeReport.conflicts.length">
          <h3>未解决分歧</h3>
          <ul>
            <li v-for="conflict in scribeReport.conflicts" :key="conflict">{{ conflict }}</li>
          </ul>
        </div>
        <details>
          <summary>研究全文</summary>
          <div class="scribe-final-report formatted-text" v-html="scribeReport.finalHtml" />
        </details>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { RotateCw } from '@lucide/vue'
import type { SeatKind, SessionPhase } from '../../domain/session'

defineProps<{
  failedRuns: {
    id: string
    seat: SeatKind
    phase: SessionPhase
    seatLabel: string
    phaseLabel: string
    error?: string
  }[]
  seatRuns: any[]
  seatStats: {
    seat: string
    seatLabel: string
    calls: number
    duration: string
    failed: number
    repaired: number
    hasUsage: boolean
    tokens?: string
    promptVersions?: string
  }[]
  hasTokenUsage: boolean
  showTrajectory: boolean
  timelineEvents: {
    id: string
    time?: string
    label: string
    badgeClass: string
  }[]
  trajectoryEvents: {
    id: string
    time?: string
    label: string
  }[]
  scribeReport?: {
    consensusHtml: string
    gaps: string[]
    conflicts: string[]
    finalHtml: string
  } | null
}>()

defineEmits<{
  loadTrajectory: []
}>()
</script>
