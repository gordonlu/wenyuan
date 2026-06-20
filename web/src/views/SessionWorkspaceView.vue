<template>
  <section v-if="details" :class="['page', 'workspace', { 'report-mode': viewMode === 'report' }]">
    <header class="page-head row-head">
      <div>
        <p class="phase-label">{{ phaseLabels[details.session.phase] }}</p>
        <h1>{{ details.session.title }}</h1>
        <span class="badge flat" style="margin-top: 4px">{{ modeLabels[details.session.mode] }}</span>
        <span v-if="details.session.vote_policy" class="badge flat" style="margin-top: 4px; margin-left: 6px">
          {{ voteStrategyLabels[details.session.vote_policy.strategy] }}
        </span>
      </div>
      <div class="actions workspace-actions">
        <div class="view-switch" role="tablist" aria-label="视图模式">
          <button
            :class="{ active: viewMode === 'workbench' }"
            type="button"
            role="tab"
            :aria-selected="viewMode === 'workbench'"
            @click="setViewMode('workbench')"
          >
            工作台
          </button>
          <button
            :class="{ active: viewMode === 'report' }"
            type="button"
            role="tab"
            :aria-selected="viewMode === 'report'"
            @click="setViewMode('report')"
          >
            报告
          </button>
        </div>
        <button title="复制最终方案" :disabled="!canCopyDecision" @click="copyDecision">
          <Copy :size="18" />
          复制
        </button>
        <div class="menu-wrap">
          <button title="导出" @click="showExportMenu = !showExportMenu">
            <Download :size="18" />
            导出
            <ChevronDown :size="16" />
          </button>
          <div v-if="showExportMenu" class="action-menu">
            <button @click="downloadAndClose('json')">JSON</button>
            <button @click="downloadAndClose('html')">HTML</button>
          </div>
        </div>
        <div class="menu-wrap">
          <button title="导出 Markdown" @click="showMdMenu = !showMdMenu">
            <FileText :size="18" />
            Markdown
            <ChevronDown :size="16" />
          </button>
          <div v-if="showMdMenu" class="action-menu">
            <button @click="downloadMarkdown('brief')">简报 (简明)</button>
            <button @click="downloadMarkdown('standard')">完整报告</button>
            <button @click="downloadMarkdown('audit')">审计全文</button>
          </div>
        </div>
        <template v-if="viewMode === 'workbench'">
          <button v-if="details.execution.running" class="icon" title="暂停" @click="pause">
            <Pause :size="18" />
          </button>
          <button v-else-if="details.execution.recovery_state === 'paused'" class="icon" title="继续" @click="resume">
            <Play :size="18" />
          </button>
          <button v-if="canManualRevision" class="icon" title="让三席重新修订方案" @click="manualRevision">
            <RotateCw :size="18" />
          </button>
          <button v-if="!details.execution.running && details.execution.recovery_state !== 'paused'" class="retry-phase" title="重试当前阶段" @click="retryCurrentPhase">
            <RefreshCw :size="18" />
            重试阶段
          </button>
          <button class="retry-session" title="重试整个议题" @click="retry">
            <RotateCw :size="18" />
            重新开议
          </button>
          <button class="icon danger" title="取消" @click="cancel">
            <Ban :size="18" />
          </button>
        </template>
      </div>
    </header>

    <template v-if="viewMode === 'workbench'">
    <PhaseProgressBar :phase="details.session.phase" />
    <section class="role-card-row" aria-label="三席状态">
      <SeatRoleCard
        v-for="seat in seats"
        :key="seat"
        :seat="seat"
        :phase="details.session.phase"
        :events="details.events"
        :running="details.execution.running"
        :runs="details.artifacts.seat_runs"
        :provider-ref="seatProviderRef(seat)"
        @retry="retrySeat"
      />
    </section>

    <ApiErrorState :message="error" />
    <ApiErrorState v-if="details.session.failure_reason" :message="`失败原因：${details.session.failure_reason}`" />
    <section v-if="recentFailedRuns.length" class="panel">
      <h2>最近失败调用</h2>
      <ul class="failure-list">
        <li v-for="run in recentFailedRuns" :key="run.id">
          <span>{{ seatLabels[run.seat] }} · {{ phaseLabels[run.phase] }}</span>
          <strong>{{ run.error || '模型返回内容无法解析' }}</strong>
        </li>
      </ul>
    </section>
    <p v-if="details.execution.recovery_state === 'retry_required'" class="notice" role="status">
      上次执行未正常完成，请使用重试继续。
    </p>
    <p v-else-if="details.execution.recovery_state === 'paused'" class="notice" role="status">
      已暂停。你可以补充背景信息后继续。
    </p>
    <p v-else-if="details.execution.running" class="notice" role="status">
      当前议题正在执行中。席位状态会实时更新；如果长时间没有变化，可以暂停或取消后重试。
    </p>

    <DecisionSummary v-if="primaryDecision" :decision="primaryDecision" :vote-policy="details.session.vote_policy" />

    <section class="panel">
      <div class="row-head">
        <h2>议题</h2>
        <button v-if="!editingContext" class="icon" title="补充背景" @click="editingContext = true">
          <Pen :size="16" />
        </button>
      </div>
      <p>{{ details.session.topic }}</p>
      <template v-if="editingContext">
        <textarea v-model="newContext" rows="4" placeholder="补充背景信息…" style="margin-top: 12px" />
        <div class="actions" style="margin-top: 8px">
          <button @click="saveContext">保存</button>
          <button @click="editingContext = false">取消</button>
        </div>
      </template>
      <p v-else-if="details.session.context" class="muted" style="margin-top: 8px">{{ details.session.context }}</p>
    </section>

    <section v-if="externalEvidence.length" class="panel evidence-source-panel">
      <div class="row-head">
        <h2>来源证据</h2>
        <span class="badge flat">{{ externalEvidence.length }} 条</span>
      </div>
      <div class="item-grid evidence-source-grid">
        <article v-for="ev in externalEvidence.slice(0, 12)" :key="ev.id" class="item evidence-source-item">
          <div class="item-head">
            <span>{{ evidenceSourceKindLabels[ev.source_kind ?? 'internal'] ?? ev.source_kind }}</span>
            <span :class="['badge', ev.trust_level === 'untrusted_external' ? 'warn' : 'ok']">
              {{ evidenceTrustLabels[ev.trust_level ?? 'internal'] ?? ev.trust_level }}
            </span>
          </div>
          <p>{{ ev.content }}</p>
          <p class="muted evidence-source-url">{{ compactSource(ev.source) }}</p>
          <div v-if="evidenceSafetyLabels(ev.safety_flags).length" class="evidence-safety-row">
            <span
              v-for="label in evidenceSafetyLabels(ev.safety_flags)"
              :key="label"
              class="badge warn"
            >
              {{ label }}
            </span>
          </div>
        </article>
      </div>
    </section>

    <section v-if="toolRuns.length" class="panel tool-run-panel">
      <div class="row-head">
        <h2>工具轨迹</h2>
        <span class="badge flat">{{ toolRuns.length }} 次</span>
      </div>
      <div class="item-grid tool-run-grid">
        <article v-for="run in toolRuns" :key="run.id" class="item tool-run-item">
          <div class="item-head">
            <span>{{ toolNameLabel(run.tool_name) }}</span>
            <span :class="['badge', run.status === 'completed' ? 'ok' : 'warn']">{{ run.status }}</span>
          </div>
          <p>{{ run.input_summary }}</p>
          <p class="muted">{{ run.duration_ms }} ms · {{ run.evidence_ids?.length ?? 0 }} 条证据</p>
          <p v-if="run.error" class="muted tool-run-error">{{ run.error }}</p>
        </article>
      </div>
    </section>

    <section class="panel">
      <h2>创意池（{{ details.artifacts.ideas.length }}）</h2>
      <div class="item-grid">
        <IdeaCard v-for="idea in details.artifacts.ideas" :key="idea.id" :idea="idea" />
      </div>
    </section>

    <section class="panel">
      <h2>批议摘要</h2>
      <div class="item-grid">
        <article v-for="critique in details.artifacts.critiques" :key="`${critique.reviewer}-${critique.target_seat}`" class="item">
          <div class="item-head">
            <span>{{ seatLabels[critique.reviewer] }} → {{ seatLabels[critique.target_seat] }}</span>
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
      v-if="details.artifacts.critiques.length"
      :ideas="details.artifacts.ideas"
      :critiques="details.artifacts.critiques"
      :proposals="details.artifacts.proposals"
    />

    <section class="panel">
      <h2>独议 / 复议差异</h2>
      <div class="item-grid">
        <article v-for="diff in revisionDiffs(details)" :key="diff.seat" class="item">
          <div class="item-head">
            <span>{{ seatLabels[diff.seat] }}</span>
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

    <ProposalCompare :proposals="details.artifacts.proposals" />

    <VoteDisplay :votes="details.artifacts.votes" :proposals="details.artifacts.proposals" />

    <VoteChanges :votes="details.artifacts.votes" :proposals="details.artifacts.proposals" />

    <section class="panel">
      <h2>讨论质量</h2>
      <div class="stat-grid">
        <article v-for="metric in qualityMetricRows(details.artifacts.quality, hasTokenUsage)" :key="metric.label" class="stat">
          <span>{{ metric.label }}</span>
          <strong>{{ metric.value }}</strong>
        </article>
      </div>
    </section>

    <section v-if="details.artifacts.scribe_report" class="panel">
      <h2>书记官报告</h2>
      <div class="scribe-report">
        <h3>共识总结</h3>
        <p>{{ details.artifacts.scribe_report.consensus_summary }}</p>
        <div v-if="details.artifacts.scribe_report.structural_gaps.length">
          <h3>结构缺失</h3>
          <ul>
            <li v-for="gap in details.artifacts.scribe_report.structural_gaps" :key="gap">{{ gap }}</li>
          </ul>
        </div>
        <div v-if="details.artifacts.scribe_report.unresolved_conflicts.length">
          <h3>未解决分歧</h3>
          <ul>
            <li v-for="conflict in details.artifacts.scribe_report.unresolved_conflicts" :key="conflict">{{ conflict }}</li>
          </ul>
        </div>
        <details>
          <summary>完整报告</summary>
          <div class="scribe-final-report">{{ details.artifacts.scribe_report.final_report }}</div>
        </details>
      </div>
    </section>

    <section v-if="details.artifacts.claims?.length" class="panel">
      <h2>证据池</h2>
      <div class="item-grid">
        <article v-for="claim in details.artifacts.claims" :key="claim.id" class="item">
          <div class="item-head">
            <span>{{ seatLabels[claim.proposed_by] }}</span>
            <span :class="['badge', claim.is_supported ? 'ok' : 'warn']">
              {{ claim.is_supported ? '有证据' : '未验证' }}
            </span>
          </div>
          <p>{{ claim.content }}</p>
          <p class="muted">来源：{{ claim.context }}</p>
          <p v-if="detailEvidence(claim.evidence_ids)" class="muted">
            证据：{{ detailEvidence(claim.evidence_ids)?.map((ev) => evidenceKindLabels[ev.kind] + ': ' + ev.content).join(' | ') }}
          </p>
        </article>
      </div>
    </section>

    <section class="panel">
      <h2>运行统计</h2>
      <p v-if="details.artifacts.seat_runs.length && !hasTokenUsage" class="muted usage-note">
        当前 Provider 未返回 token usage；费用和额度请以供应商按调用次数或控制台账单为准。
      </p>
      <div class="stat-grid">
        <article v-for="stat in seatRunStats(details.artifacts.seat_runs)" :key="stat.seat" class="stat">
          <span>{{ seatLabels[stat.seat] }}</span>
          <strong>{{ stat.calls }} 次调用</strong>
          <p>
            {{ stat.durationMs }} ms · {{ stat.failed }} 次失败 · {{ stat.repaired }} 次修复
            <template v-if="stat.hasUsage"> · {{ stat.tokens }} tokens</template>
          </p>
          <p class="muted">{{ stat.promptVersions || '暂无 Prompt 版本' }}</p>
          <button v-if="!details.execution.running && stat.failed > 0" class="stat-action" title="重试该席位" @click="retrySeat(stat.seat)">
            <RotateCw :size="14" /> 重试
          </button>
        </article>
      </div>
    </section>

    <section class="panel">
      <div class="row-head timeline-head">
        <h2>事件时间线</h2>
        <button v-if="!showTrajectory" class="stat-action" title="查看阶段轨迹" @click="loadTrajectory">
          <RotateCw :size="14" /> 查看阶段轨迹
        </button>
      </div>
      <div class="timeline-box">
        <ol class="timeline">
          <li v-for="event in timelineEvents" :key="event.id">
            <time>{{ new Date(event.created_at).toLocaleString() }}</time>
            <span :class="['badge', eventBadge(event.event_type)]">{{ event.event_type }}</span>
          </li>
        </ol>
      </div>
      <div v-if="showTrajectory && trajectory.length" class="trajectory-block">
        <h3>阶段轨迹</h3>
        <div class="timeline-box compact">
          <ol class="timeline">
            <li v-for="ev in trajectoryEvents" :key="ev.id">
              <time>{{ new Date(ev.created_at).toLocaleString() }}</time>
              <span class="badge ok">{{ ev.event_type }}</span>
            </li>
          </ol>
        </div>
      </div>
    </section>
    </template>

    <template v-else>
      <div class="report-meta">
        <span>{{ phaseLabels[details.session.phase] }}</span>
        <span>{{ modeLabels[details.session.mode] }}</span>
        <span>{{ details.session.id }}</span>
      </div>

      <section class="panel report-topic">
        <h2>议题</h2>
        <p>{{ details.session.topic }}</p>
        <p v-if="details.session.context" class="muted">{{ details.session.context }}</p>
      </section>

      <section v-if="externalEvidence.length" class="panel evidence-source-panel">
        <div class="row-head">
          <h2>来源证据</h2>
          <span class="badge flat">{{ externalEvidence.length }} 条</span>
        </div>
        <div class="item-grid evidence-source-grid">
          <article v-for="ev in externalEvidence.slice(0, 12)" :key="ev.id" class="item evidence-source-item">
            <div class="item-head">
              <span>{{ evidenceSourceKindLabels[ev.source_kind ?? 'internal'] ?? ev.source_kind }}</span>
              <span :class="['badge', ev.trust_level === 'untrusted_external' ? 'warn' : 'ok']">
                {{ evidenceTrustLabels[ev.trust_level ?? 'internal'] ?? ev.trust_level }}
              </span>
            </div>
            <p>{{ ev.content }}</p>
            <p class="muted evidence-source-url">{{ compactSource(ev.source) }}</p>
            <div v-if="evidenceSafetyLabels(ev.safety_flags).length" class="evidence-safety-row">
              <span
                v-for="label in evidenceSafetyLabels(ev.safety_flags)"
                :key="label"
                class="badge warn"
              >
                {{ label }}
              </span>
            </div>
          </article>
        </div>
      </section>

      <section v-if="toolRuns.length" class="panel tool-run-panel">
        <div class="row-head">
          <h2>工具轨迹</h2>
          <span class="badge flat">{{ toolRuns.length }} 次</span>
        </div>
        <div class="item-grid tool-run-grid">
          <article v-for="run in toolRuns" :key="run.id" class="item tool-run-item">
            <div class="item-head">
              <span>{{ toolNameLabel(run.tool_name) }}</span>
              <span :class="['badge', run.status === 'completed' ? 'ok' : 'warn']">{{ run.status }}</span>
            </div>
            <p>{{ run.input_summary }}</p>
            <p class="muted">{{ run.duration_ms }} ms · {{ run.evidence_ids?.length ?? 0 }} 条证据</p>
            <p v-if="run.error" class="muted tool-run-error">{{ run.error }}</p>
          </article>
        </div>
      </section>

    <DecisionSummary v-if="primaryDecision" :decision="primaryDecision" :vote-policy="details.session.vote_policy" />

      <section class="role-card-row report-seat-row" aria-label="三席报告">
        <SeatRoleCard
          v-for="seat in seats"
          :key="seat"
          :seat="seat"
          :phase="details.session.phase"
          :events="details.events"
          :running="false"
          :runs="details.artifacts.seat_runs"
          :provider-ref="seatProviderRef(seat)"
          report-mode
        />
      </section>

      <ProposalCompare :proposals="details.artifacts.proposals" />

      <VoteDisplay :votes="details.artifacts.votes" :proposals="details.artifacts.proposals" />

      <VoteChanges :votes="details.artifacts.votes" :proposals="details.artifacts.proposals" />

      <section class="panel">
        <h2>讨论质量</h2>
        <div class="stat-grid">
          <article v-for="metric in qualityMetricRows(details.artifacts.quality, hasTokenUsage)" :key="metric.label" class="stat">
            <span>{{ metric.label }}</span>
            <strong>{{ metric.value }}</strong>
          </article>
        </div>
      </section>

      <section v-if="details.artifacts.claims?.length" class="panel">
        <h2>证据与待核验判断</h2>
        <div class="item-grid">
          <article v-for="claim in details.artifacts.claims" :key="claim.id" class="item">
            <div class="item-head">
              <span>{{ seatLabels[claim.proposed_by] }}</span>
              <span :class="['badge', claim.is_supported ? 'ok' : 'warn']">
                {{ claim.is_supported ? '已有依据' : '仍需核验' }}
              </span>
            </div>
            <p>{{ claim.content }}</p>
            <p class="muted">来源：{{ claim.context }}</p>
            <p v-if="detailEvidence(claim.evidence_ids)" class="muted">
              证据：{{ detailEvidence(claim.evidence_ids)?.map((ev) => evidenceKindLabels[ev.kind] + ': ' + ev.content).join(' | ') }}
            </p>
          </article>
        </div>
      </section>
    </template>
  </section>
  <section v-else class="page">
    <ApiErrorState :message="error || '加载中'" />
  </section>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Ban, ChevronDown, Copy, Download, FileText, Pause, Pen, Play, RefreshCw, RotateCw } from '@lucide/vue'
