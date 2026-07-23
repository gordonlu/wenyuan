<template>
  <section v-if="details" :class="['page', 'workspace', { 'report-mode': viewMode === 'report' }]">
    <header class="page-head row-head edict">
      <div class="edict-head">
        <span :class="['seal-stamp', 'edict-seal', sealKind]">{{ sealChar }}</span>
        <div>
          <p class="phase-label">{{ phaseLabels[details.session.phase] }}</p>
          <h1>{{ details.session.title }}</h1>
          <div class="title-tag-row" style="margin-top: 4px">
            <span v-if="details.artifacts.topic_type" class="badge flat topic-tag">{{ topicTypeLabel(details.artifacts.topic_type) }}</span>
            <span class="badge flat">{{ modeLabels[details.session.mode] }}</span>
            <span v-if="details.session.vote_policy && details.session.mode !== 'single_agent'" class="badge flat" style="margin-left: 6px">
              {{ voteStrategyLabels[details.session.vote_policy.strategy] }}
            </span>
          </div>
        </div>
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
        <button title="分享审议结果" @click="showShare = true">
          <Share2 :size="18" />
        </button>
        <div class="menu-wrap">
          <button title="导出 Markdown" @click="showMdMenu = !showMdMenu">
            <FileText :size="18" />
            Markdown
            <ChevronDown :size="16" />
          </button>
          <div v-if="showMdMenu" class="action-menu">
            <button @click="downloadMarkdown('brief')">普通报告</button>
            <button @click="downloadMarkdown('standard')">深度研究报告</button>
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
            <RotateCw :size="14" />
            重试阶段
          </button>
          <button class="retry-session" title="重试整个议题" @click="retry">
            <RotateCw :size="18" />
            重新开议
          </button>
          <button class="icon danger" title="删除" @click="removeSession">
            <Ban :size="18" />
          </button>
        </template>
      </div>
    </header>

    <template v-if="viewMode === 'workbench'">
    <WorkspaceTabs :tab="currentTab" @update:tab="setTab" />
    <div class="workspace-main">
      <WorkspaceOverviewTab
        v-if="currentTab === 'overview'"
        :phase="details.session.phase"
        :running="details.execution.running"
        :events="details.events"
        :seat-list="seatSlotList"
        :error-message="error"
        :failure-reason="details.session.failure_reason ?? undefined"
        :failed-runs="failedRunSlots"
        :retry-required="details.execution.recovery_state === 'retry_required'"
        :paused="details.execution.recovery_state === 'paused'"
        :phase-label="phaseLabels[details.session.phase]"
        :running-seat-label="runningSeatLabel"
        :running-activity-label="runningActivityLabel"
        :last-event-time="lastEventTime"
        :current-digest="currentDigest"
        :evidence-summary="currentEvidenceSummary"
        :donut-data="donutData"
        :radar-data="radarData"
        :quality-metrics="qualityMetricRows(details.artifacts.quality, hasTokenUsage)"
        :primary-decision="primaryDecision"
        :vote-policy="details.session.vote_policy"
        :mode="details.session.mode"
        :recent-events="recentFiveEvents"
      />
      <WorkspaceProcessTab
        v-else-if="currentTab === 'process'"
        :topic-html="renderReportText(details.session.topic)"
        :context-html="renderReportText(details.session.context)"
        :editing-context="editingContext"
        :edit-context-text="newContext"
        :ideas="details.artifacts.ideas"
        :critiques="details.artifacts.critiques"
        :proposals="details.artifacts.proposals"
        :revision-diffs="revisionDiffs(details)"
        :selected-proposal-id="selectedProposalId"
        @start-edit-context="editingContext = true"
        @save-context="saveContext"
        @cancel-edit-context="editingContext = false"
      />
      <WorkspaceEvidenceTab
        v-else-if="currentTab === 'evidence'"
        :evidence-summary="currentEvidenceSummary"
        :donut-data="donutData"
        :radar-data="radarData"
        :quality-metrics="qualityMetricRows(details.artifacts.quality, hasTokenUsage)"
        :external-evidence="externalEvidenceSlots"
        :tool-runs="toolRunSlots"
        :tool-run-summary="toolRunSummaryData.by_tool"
        :tool-run-duration="(toolRunSummaryData.total_ms / 1000).toFixed(1)"
        :tool-run-failed="toolRunSummaryData.failed"
        :supported-claims="claimSlots(true)"
        :unsupported-claims="claimSlots(false)"
      />
      <WorkspaceDecisionTab
        v-else-if="currentTab === 'decision'"
        :primary-decision="primaryDecision"
        :vote-policy="details.session.vote_policy"
        :mode="details.session.mode"
        :votes="details.artifacts.votes"
        :proposals="details.artifacts.proposals"
        :has-decision-objects="hasDecisionObjects"
        :decision-objects="decisionObjects"
        @resolve-object="handleResolveObject"
        @dismiss-object="handleDismissObject"
      />
      <WorkspaceFollowUpTab
        v-else-if="currentTab === 'followup'"
        :suggestions="followupSuggestions"
        :loading="followupsLoading"
        :turns="followupTurns"
        :objects="decisionObjects"
        :re-delib-running="reDelibRunning"
        :re-delib-result="reDelibResult"
        :re-delib-error="reDelibError"
        @regenerate="handleRegenerateFollowups"
        @start-followup="handleStartFollowup"
        @re-deliberate="handleReDeliberate"
        @clear-re-delib-error="reDelibError = ''"
      />
      <WorkspaceAuditTab
        v-else-if="currentTab === 'audit'"
        :failed-runs="failedRunSlots"
        :seat-runs="details.artifacts.seat_runs"
        :seat-stats="auditSeatStats"
        :has-token-usage="hasTokenUsage"
        :show-trajectory="showTrajectory"
        :timeline-events="auditTimelineSlots"
        :trajectory-events="trajectorySlotList"
        :scribe-report="auditScribeReport"
        @load-trajectory="loadTrajectory"
      />
    </div>
    </template>

    <ReportView
      v-if="viewMode === 'report' && details"
      :details="details"
      :scribe-mode="scribeMode"
      :external-evidence="externalEvidence"
      :tool-runs="toolRuns"
      :supported-claims="supportedClaims"
      :unsupported-claims="unsupportedClaims"
    />
  </section>

  <ShareExportPanel
    v-if="showShare && shareDigest"
    :visible="showShare"
    :digest="shareDigest"
    :title="details?.session.title ?? ''"
    :seat-summary="seatSummary"
    :evidence-total="shareDigest.evidence_total"
    :untrusted-count="shareDigest.untrusted_count"
    :vote-count="shareDigest.vote_count"
    @close="showShare = false"
  />

  <section v-else class="page">
    <p v-if="error" class="error-state">{{ error }}</p>
    <p v-else-if="loading" class="loading-state">加载中…</p>
  </section>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Ban, ChevronDown, ChevronUp, Copy, Download, FileText, Pause, Pen, Play, RefreshCw, RotateCw, Share2 } from '@lucide/vue'
