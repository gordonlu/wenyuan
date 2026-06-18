<template>
  <section v-if="details" class="page workspace">
    <header class="page-head row-head">
      <div>
        <p>{{ phaseLabels[details.session.phase] }}</p>
        <h1>{{ details.session.title }}</h1>
      </div>
      <div class="actions">
        <button class="icon" title="导出 Markdown" @click="downloadMarkdown">
          <Download :size="18" />
        </button>
        <button class="icon" title="重试" @click="retry">
          <RotateCw :size="18" />
        </button>
        <button class="icon danger" title="取消" @click="cancel">
          <Ban :size="18" />
        </button>
      </div>
    </header>

    <ApiErrorState :message="error" />
    <ApiErrorState v-if="details.session.failure_reason" :message="`失败原因：${details.session.failure_reason}`" />
    <p v-if="details.execution.recovery_state === 'retry_required'" class="notice" role="status">
      上次执行未正常完成，请使用重试继续。
    </p>
    <p v-else-if="details.execution.running" class="notice" role="status">
      当前议题正在执行中，租约到期时间：{{ details.execution.lease_expires_at }}
    </p>
    <SeatStatusStrip :phase="details.session.phase" :events="details.events" />

    <section class="panel">
      <h2>议题</h2>
      <p>{{ details.session.topic }}</p>
      <p v-if="details.session.context" class="muted">{{ details.session.context }}</p>
    </section>

    <section class="panel">
      <h2>Idea 池</h2>
      <div class="item-grid">
        <article v-for="idea in details.artifacts.ideas" :key="idea.id" class="item">
          <span>{{ seatLabels[idea.proposed_by] }}</span>
          <h3>{{ idea.title }}</h3>
          <p>{{ idea.summary }}</p>
          <p v-if="idea.source_seats?.length && idea.source_seats.length > 1" class="muted">
            合并来源：{{ idea.source_seats.map((seat) => seatLabels[seat]).join('、') }}
          </p>
          <p v-if="idea.unconventional" class="muted">非主流方向</p>
          <p v-if="idea.assumptions?.length" class="muted">假设：{{ idea.assumptions.join('、') }}</p>
          <p v-if="idea.risks?.length" class="muted">风险：{{ idea.risks.join('、') }}</p>
        </article>
      </div>
    </section>

    <section class="panel">
      <h2>批议摘要</h2>
      <div class="item-grid">
        <article v-for="critique in details.artifacts.critiques" :key="`${critique.reviewer}-${critique.target_seat}`" class="item">
          <span>{{ seatLabels[critique.reviewer] }} → {{ seatLabels[critique.target_seat] }}</span>
          <p class="muted">强点：{{ critique.strongest_point || '暂无' }}</p>
          <p class="muted">弱点：{{ critique.weakest_point || '暂无' }}</p>
          <p>{{ critique.challenge }}</p>
          <p v-if="critique.counterexample" class="muted">反例：{{ critique.counterexample }}</p>
          <p class="muted">{{ critique.suggested_improvement }}</p>
          <p v-if="critique.evidence_question" class="muted">补证：{{ critique.evidence_question }}</p>
        </article>
      </div>
    </section>

    <section class="panel">
      <h2>独议 / 复议差异</h2>
      <div class="item-grid">
        <article v-for="diff in revisionDiffs(details)" :key="diff.seat" class="item">
          <span>{{ seatLabels[diff.seat] }}</span>
          <h3>{{ diff.proposalTitle || '暂无复议策案' }}</h3>
          <p class="muted">采纳独议：{{ diff.ideaTitles.join('、') || '暂无' }}</p>
          <p>
            {{ diff.titleChanged || diff.summaryChanged ? '复议已调整表达或方向' : '复议基本延续独议' }}
          </p>
          <p v-if="diff.initialSummary" class="muted">独议：{{ diff.initialSummary }}</p>
          <p v-if="diff.revisedSummary">复议：{{ diff.revisedSummary }}</p>
          <p v-if="diff.addedImplementationPath" class="muted">落地：{{ diff.addedImplementationPath }}</p>
          <p v-if="diff.addedSuccessMetrics.length" class="muted">指标：{{ diff.addedSuccessMetrics.join('、') }}</p>
          <p v-if="details.artifacts.proposals.find((proposal) => proposal.proposed_by === diff.seat)?.changes_from_initial?.length" class="muted">
            修改：{{ details.artifacts.proposals.find((proposal) => proposal.proposed_by === diff.seat)?.changes_from_initial?.join('、') }}
          </p>
        </article>
      </div>
    </section>

    <section class="panel">
      <h2>策案对比</h2>
      <div class="item-grid proposals">
        <article v-for="proposal in details.artifacts.proposals" :key="proposal.id" class="item">
          <span>{{ seatLabels[proposal.proposed_by] }}</span>
          <h3>{{ proposal.title }}</h3>
          <p>{{ proposal.summary }}</p>
          <p class="muted">{{ proposal.implementation_path }}</p>
          <p v-if="proposal.adopted_points?.length" class="muted">采纳：{{ proposal.adopted_points.join('、') }}</p>
          <p v-if="proposal.rejected_points?.length" class="muted">拒绝：{{ proposal.rejected_points.join('、') }}</p>
          <p v-if="proposal.rejection_reasons?.length" class="muted">理由：{{ proposal.rejection_reasons.join('、') }}</p>
          <p v-if="proposal.confidence !== undefined" class="muted">置信度：{{ Math.round(proposal.confidence * 100) }}%</p>
        </article>
      </div>
    </section>

    <section class="panel">
      <h2>讨论质量</h2>
      <div class="stat-grid">
        <article v-for="metric in qualityMetricRows(details.artifacts.quality)" :key="metric.label" class="stat">
          <span>{{ metric.label }}</span>
          <strong>{{ metric.value }}</strong>
        </article>
      </div>
    </section>

    <section class="panel">
      <h2>运行统计</h2>
      <div class="stat-grid">
        <article v-for="stat in seatRunStats(details.artifacts.seat_runs)" :key="stat.seat" class="stat">
          <span>{{ seatLabels[stat.seat] }}</span>
          <strong>{{ stat.tokens }} tokens</strong>
          <p>{{ stat.calls }} 次调用 · {{ stat.durationMs }} ms · {{ stat.failed }} 次失败 · {{ stat.repaired }} 次修复</p>
          <p class="muted">{{ stat.promptVersions || '暂无 Prompt 版本' }}</p>
        </article>
      </div>
    </section>

    <DecisionSummary :decision="details.session.result ?? details.artifacts.decision" />

    <section class="panel">
      <h2>事件时间线</h2>
      <ol class="timeline">
        <li v-for="event in details.events" :key="event.id">
          <span>{{ new Date(event.created_at).toLocaleString() }}</span>
          {{ event.event_type }}
        </li>
      </ol>
    </section>
  </section>
  <section v-else class="page">
    <ApiErrorState :message="error || '加载中'" />
  </section>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { Ban, Download, RotateCw } from '@lucide/vue'