import { api } from '../api'
import ApiErrorState from '../components/ApiErrorState.vue'
import CritiqueGraph from '../components/CritiqueGraph.vue'
import DecisionSummary from '../components/DecisionSummary.vue'
import IdeaCard from '../components/IdeaCard.vue'
import PhaseProgressBar from '../components/PhaseProgressBar.vue'
import ProposalCompare from '../components/ProposalCompare.vue'
import SeatRoleCard from '../components/SeatRoleCard.vue'
import VoteChanges from '../components/VoteChanges.vue'
import VoteDisplay from '../components/VoteDisplay.vue'
import { hasStoredViewMode, useViewMode } from '../composables/useViewMode'
import { evidenceSafetyLabels, evidenceSourceKindLabels, evidenceTrustLabels, exportSessionMarkdown, ideaStatusLabels, evidenceKindLabels, modeLabels, phaseLabels, qualityMetricRows, revisionDiffs, seatLabels, seatRunStats, voteStrategyLabels, type SeatKind, type SessionDetails } from '../domain/session'

const route = useRoute()
const router = useRouter()
const { viewMode, setViewMode } = useViewMode({ route, router })
const id = computed(() => String(route.params.id))
const seats: SeatKind[] = ['mouyuan', 'jingshi', 'chizheng']
const details = ref<SessionDetails | null>(null)
const error = ref('')
const editingContext = ref(false)
const newContext = ref('')
const showTrajectory = ref(false)
const showExportMenu = ref(false)
const showMdMenu = ref(false)
const trajectory = ref<Array<{ id: number; event_type: string; created_at: string }>>([])
const canManualRevision = computed(
  () =>
    Boolean(details.value) &&
    !details.value?.execution.running &&
    (details.value?.session.phase === 'independent_deliberation' || details.value?.session.phase === 'cross_critique'),
)
const canCopyDecision = computed(() => {
  const decision = details.value?.session.result ?? details.value?.artifacts.decision
  return Boolean(decision?.selected_proposal)
})
const recentFailedRuns = computed(() =>
  (details.value?.artifacts.seat_runs ?? [])
    .filter((run) => run.status === 'failed')
    .slice(-3)
    .reverse(),
)
const hasTokenUsage = computed(() => (details.value?.artifacts.seat_runs ?? []).some((run) => typeof run.total_tokens === 'number'))
const primaryDecision = computed(() => details.value?.session.result ?? details.value?.artifacts.decision ?? null)
const timelineEvents = computed(() => [...(details.value?.events ?? [])].reverse())
const trajectoryEvents = computed(() => [...trajectory.value].reverse())
const externalEvidence = computed(() =>
  (details.value?.artifacts.evidence ?? []).filter((ev) => ev.source_kind && ev.source_kind !== 'internal'),
)
const toolRuns = computed(() => details.value?.artifacts.tool_runs ?? [])
let source: EventSource | null = null
let timer: number | undefined