import { api } from '../api'
import ReportView from '../components/ReportView.vue'
import ShareExportPanel from '../components/ShareExportPanel.vue'
import WorkspaceTabs from '../components/workspace/WorkspaceTabs.vue'
import WorkspaceOverviewTab from '../components/workspace/WorkspaceOverviewTab.vue'
import WorkspaceProcessTab from '../components/workspace/WorkspaceProcessTab.vue'
import WorkspaceEvidenceTab from '../components/workspace/WorkspaceEvidenceTab.vue'
import WorkspaceDecisionTab from '../components/workspace/WorkspaceDecisionTab.vue'
import WorkspaceFollowUpTab from '../components/workspace/WorkspaceFollowUpTab.vue'
import WorkspaceAuditTab from '../components/workspace/WorkspaceAuditTab.vue'
import { hasStoredViewMode, useViewMode } from '../composables/useViewMode'
import { useConfirm } from '../composables/useConfirm'
import { cleanReportText, decisionDigest, evidenceSafetyLabels, evidenceSourceKindLabels, evidenceSummary, evidenceTrustLabels, exportSessionMarkdown, followUpImpactLabels, followUpKindLabels, ideaStatusLabels, evidenceKindLabels, modeLabels, phaseLabels, qualityMetricRows, renderReportText, revisionDiffs, seatLabels, seatRunStats, toolNameLabel, toolRunSummary, voteStrategyLabels, type DecisionObject, type FollowUpSuggestion, type FollowUpTurn, type SeatKind, type SessionDetails } from '../domain/session'
import { evidenceDonutSegments, qualityRadarAxes } from '../utils/chart-data'

const route = useRoute()
const router = useRouter()
const { viewMode, setViewMode } = useViewMode({ route, router })
const { confirm } = useConfirm()
const id = computed(() => String(route.params.id))
const seats: SeatKind[] = ['mouyuan', 'jingshi', 'chizheng']
const details = ref<SessionDetails | null>(null)
const loading = ref(true)
const error = ref('')
const editingContext = ref(false)
const newContext = ref('')
const showTrajectory = ref(false)
const showToolDetail = ref(false)
const showExportMenu = ref(false)
const showMdMenu = ref(false)
const showShare = ref(false)

// Follow-up / 续议
const decisionObjects = ref<DecisionObject[]>([])
const followupSuggestions = ref<FollowUpSuggestion[]>([])
const followupTurns = ref<FollowUpTurn[]>([])
const followupsLoading = ref(false)
const reDelibRunning = ref(false)
const reDelibResult = ref<unknown>(null)
const reDelibError = ref('')