import { api } from '../api'
import ApiErrorState from '../components/ApiErrorState.vue'
import DecisionSummary from '../components/DecisionSummary.vue'
import SeatStatusStrip from '../components/SeatStatusStrip.vue'
import { exportSessionMarkdown, phaseLabels, qualityMetricRows, revisionDiffs, seatLabels, seatRunStats, type SessionDetails } from '../domain/session'

const route = useRoute()
const id = computed(() => String(route.params.id))
const details = ref<SessionDetails | null>(null)
const error = ref('')
let source: EventSource | null = null
let timer: number | undefined

async function load() {
  try {
    details.value = await api.getSession(id.value)
  } catch (err) {
    error.value = err instanceof Error ? err.message : '加载失败'
  }
}

async function retry() {
  details.value = await api.retrySession(id.value)
  timer = window.setTimeout(load, 500)
}

async function cancel() {
  details.value = await api.cancelSession(id.value)
}

function downloadMarkdown() {
  if (!details.value) return
  const markdown = exportSessionMarkdown(details.value)
  const blob = new Blob([markdown], { type: 'text/markdown;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = `${safeFilename(details.value.session.title)}.md`
  link.click()
  URL.revokeObjectURL(url)
}

function safeFilename(value: string) {
  return value.trim().replace(/[\\/:*?"<>|]+/g, '-').replace(/\s+/g, '-').slice(0, 80) || 'wenyuan-session'
}

onMounted(async () => {
  await load()
  source = new EventSource(`/api/sessions/${id.value}/events`)
  source.onmessage = () => load()
  timer = window.setInterval(load, 1200)
})

onBeforeUnmount(() => {
  source?.close()
  if (timer) window.clearInterval(timer)
})
</script>