function seatProviderRef(seat: SeatKind) {
  return details.value?.seats?.find((item) => item.seat === seat)?.provider_ref ?? ''
}

function detailEvidence(evidenceIds?: string[]) {
  if (!evidenceIds?.length || !details.value?.artifacts.evidence?.length) return []
  return details.value.artifacts.evidence.filter((ev) => evidenceIds.includes(ev.id))
}

function eventBadge(type: string) {
  if (type.includes('completed') || type.includes('majority')) return 'ok'
  if (type.includes('failed') || type.includes('error') || type.includes('cancelled')) return 'warn'
  return ''
}

function compactSource(sourceText: string) {
  if (!sourceText) return '未记录来源'
  try {
    const url = new URL(sourceText)
    return `${url.hostname}${url.pathname === '/' ? '' : url.pathname}`
  } catch {
    return sourceText.replace(/^file:\/\//, '')
  }
}

function toolNameLabel(name: string) {
  const labels: Record<string, string> = {
    web_search: '网页搜索',
    document_parse: '文档解析',
    code_search: '代码搜索',
  }
  return labels[name] ?? name
}

function downloadAndClose(format: 'json' | 'html') {
  showExportMenu.value = false
  if (format === 'json') {
    downloadJSON()
  } else {
    downloadHTML()
  }
}

function copyDecision() {
  if (!details.value) return
  const decision = details.value.session.result ?? details.value.artifacts.decision
  if (!decision?.selected_proposal) return
  const text = `【${details.value.session.title}】\n\n${decision.selected_proposal.title}\n${decision.selected_proposal.summary}`
  navigator.clipboard.writeText(text).catch(() => {})
}

function exportJSON() {
  if (!details.value) return null
  return {
    title: details.value.session.title,
    topic: details.value.session.topic,
    phase: details.value.session.phase,
    session_id: details.value.session.id,
    decision: details.value.session.result ?? details.value.artifacts.decision,
    ideas: details.value.artifacts.ideas,
    proposals: details.value.artifacts.proposals,
    critiques: details.value.artifacts.critiques,
    votes: details.value.artifacts.votes,
    quality: details.value.artifacts.quality,
    claims: details.value.artifacts.claims,
    evidence: details.value.artifacts.evidence,
  }
}

function downloadJSON() {
  if (!details.value) return
  const data = exportJSON()
  if (!data) return
  const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = `${safeFilename(details.value.session.title)}.json`
  link.click()
  URL.revokeObjectURL(url)
}

function downloadHTML() {
  if (!details.value) return
  const html = generateHTML(details.value)
  const blob = new Blob([html], { type: 'text/html;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = `${safeFilename(details.value.session.title)}.html`
  link.click()
  URL.revokeObjectURL(url)
}

function generateHTML(details: SessionDetails): string {
  const decision = details.session.result ?? details.artifacts.decision
  return `<!DOCTYPE html>
<html lang="zh-CN">
<head><meta charset="utf-8"><title>${escapeHTML(details.session.title)} — 文渊阁</title>
<style>
  body { font-family: Inter, "Noto Sans SC", system-ui, sans-serif; background: #f7f4ed; color: #20231f; max-width: 900px; margin: 0 auto; padding: 32px; line-height: 1.7; }
  h1 { font-family: "Noto Serif SC", serif; font-size: 28px; }
  h2 { font-family: "Noto Serif SC", serif; font-size: 20px; margin-top: 32px; border-bottom: 1px solid #ded5c5; padding-bottom: 8px; }
  h3 { font-family: "Noto Serif SC", serif; font-size: 16px; }
  .meta { color: #6d6a61; font-size: 14px; }
  .section { background: #fffdf8; border: 1px solid #ded5c5; padding: 16px; margin: 12px 0; border-radius: 8px; }
  ul { padding-left: 20px; }
  li { margin: 4px 0; }
  .badge { display: inline-block; border: 1px solid #c9c0b2; padding: 2px 8px; border-radius: 4px; font-size: 12px; }
</style></head>
<body>
  <h1>${escapeHTML(details.session.title)}</h1>
  <p class="meta">${phaseLabels[details.session.phase]} · ${details.session.id}</p>
  <h2>议题</h2>
  <div class="section"><p>${escapeHTML(details.session.topic)}</p></div>
  ${details.artifacts.ideas.length ? `<h2>创意池（${details.artifacts.ideas.length}）</h2>
  ${details.artifacts.ideas.map(i => `<div class="section"><h3>${escapeHTML(i.title)}</h3><p>${escapeHTML(i.summary)}</p></div>`).join('')}` : ''}
  ${details.artifacts.proposals.length ? `<h2>策案对比</h2>
  ${details.artifacts.proposals.map(p => `<div class="section"><h3>${escapeHTML(p.title)}</h3><p>${escapeHTML(p.summary)}</p></div>`).join('')}` : ''}
  ${decision ? `<h2>表决结果</h2><div class="section"><p>${decision.status === 'majority_reached' ? '形成多数' : decision.status === 'conditionally_adopted' ? '有条件通过' : '未形成多数'}${decision.has_risk_blocker ? ' ⚠️存在风险阻塞' : ''}</p></div>` : ''}
</body></html>`
}

function escapeHTML(s: string) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
}

async function load() {
  try {
    details.value = await api.getSession(id.value)
  } catch (err) {
    error.value = err instanceof Error ? err.message : '加载失败'
  }
}

async function saveContext() {
  if (!details.value) return
  try {
    details.value = await api.updateContext(id.value, newContext.value)
    editingContext.value = false
  } catch (err) {
    error.value = err instanceof Error ? err.message : '保存失败'
  }
}

async function pause() {
  try {
    details.value = await api.pauseSession(id.value)
  } catch (err) {
    error.value = err instanceof Error ? err.message : '暂停失败'
  }
}

async function resume() {
  try {
    details.value = await api.resumeSession(id.value)
    timer = window.setTimeout(load, 500)
  } catch (err) {
    error.value = err instanceof Error ? err.message : '继续失败'
  }
}

async function retry() {
  details.value = await api.retrySession(id.value)
  timer = window.setTimeout(load, 500)
}

async function retryCurrentPhase() {
  try {
    details.value = await api.retryPhase(id.value)
  } catch (err) {
    error.value = err instanceof Error ? err.message : '重试阶段失败'
  }
}

async function cancel() {
  details.value = await api.cancelSession(id.value)
}

async function manualRevision() {
  try {
    details.value = await api.manualRevision(id.value)
  } catch (err) {
    error.value = err instanceof Error ? err.message : '触发失败'
  }
}

async function retrySeat(seat: string) {
  try {
    details.value = await api.retrySeat(id.value, seat)
  } catch (err) {
    error.value = err instanceof Error ? err.message : '重试席位失败'
  }
}

async function loadTrajectory() {
  showTrajectory.value = true
  try {
    trajectory.value = await api.phaseTrajectory(id.value)
  } catch (err) {
    error.value = err instanceof Error ? err.message : '加载轨迹失败'
  }
}

function downloadMarkdown(level: 'brief' | 'standard' | 'audit') {
  if (!details.value) return
  showMdMenu.value = false
  const name = level === 'brief' ? '简报' : level === 'standard' ? '完整报告' : '审计全文'
  const markdown = exportSessionMarkdown(details.value, level)
  const blob = new Blob([markdown], { type: 'text/markdown;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = `${safeFilename(details.value.session.title)}-${name}.md`
  link.click()
  URL.revokeObjectURL(url)
}

function safeFilename(value: string) {
  return value.trim().replace(/[\\/:*?"<>|]+/g, '-').replace(/\s+/g, '-').slice(0, 80) || 'wenyuan-session'
}

async function applyDefaultViewPreference() {
  if (route.query.view) return
  const storage = typeof window === 'undefined' ? null : window.localStorage
  if (hasStoredViewMode(storage)) return
  try {
    const preferences = await api.preferences()
    if (preferences.defaults.view_mode === 'report') {
      setViewMode('report')
    }
  } catch {
    // Preferences are a convenience layer; session loading should not depend on them.
  }
}

onMounted(async () => {
  await applyDefaultViewPreference()
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

<style scoped>
.scribe-report h3 {
  margin-top: 16px;
  margin-bottom: 6px;
  font-size: 14px;
  color: var(--color-text-muted);
}
.scribe-report ul {
  padding-left: 20px;
  margin-bottom: 12px;
}
.scribe-report details {
  margin-top: 12px;
}
.scribe-report details summary {
  cursor: pointer;
  font-weight: 600;
}
.scribe-final-report {
  margin-top: 8px;
  padding: 12px;
  background: var(--color-bg-subtle);
  border-radius: var(--radius-sm);
  white-space: pre-wrap;
  font-size: 14px;
  line-height: 1.7;
}

.evidence-source-panel .row-head {
  margin-bottom: var(--space-md);
}

.evidence-source-panel h2 {
  margin-bottom: 0;
}

.evidence-source-grid {
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
}

.evidence-source-item {
  display: grid;
  gap: 8px;
}

.evidence-source-item p {
  display: -webkit-box;
  -webkit-line-clamp: 4;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.evidence-source-url {
  font-family: var(--font-mono);
  font-size: 12px !important;
  word-break: break-all;
}

.evidence-safety-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.tool-run-panel .row-head {
  margin-bottom: var(--space-md);
}

.tool-run-panel h2 {
  margin-bottom: 0;
}

.tool-run-grid {
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
}

.tool-run-item {
  display: grid;
  gap: 8px;
}

.tool-run-error {
  color: var(--color-danger) !important;
}
</style>