const hasDecisionObjects = computed(() => decisionObjects.value.length > 0)
const hasFollowups = computed(() => followupSuggestions.value.length > 0)
const hasFollowupTurns = computed(() => followupTurns.value.length > 0)

// 议题卷轴印章状态
const sealChar = computed(() => {
  const phase = details.value?.session.phase
  if (phase === 'completed') return '决'
  if (phase === 'failed' || phase === 'cancelled') return '废'
  if (phase === 'draft') return '启'
  return '议'
})
const sealKind = computed(() => {
  const phase = details.value?.session.phase
  if (phase === 'completed') return ''
  if (phase === 'failed' || phase === 'cancelled') return 'ash'
  return 'jade'
})

const shareDigest = computed(() => {
  if (!details.value) return null
  const deets = details.value
  return {
    title: deets.session.title,
    status_label: currentDigest.value?.status_label ?? '尚无结论',
    status_class: currentDigest.value?.status_class ?? 'warn',
    selected_proposal_title: currentDigest.value?.selected_proposal_title ?? '',
    selected_proposal_summary: currentDigest.value?.selected_proposal_summary ?? '',
    majority_summary: currentDigest.value?.majority_reason_summary ?? '',
    risk_summary: currentDigest.value?.has_risk_blocker ? '存在风险阻塞，需先处理采纳条件' : '',
    evidence_total: currentEvidenceSummary.value?.total ?? 0,
    untrusted_count: currentEvidenceSummary.value?.untrusted_count ?? 0,
    vote_count: currentDigest.value?.vote_count ?? 0,
    seat_count: deets.seats?.length ?? 0,
  }
})

const seatSummary = computed(() => {
  if (!details.value) return ''
  return details.value.seats?.map((s) => seatLabels[s.seat]).join(' · ') ?? ''
})
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
const scribeMode = computed(() => details.value?.session.scribe_enabled ? 'full' : 'light')
const primaryDecision = computed(() => details.value?.session.result ?? details.value?.artifacts.decision ?? null)
const selectedProposalId = computed(() => primaryDecision.value?.selected_proposal?.id ?? '')
const currentEvidenceSummary = computed(() => details.value ? evidenceSummary(details.value) : null)

const donutData = computed(() => evidenceDonutSegments(currentEvidenceSummary.value))
const radarData = computed(() => qualityRadarAxes(details.value?.artifacts.quality))
const currentDigest = computed(() => {
  if (!details.value) return null
  const evSum = currentEvidenceSummary.value ?? undefined
  return decisionDigest(details.value, evSum)
})
const timelineEvents = computed(() => {
  const items: Array<{ id: string; event_type: string; payload: unknown; created_at: string }> = []
  for (const event of details.value?.events ?? []) {
    items.push({ id: `evt-${event.id}`, event_type: event.event_type, payload: event.payload, created_at: event.created_at })
  }
  for (const text of details.value?.artifacts.events ?? []) {
    items.push({ id: `art-${text}`, event_type: text, payload: null, created_at: '' })
  }
  return items.reverse()
})

const seatEvents = computed(() => (details.value?.events ?? []).filter((e) =>
  ['seat_started', 'seat_completed', 'seat_failed'].includes(e.event_type)
))
const runningSeatLabel = computed(() => {
  if (!details.value?.execution.running) return ''
  const events = seatEvents.value
  const latest = events[events.length - 1]
  if (latest?.event_type === 'seat_started') {
    const seat = (latest.payload as { seat?: SeatKind })?.seat
    if (seat) return `${seatLabels[seat]}工作中`
  }
  if (details.value.session.phase === 'independent_deliberation') return '三席独立陈策'
  if (details.value.session.phase === 'cross_critique') return '三席交叉批议'
  if (details.value.session.phase === 'revision') return '三席修订策案'
  if (details.value.session.phase === 'voting') return '三席阁议投票'
  if (details.value.session.phase === 'convergence') return '合案复议'
  return '执行中'
})
const runningActivityLabel = computed(() => {
  if (!details.value?.execution.running) return ''
  const latest = [...(details.value.events ?? [])]
    .reverse()
    .find((event) => ['tool_started', 'tool_completed', 'tool_failed', 'seat_started', 'seat_completed', 'seat_failed'].includes(event.event_type))
  if (!latest) return ''
  if (latest.event_type === 'seat_started') return '模型调用中'
  if (latest.event_type === 'seat_completed') return '模型返回'
  if (latest.event_type === 'seat_failed') return '模型调用失败'
  return eventLabel(latest)
})
const lastEventTime = computed(() => {
  void tick.value
  const events = details.value?.events ?? []
  if (!events.length) return ''
  const last = events[events.length - 1]
  try {
    const d = new Date(last.created_at)
    const now = new Date()
    const diffSec = Math.floor((now.getTime() - d.getTime()) / 1000)
    if (diffSec < 60) return `${diffSec}秒前`
    if (diffSec < 3600) return `${Math.floor(diffSec / 60)}分钟前`
    return d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
  } catch { return '' }
})
const trajectoryEvents = computed(() => [...trajectory.value].reverse())
const externalEvidence = computed(() =>
  (details.value?.artifacts.evidence ?? []).filter((ev) => ev.source_kind && ev.source_kind !== 'internal'),
)
const supportedClaims = computed(() =>
  (details.value?.artifacts.claims ?? []).filter((c) => c.is_supported),
)
const unsupportedClaims = computed(() =>
  (details.value?.artifacts.claims ?? []).filter((c) => !c.is_supported),
)
const toolRuns = computed(() => details.value?.artifacts.tool_runs ?? [])
const toolRunSummaryData = computed(() => toolRunSummary(toolRuns.value))

// ── Tab state ──
const currentTab = computed(() => {
  const tab = route.query.tab
  const validTabs = ['overview', 'process', 'evidence', 'decision', 'followup', 'audit']
  return typeof tab === 'string' && validTabs.includes(tab) ? tab : 'overview'
})

function setTab(tab: string) {
  router.replace({ query: { ...route.query, tab } })
}

// ── Tab slot data ──
const seatSlotList = computed(() => seats.map((seat) => ({
  seat,
  runs: details.value!.artifacts.seat_runs,
  toolRuns: toolRuns.value,
  providerRef: seatProviderRef(seat),
  inactive: details.value!.session.mode === 'single_agent' && seat !== 'mouyuan',
})))

const failedRunSlots = computed(() => recentFailedRuns.value.map((r) => ({
  id: r.id,
  seat: r.seat,
  seatLabel: seatLabels[r.seat],
  phase: r.phase,
  phaseLabel: phaseLabels[r.phase],
  error: r.error ?? undefined,
})))

const recentFiveEvents = computed(() => {
  const evs = details.value?.events ?? []
  return evs.slice(-5).reverse().map((e) => ({
    id: (e as any).id ?? '',
    time: (e as any).created_at ? new Date((e as any).created_at).toLocaleString() : '',
    label: eventLabel(e),
    badgeClass: eventBadge(e.event_type),
  }))
})

const externalEvidenceSlots = computed(() => externalEvidence.value.slice(0, 12).map((ev) => ({
  id: ev.id,
  kindLabel: evidenceSourceKindLabels[ev.source_kind ?? 'internal'] ?? ev.source_kind,
  trustClass: ev.trust_level === 'untrusted_external' ? 'warn' : 'ok',
  trustLabel: evidenceTrustLabels[ev.trust_level ?? 'internal'] ?? ev.trust_level,
  contentHtml: renderReportText(ev.content),
  source: compactSource(ev.source),
  safetyLabels: evidenceSafetyLabels(ev.safety_flags),
})))

const toolRunSlots = computed(() => toolRuns.value.map((r) => ({
  id: r.id,
  toolName: toolNameLabel(r.tool_name),
  status: r.status,
  summary: reportText(r.input_summary),
  duration: (r.duration_ms / 1000).toFixed(1),
  evidenceCount: r.evidence_ids?.length ?? 0,
  error: r.error ?? undefined,
})))

function claimSlots(supported: boolean) {
  const claims = supported ? supportedClaims.value : unsupportedClaims.value
  return claims.map((c) => ({
    id: c.id,
    seat: c.proposed_by,
    seatLabel: seatLabels[c.proposed_by] ?? c.proposed_by,
    contentHtml: renderReportText(c.content),
    contextText: reportText(c.context),
    evidenceDetail: detailEvidence(c.evidence_ids)
      ?.map((ev) => evidenceKindLabels[ev.kind] + ': ' + reportText(ev.content)).join(' | '),
  }))
}

const auditSeatStats = computed(() => seatRunStats(details.value!.artifacts.seat_runs).map((s) => ({
  seat: s.seat,
  seatLabel: seatLabels[s.seat] ?? s.seat,
  calls: s.calls,
  duration: s.durationMs ? (s.durationMs / 1000).toFixed(1) + ' 秒' : '0 秒',
  failed: s.failed,
  repaired: s.repaired,
  hasUsage: s.hasUsage,
  tokens: s.hasUsage ? String(s.tokens) : undefined,
  promptVersions: s.promptVersions,
})))

const auditTimelineSlots = computed(() => timelineEvents.value.map((e) => ({
  id: (e as any).id ?? '',
  time: (e as any).created_at ? new Date((e as any).created_at).toLocaleString() : undefined,
  label: eventLabel(e),
  badgeClass: eventBadge(e.event_type),
})))

const trajectorySlotList = computed(() => trajectoryEvents.value.map((e) => ({
  id: (e as any).id ?? '',
  time: (e as any).created_at ? new Date((e as any).created_at).toLocaleString() : undefined,
  label: (e as any).event_type ?? '',
})))

const auditScribeReport = computed(() => {
  const report = details.value?.artifacts.scribe_report
  if (!report) return null
  return {
    consensusHtml: renderReportText(report.consensus_summary),
    gaps: report.structural_gaps,
    conflicts: report.unresolved_conflicts,
    finalHtml: renderReportText(report.final_report),
  }
})

// Tick every 3s so lastEventTime refreshes even without new SSE events
const tick = ref(0)
let tickTimer: number | undefined
onMounted(() => { tickTimer = window.setInterval(() => { tick.value++ }, 3000) })
onBeforeUnmount(() => { if (tickTimer) window.clearInterval(tickTimer) })

let source: EventSource | null = null
let timer: number | undefined
let pollTimer: number | undefined

function seatProviderRef(seat: SeatKind) {
  return details.value?.seats?.find((item) => item.seat === seat)?.provider_ref ?? ''
}

function detailEvidence(evidenceIds?: string[]) {
  if (!evidenceIds?.length || !details.value?.artifacts.evidence?.length) return []
  return details.value.artifacts.evidence.filter((ev) => evidenceIds.includes(ev.id))
}

function reportText(value?: string | null) {
  return cleanReportText(value)
}

function eventBadge(type: string) {
  if (type.includes('completed') || type.includes('majority')) return 'ok'
  if (type.includes('failed') || type.includes('error') || type.includes('cancelled')) return 'warn'
  return ''
}

const seatEventLabels: Record<string, string> = {
  seat_started: '开始',
  seat_completed: '完成',
  seat_failed: '失败',
}

function topicTypeLabel(key: string) {
  const labels: Record<string, string> = {
    PersonalLife: '生活决策',
    Consumer: '消费决策',
    Legal: '法律问题',
    Academic: '学术问题',
    Medical: '医疗健康',
    Financial: '财务投资',
    Technical: '技术产品',
    Product: '产品战略',
    Strategy: '企业战略',
  }
  return labels[key] || key
}

function eventLabel(event: { event_type: string; payload: unknown }) {
  const payload = (typeof event.payload === 'object' && event.payload !== null) ? event.payload as Record<string, unknown> : {}
  const seat = typeof payload.seat === 'string' ? payload.seat : undefined
  const query = typeof payload.query === 'string' ? payload.query : undefined
  const toolName = typeof payload.tool_name === 'string' ? payload.tool_name : undefined
  const count = typeof payload.count === 'number' ? payload.count : undefined
  const error = typeof payload.error === 'string' ? payload.error : undefined

  if (event.event_type.startsWith('tool_')) {
    const actor = seat ? `${seatLabels[seat as SeatKind] || seat}` : ''
    const toolLabel = toolActionLabel(toolName)
    const queryText = query ? `：${query}` : ''
    if (event.event_type === 'tool_started') return `${actor}执行了${toolLabel}${queryText}`
    if (event.event_type === 'tool_completed') return `${actor}完成${toolLabel}${queryText}${typeof count === 'number' ? `（${count} 条）` : ''}`
    if (event.event_type === 'tool_failed') return `${actor}${toolLabel}失败${queryText}${error ? `（${error}）` : ''}`
  }

  const label = seatEventLabels[event.event_type] || artifactEventLabel(event.event_type) || event.event_type
  let result = label
  if (query) {
    result += ` 查询：${query}`
  }
  if (seat) {
    result = `${result} · ${seatLabels[seat as SeatKind] || seat}`
  }
  return result
}

function artifactEventLabel(type: string): string | undefined {
  if (type.startsWith('search_completed')) return '搜索完成'
  if (type.startsWith('search_failed')) return '搜索失败'
  if (type.startsWith('scribe_completed')) return '书记官完成'
  if (type.startsWith('scribe_failed')) {
    const err = type.slice('scribe_failed'.length).trim().replace(/^:/, '').trim()
    return err ? `书记官失败：${err}` : '书记官失败'
  }
  return undefined
}

function compactSource(sourceText: string) {
  if (!sourceText) return '未记录来源'
  if (sourceText.startsWith('file://')) {
    return compactPath(sourceText.replace(/^file:\/\//, '').split('#')[0])
  }
  if (sourceText.startsWith('code://')) {
    return sourceText.replace(/^code:\/\//, '')
  }
  try {
    const url = new URL(sourceText)
    return `${url.hostname}${url.pathname === '/' ? '' : url.pathname}`
  } catch {
    return compactPath(sourceText)
  }
}

function compactPath(value: string) {
  const cleaned = value.replace(/\\/g, '/').replace(/\/+$/g, '')
  return cleaned.split('/').filter(Boolean).pop() || '本地来源'
}

function toolActionLabel(name?: string) {
  if (name === 'web_search') return '搜索'
  return name ? toolNameLabel(name) : '工具'
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
    decision_objects: decisionObjects.value,
    followup_turns: followupTurns.value,
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
  const evSum = evidenceSummary(details)
  const digest = decisionDigest(details, evSum)
  return `<!DOCTYPE html>
<html lang="zh-CN">
<head><meta charset="utf-8"><title>${escapeHTML(reportText(details.session.title) || details.session.title)} — 文渊阁</title>
<style>
  body { font-family: Inter, "Noto Sans SC", system-ui, sans-serif; background: #f7f4ed; color: #20231f; max-width: 900px; margin: 0 auto; padding: 32px; line-height: 1.7; }
  h1 { font-family: "Noto Serif SC", serif; font-size: 28px; margin-bottom: 4px; }
  h2 { font-family: "Noto Serif SC", serif; font-size: 20px; margin-top: 32px; border-bottom: 1px solid #ded5c5; padding-bottom: 8px; }
  h3 { font-family: "Noto Serif SC", serif; font-size: 16px; }
  .meta { color: #6d6a61; font-size: 14px; }
  .cover-meta { color: #6d6a61; font-size: 13px; margin-bottom: 24px; }
  .section { background: #fffdf8; border: 1px solid #ded5c5; padding: 16px; margin: 12px 0; border-radius: 8px; }
  ul { padding-left: 20px; }
  li { margin: 4px 0; }
  .badge { display: inline-block; border: 1px solid #c9c0b2; padding: 2px 8px; border-radius: 4px; font-size: 12px; }
  .badge-ok { border-color: #2f5d50; background: #e8f5ee; color: #2f5d50; }
  .badge-warn { border-color: #d4b86a; background: #fff8e6; color: #6f5223; }
  .flag { display: inline-block; padding: 2px 8px; border-radius: 4px; font-size: 11px; margin-right: 6px; }
  .flag-warn { background: #fff8e6; color: #6f5223; border: 1px solid #d4b86a; }
  .flag-danger { background: #fdf0ee; color: #9a3f34; border: 1px solid rgba(154,63,52,0.2); }
</style></head>
<body>
  <h1>${escapeHTML(reportText(details.session.title) || details.session.title)}</h1>
  <p class="cover-meta">${phaseLabels[details.session.phase]} · ${evSum.total} 项来源 · ${digest.vote_count} 票</p>

  ${digest.has_decision ? `<p><span class="badge ${digest.status_class === 'ok' ? 'badge-ok' : 'badge-warn'}">${escapeHTML(digest.status_label)}</span>${digest.selected_proposal_title ? ` <strong>${escapeHTML(digest.selected_proposal_title)}</strong>` : ''}</p>` : ''}

  <div>${digest.has_risk_blocker ? '<span class="flag flag-warn">存在风险阻塞</span>' : ''}${digest.has_untrusted_external ? '<span class="flag flag-warn">含不可信外部来源</span>' : ''}${digest.has_injection_risk ? '<span class="flag flag-danger">检测到疑似注入</span>' : ''}</div>

  <h2>议题</h2>
  <div class="section"><p>${escapeHTML(reportText(details.session.topic))}</p></div>
  ${details.artifacts.ideas.length ? `<h2>创意池（${details.artifacts.ideas.length}）</h2>
  ${details.artifacts.ideas.map(i => `<div class="section"><h3>${escapeHTML(reportText(i.title))}</h3><p>${escapeHTML(reportText(i.summary))}</p></div>`).join('')}` : ''}
  ${details.artifacts.proposals.length ? `<h2>策案对比</h2>
  ${details.artifacts.proposals.map(p => `<div class="section"><h3>${escapeHTML(reportText(p.title))}</h3><p>${escapeHTML(reportText(p.summary))}</p></div>`).join('')}` : ''}
  ${decision ? `<h2>表决结果</h2><div class="section"><p>${decision.status === 'majority_reached' ? '形成多数' : decision.status === 'conditionally_adopted' ? '有条件通过' : '未形成多数'}${decision.has_risk_blocker ? ' ⚠️存在风险阻塞' : ''}</p></div>` : ''}
</body></html>`
}

function escapeHTML(s: string) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
}

async function load() {
  loading.value = true
  try {
    const data = await api.getSession(id.value)
    if (data) {
      details.value = data
      error.value = ''
      loadFollowupData()
    } else {
      error.value = '未找到合议记录'
    }
  } catch (err) {
    error.value = err instanceof Error ? err.message : '加载失败'
  } finally {
    loading.value = false
  }
}

async function saveContext(text?: string) {
  if (!details.value) return
  try {
    const ctx = text ?? newContext.value
    details.value = await api.updateContext(id.value, ctx)
    editingContext.value = false
    newContext.value = ''
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
  if (!(await confirm('确认重新开议？'))) return
  details.value = await api.retrySession(id.value)
  timer = window.setTimeout(load, 500)
}

async function retryCurrentPhase() {
  if (!(await confirm('确认重试当前阶段？'))) return
  try {
    details.value = await api.retryPhase(id.value)
  } catch (err) {
    error.value = err instanceof Error ? err.message : '重试阶段失败'
  }
}

async function removeSession() {
  if (!(await confirm('确认删除本次议题？'))) return
  await api.deleteSession(id.value)
  router.push('/history')
}

async function manualRevision() {
  if (!(await confirm('确认手动触发复议？'))) return
  try {
    details.value = await api.manualRevision(id.value)
  } catch (err) {
    error.value = err instanceof Error ? err.message : '触发失败'
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
  const name = level === 'brief' ? '普通报告' : level === 'standard' ? '深度研究报告' : '审计全文'
  const markdown = exportSessionMarkdown(details.value, level)
  const blob = new Blob([markdown], { type: 'text/markdown;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = `${safeFilename(details.value.session.title)}-${name}.md`
  link.click()
  URL.revokeObjectURL(url)
}

// ── Follow-up handlers ──

async function loadFollowupData() {
  if (!details.value || details.value.session.phase !== 'completed') return
  try {
    const [objResp, sugResp, turnResp] = await Promise.all([
      api.getDecisionObjects(id.value),
      api.getFollowups(id.value),
      api.getFollowupTurns(id.value),
    ])
    decisionObjects.value = objResp.objects
    followupSuggestions.value = sugResp.suggestions
    followupTurns.value = turnResp.turns
  } catch {
    // non-critical; follow-up data is supplementary
  }
}

async function handleRegenerateFollowups() {
  followupsLoading.value = true
  try {
    const resp = await api.regenerateFollowups(id.value)
    followupSuggestions.value = resp.suggestions
  } catch (err) {
    error.value = err instanceof Error ? err.message : '重新生成续议建议失败'
  } finally {
    followupsLoading.value = false
  }
}

async function handleStartFollowup(payload: { suggestion: FollowUpSuggestion; mode: string }) {
  try {
    const resp = await api.startFollowup(payload.suggestion.id, payload.mode)
    // Reload turns to include the new one
    const turnResp = await api.getFollowupTurns(id.value)
    followupTurns.value = turnResp.turns
  } catch (err) {
    error.value = err instanceof Error ? err.message : '启动续议失败'
  }
}

async function handleReDeliberate(payload: { new_fact: string; affected_object_ids: string[] }) {
  reDelibRunning.value = true
  reDelibResult.value = null
  reDelibError.value = ''
  try {
    const resp = await api.reDeliberate(id.value, payload.new_fact, payload.affected_object_ids)
    reDelibResult.value = resp.result
    // Reload decision objects (some may now be superseded)
    const objResp = await api.getDecisionObjects(id.value)
    decisionObjects.value = objResp.objects
    // Reload turns
    const turnResp = await api.getFollowupTurns(id.value)
    followupTurns.value = turnResp.turns
  } catch (err) {
    reDelibError.value = err instanceof Error ? err.message : '复议失败'
  } finally {
    reDelibRunning.value = false
  }
}

async function handleResolveObject(objectId: string) {
  try {
    await api.updateDecisionObjectStatus(objectId, 'resolved')
    const objResp = await api.getDecisionObjects(id.value)
    decisionObjects.value = objResp.objects
  } catch (err) {
    error.value = err instanceof Error ? err.message : '更新状态失败'
  }
}

async function handleDismissObject(objectId: string) {
  try {
    await api.updateDecisionObjectStatus(objectId, 'dismissed')
    const objResp = await api.getDecisionObjects(id.value)
    decisionObjects.value = objResp.objects
  } catch (err) {
    error.value = err instanceof Error ? err.message : '更新状态失败'
  }
}

function safeFilename(value: string) {
  return value.trim().replace(/[\\/:*?"<>|]+/g, '-').replace(/\s+/g, '-').slice(0, 80) || 'wenyuan-session'
}

function startFallbackPolling() {
  if (!pollTimer) {
    pollTimer = window.setInterval(load, 2500)
  }
}

function stopFallbackPolling() {
  if (pollTimer) {
    window.clearInterval(pollTimer)
    pollTimer = undefined
  }
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
  source.onmessage = () => {
    stopFallbackPolling()
    load()
  }
  source.onerror = () => startFallbackPolling()
})

onBeforeUnmount(() => {
  source?.close()
  if (timer) window.clearTimeout(timer)
  stopFallbackPolling()
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
  font-size: 14px;
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

/* ── Report cover ── */
.report-cover {
  padding: 32px 24px;
  margin-bottom: 16px;
  border-bottom: 1px solid var(--color-border-light);
}

.report-cover-title {
  margin: 0 0 8px;
  font-size: 28px;
  font-weight: 800;
  line-height: 1.25;
  color: var(--color-text);
}

.report-cover-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  font-size: 13px;
  color: var(--color-text-muted);
  margin-bottom: 16px;
}

.report-cover-decision {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.report-cover-proposal {
  font-size: 16px;
  font-weight: 700;
  color: var(--color-text);
}

.report-cover-flags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.report-flag {
  padding: 3px 10px;
  border-radius: var(--radius-sm);
  font-size: 11px;
  font-weight: 600;
}

.report-flag-warn {
  background: var(--color-warning-bg);
  color: var(--color-warning-text);
  border: 1px solid var(--color-warning-border);
}

.report-flag-danger {
  background: var(--color-danger-light);
  color: var(--color-danger);
  border: 1px solid rgba(154, 63, 52, 0.2);
}

.tool-run-summary {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
}

.tool-run-chip {
  padding: 3px 10px;
  border-radius: 12px;
  background: var(--color-accent-light);
  color: var(--color-accent-text);
  font-size: 12px;
  font-weight: 600;
}

.tool-run-meta {
  font-size: 12px;
  color: var(--color-text-dim);
  margin-left: auto;
}

/* ── Status bar ── */
.status-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 16px;
  border-radius: var(--radius-sm);
  font-size: 13px;
  line-height: 1.4;
}

.status-bar-warn {
  background: var(--color-warning-bg);
  border: 1px solid var(--color-warning-border);
  color: var(--color-warning-text);
}

.status-bar-icon {
  font-size: 15px;
  flex-shrink: 0;
}

.status-bar-live {
  background: var(--color-accent-light);
  border: 1px solid rgba(15, 138, 161, 0.25);
  color: var(--color-accent-text);
  margin-bottom: 10px;
}

.status-bar-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--color-accent);
  flex-shrink: 0;
  animation: status-dot-pulse 1.4s ease-in-out infinite;
}

@keyframes status-dot-pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.4; transform: scale(0.7); }
}

.status-bar-phase {
  font-weight: 700;
}

.status-bar-sep {
  color: var(--color-text-dim);
}

.status-bar-seat {
  color: var(--color-text);
}

.status-bar-tool {
  overflow: hidden;
  max-width: min(46vw, 520px);
  padding: 2px 8px;
  border: 1px solid rgba(47, 191, 212, 0.24);
  border-radius: 4px;
  background: rgba(47, 191, 212, 0.08);
  color: var(--color-text);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.status-bar-time {
  margin-left: auto;
  font-size: 11px;
  color: var(--color-text-dim);
  white-space: nowrap;
}

.seat-tag {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 5px;
  font-size: 12px;
  font-weight: 600;
}
.seat-tag.mouyuan {
  background: rgba(47, 191, 212, 0.14);
  color: #7fd9e8;
}
.seat-tag.jingshi {
  background: rgba(232, 163, 61, 0.14);
  color: #eec27f;
}
.seat-tag.chizheng {
  background: rgba(224, 92, 138, 0.15);
  color: #f0a0c0;
}

.title-tag-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px;
}

.topic-tag {
  background: rgba(96, 165, 250, 0.14);
  color: #93c5fd;
  border: 1px solid rgba(96, 165, 250, 0.4);
}

.report-topic-tag {
  color: var(--color-text-muted);
  font-size: 13px;
}

.workspace-main {
  min-width: 0;
}

@media (prefers-reduced-motion: reduce) {
  .status-bar-dot {
    animation: none;
    opacity: 0.7;
  }
}

/* ── 议题卷轴头 ── */
.edict {
  position: relative;
  padding-left: 20px;
}

.edict::before {
  content: '';
  position: absolute;
  left: 0;
  top: 8px;
  bottom: 8px;
  width: 3px;
  border-radius: 2px;
  background: linear-gradient(180deg, #c03a2b, rgba(192, 58, 43, 0.08));
}

.edict-head {
  display: flex;
  align-items: flex-start;
  gap: 16px;
  min-width: 0;
}

.edict-seal {
  margin-top: 10px;
}

.edict h1 {
  font-family: var(--font-display);
  font-size: 32px;
  letter-spacing: 1.5px;
  line-height: 1.3;
}

@media (max-width: 860px) {
  .edict {
    padding-left: 14px;
  }
  .edict h1 {
    font-size: 24px;
  }
}
</style>
