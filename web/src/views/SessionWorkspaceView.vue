<template>
  <section v-if="details" :class="['page', 'workspace', { 'report-mode': viewMode === 'report' }]">
    <header class="page-head row-head">
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
    <nav class="side-nav">
      <a href="#seats" title="三席状态"><span class="nav-dot" /><span>三席</span></a>
      <a href="#topic" title="议题"><span class="nav-dot" /><span>议题</span></a>
      <a v-if="externalEvidence.length > 0" href="#evidence" title="证据"><span class="nav-dot" /><span>证据</span></a>
      <a v-if="toolRuns.length > 0" href="#tools" title="工具轨迹"><span class="nav-dot" /><span>工具</span></a>
      <a href="#ideas" title="创意池"><span class="nav-dot" /><span>创意</span></a>
      <a href="#critiques" title="批议摘要"><span class="nav-dot" /><span>批议</span></a>
      <a href="#revisions" title="差异对比"><span class="nav-dot" /><span>差异</span></a>
      <a href="#proposals" title="策案对比"><span class="nav-dot" /><span>策案</span></a>
      <a href="#votes" title="投票"><span class="nav-dot" /><span>投票</span></a>
      <a href="#quality" title="讨论质量"><span class="nav-dot" /><span>质量</span></a>
      <a v-if="details?.artifacts.scribe_report" href="#deep-report" title="深度报告"><span class="nav-dot" /><span>深度</span></a>
      <a v-if="supportedClaims.length > 0" href="#claims" title="主张"><span class="nav-dot" /><span>主张</span></a>
      <a href="#stats" title="运行统计"><span class="nav-dot" /><span>统计</span></a>
      <a href="#timeline" title="时间线"><span class="nav-dot" /><span>时间</span></a>
      <a v-if="hasDecisionObjects" href="#decision-objects" title="决策对象"><span class="nav-dot" /><span>决策</span></a>
      <a v-if="hasFollowups" href="#followups" title="续议建议"><span class="nav-dot" /><span>续议</span></a>
      <a v-if="hasFollowupTurns" href="#followup-timeline" title="续议时间线"><span class="nav-dot" /><span>续议时间</span></a>
      <a v-if="hasDecisionObjects" href="#re-deliberation" title="新事实复议"><span class="nav-dot" /><span>复议</span></a>
    </nav>
    <div class="workspace-main">
    <PhaseProgressBar :phase="details.session.phase" />
    <section id="seats" class="role-card-row" aria-label="三席状态">
      <SeatRoleCard
        v-for="seat in seats"
        :key="seat"
        :seat="seat"
        :phase="details.session.phase"
        :events="details.events"
        :running="details.execution.running"
        :runs="details.artifacts.seat_runs"
        :tool-runs="toolRuns"
        :provider-ref="seatProviderRef(seat)"
        :inactive="details.session.mode === 'single_agent' && seat !== 'mouyuan'"
      />
    </section>

    <ApiErrorState :message="error" />
    <ApiErrorState v-if="details.session.failure_reason" :message="`失败原因：${details.session.failure_reason}`" />
    <section v-if="recentFailedRuns.length" class="panel">
      <h2>最近失败调用</h2>
      <ul class="failure-list">
        <li v-for="run in recentFailedRuns" :key="run.id">
          <span :class="['seat-tag', run.seat]">{{ seatLabels[run.seat] }} · {{ phaseLabels[run.phase] }}</span>
          <strong>{{ run.error || '模型返回内容无法解析' }}</strong>
        </li>
      </ul>
    </section>
    <div v-if="details.execution.recovery_state === 'retry_required'" class="status-bar status-bar-warn" role="status">
      <span class="status-bar-icon">&#9888;</span>
      <span>上次执行未正常完成，请使用重试继续。</span>
    </div>
    <div v-else-if="details.execution.recovery_state === 'paused'" class="status-bar status-bar-warn" role="status">
      <span class="status-bar-icon">&#9208;</span>
      <span>已暂停。你可以补充背景信息后继续。</span>
    </div>
    <div v-else-if="details.execution.running" class="status-bar status-bar-live" role="status" aria-live="polite">
      <span class="status-bar-dot" />
      <span class="status-bar-phase">{{ phaseLabels[details.session.phase] }}</span>
      <span class="status-bar-sep">·</span>
      <span class="status-bar-seat">{{ runningSeatLabel }}</span>
      <span v-if="runningActivityLabel" class="status-bar-tool">{{ runningActivityLabel }}</span>
      <span v-if="details.events?.length" class="status-bar-time">{{ lastEventTime }}</span>
    </div>

    <div v-if="viewMode === 'workbench' && currentDigest" class="digest-row">
      <DecisionDigest :digest="currentDigest" />
      <EvidenceSummary
        v-if="currentEvidenceSummary"
        element-id="quality"
        :summary="currentEvidenceSummary"
        :donut-segments="donutData"
        :radar-axes="radarData"
        :quality-metrics="qualityMetricRows(details.artifacts.quality, hasTokenUsage)"
      />
    </div>

    <DecisionSummary v-if="primaryDecision" :decision="primaryDecision" :vote-policy="details.session.vote_policy" :mode="details.session.mode" />

    <section id="topic" class="panel">
      <div class="row-head">
        <h2>议题</h2>
        <button v-if="!editingContext" class="icon" title="补充背景" @click="editingContext = true">
          <Pen :size="16" />
        </button>
      </div>
      <div class="formatted-text" v-html="renderReportText(details.session.topic)" />
      <template v-if="editingContext">
        <textarea v-model="newContext" rows="4" placeholder="补充背景信息…" style="margin-top: 12px" />
        <div class="actions" style="margin-top: 8px">
          <button @click="saveContext">保存</button>
          <button @click="editingContext = false">取消</button>
        </div>
      </template>
      <div v-else-if="renderReportText(details.session.context)" class="formatted-text muted" style="margin-top: 8px" v-html="renderReportText(details.session.context)" />
    </section>

    <section id="evidence" v-if="externalEvidence.length" class="panel evidence-source-panel">
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
          <div class="formatted-text" v-html="renderReportText(ev.content)" />
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

    <section id="tools" v-if="toolRuns.length" class="panel tool-run-panel">
      <div class="row-head">
        <h2>工具轨迹</h2>
        <button class="icon" title="展开详情" @click="showToolDetail = !showToolDetail">
          <ChevronDown v-if="!showToolDetail" :size="16" />
          <ChevronUp v-else :size="16" />
        </button>
      </div>
      <div v-if="!showToolDetail" class="tool-run-summary">
        <span v-for="(count, name) in toolRunSummaryData.by_tool" :key="name" class="tool-run-chip">
          {{ toolNameLabel(name) }} {{ count }} 次
        </span>
        <span class="tool-run-meta">{{ (toolRunSummaryData.total_ms / 1000).toFixed(1) }} 秒 · {{ toolRunSummaryData.failed }} 次失败</span>
      </div>
      <div v-else class="item-grid tool-run-grid">
        <article v-for="run in toolRuns" :key="run.id" class="item tool-run-item">
          <div class="item-head">
            <span>{{ toolNameLabel(run.tool_name) }}</span>
            <span :class="['badge', run.status === 'completed' ? 'ok' : 'warn']">{{ run.status }}</span>
          </div>
          <p>{{ reportText(run.input_summary) }}</p>
          <p class="muted">{{ (run.duration_ms / 1000).toFixed(1) }} 秒 · {{ run.evidence_ids?.length ?? 0 }} 条证据</p>
          <p v-if="run.error" class="muted tool-run-error">{{ run.error }}</p>
        </article>
      </div>
    </section>

    <section id="ideas" class="panel">
      <h2>创意池（{{ details.artifacts.ideas.length }}）</h2>
      <div class="item-grid">
        <IdeaCard v-for="idea in details.artifacts.ideas" :key="idea.id" :idea="idea" />
      </div>
    </section>

    <section id="critiques" class="panel">
      <h2>批议摘要</h2>
      <div class="item-grid">
        <article v-for="critique in details.artifacts.critiques" :key="`${critique.reviewer}-${critique.target_seat}`" class="item">
          <div class="item-head">
            <span :class="['seat-tag', critique.reviewer]">{{ seatLabels[critique.reviewer] }}</span> → <span :class="['seat-tag', critique.target_seat]">{{ seatLabels[critique.target_seat] }}</span>
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

    <section id="revisions" class="panel">
      <h2>独议 / 复议差异</h2>
      <div class="item-grid">
        <article v-for="diff in revisionDiffs(details)" :key="diff.seat" class="item">
          <div class="item-head">
            <span :class="['seat-tag', diff.seat]">{{ seatLabels[diff.seat] }}</span>
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

    <ProposalCompare id="proposals" :proposals="details.artifacts.proposals" />

    <VoteDisplay id="votes" :votes="details.artifacts.votes" :proposals="details.artifacts.proposals" />

    <VoteChanges id="vote-changes" :votes="details.artifacts.votes" :proposals="details.artifacts.proposals" />

    <section id="deep-report" v-if="details.artifacts.scribe_report" class="panel">
      <h2>深度研究报告</h2>
      <div class="scribe-report">
        <h3>共识总结</h3>
        <div class="formatted-text" v-html="renderReportText(details.artifacts.scribe_report.consensus_summary)" />
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
          <summary>研究全文</summary>
          <div class="scribe-final-report formatted-text" v-html="renderReportText(details.artifacts.scribe_report.final_report)" />
        </details>
      </div>
    </section>

    <section id="claims" v-if="supportedClaims.length" class="panel">
      <h2>有证据的主张</h2>
      <div class="item-grid">
        <article v-for="claim in supportedClaims" :key="claim.id" class="item">
          <div class="item-head">
            <span :class="['seat-tag', claim.proposed_by]">{{ seatLabels[claim.proposed_by] }}</span>
            <span class="badge ok">有证据</span>
          </div>
          <div class="formatted-text" v-html="renderReportText(claim.content)" />
          <p class="muted">来源：{{ reportText(claim.context) }}</p>
          <p v-if="detailEvidence(claim.evidence_ids)" class="muted">
            证据：{{ detailEvidence(claim.evidence_ids)?.map((ev) => evidenceKindLabels[ev.kind] + ': ' + reportText(ev.content)).join(' | ') }}
          </p>
        </article>
      </div>
    </section>
    <section v-if="unsupportedClaims.length" class="panel">
      <h2>未验证的主张</h2>
      <div class="item-grid">
        <article v-for="claim in unsupportedClaims" :key="claim.id" class="item">
          <div class="item-head">
            <span :class="['seat-tag', claim.proposed_by]">{{ seatLabels[claim.proposed_by] }}</span>
            <span class="badge warn">未验证</span>
          </div>
          <div class="formatted-text" v-html="renderReportText(claim.content)" />
          <p class="muted">来源：{{ reportText(claim.context) }}</p>
          <p v-if="detailEvidence(claim.evidence_ids)" class="muted">
            证据：{{ detailEvidence(claim.evidence_ids)?.map((ev) => evidenceKindLabels[ev.kind] + ': ' + reportText(ev.content)).join(' | ') }}
          </p>
        </article>
      </div>
    </section>

    <section id="stats" class="panel">
      <h2>运行统计</h2>
      <p v-if="details.artifacts.seat_runs.length && !hasTokenUsage" class="muted usage-note">
        当前 Provider 未返回 token usage；费用和额度请以供应商按调用次数或控制台账单为准。
      </p>
      <div class="stat-grid">
        <article v-for="stat in seatRunStats(details.artifacts.seat_runs)" :key="stat.seat" class="stat">
          <span :class="['seat-tag', stat.seat]">{{ seatLabels[stat.seat] }}</span>
          <strong>{{ stat.calls }} 次调用</strong>
          <p>
            {{ stat.durationMs ? (stat.durationMs / 1000).toFixed(1) : 0 }} 秒 · {{ stat.failed }} 次失败 · {{ stat.repaired }} 次修复
            <template v-if="stat.hasUsage"> · {{ stat.tokens }} tokens</template>
          </p>
          <p class="muted">{{ stat.promptVersions || '暂无 Prompt 版本' }}</p>
        </article>
      </div>
    </section>

    <section id="timeline" class="panel">
      <div class="row-head timeline-head">
        <h2>事件时间线</h2>
        <button v-if="!showTrajectory" class="stat-action" title="查看阶段轨迹" @click="loadTrajectory">
          <RotateCw :size="14" /> 查看阶段轨迹
        </button>
      </div>
      <div class="timeline-box">
        <ol class="timeline">
          <li v-for="event in timelineEvents" :key="event.id">
            <time v-if="event.created_at">{{ new Date(event.created_at).toLocaleString() }}</time>
            <span :class="['badge', eventBadge(event.event_type)]">{{ eventLabel(event) }}</span>
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

    <DecisionObjectsPanel
      id="decision-objects"
      v-if="hasDecisionObjects"
      :objects="decisionObjects"
      @resolve="handleResolveObject"
      @dismiss="handleDismissObject"
    />

    <FollowUpCards
      id="followups"
      v-if="hasFollowups"
      :suggestions="followupSuggestions"
      :loading="followupsLoading"
      @regenerate="handleRegenerateFollowups"
      @start="handleStartFollowup"
    />

    <FollowUpTimeline
      id="followup-timeline"
      v-if="hasFollowupTurns"
      :turns="followupTurns"
    />

    <ReDeliberationBox
      id="re-deliberation"
      v-if="hasDecisionObjects"
      :objects="decisionObjects"
      :running="reDelibRunning"
      :result="reDelibResult"
      :error-message="reDelibError"
      @submit="handleReDeliberate"
      @clear="reDelibError = ''"
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
import ApiErrorState from '../components/ApiErrorState.vue'
import CritiqueGraph from '../components/CritiqueGraph.vue'
import DecisionDigest from '../components/DecisionDigest.vue'
import DecisionSummary from '../components/DecisionSummary.vue'
import EvidenceSummary from '../components/EvidenceSummary.vue'
import ShareExportPanel from '../components/ShareExportPanel.vue'
import IdeaCard from '../components/IdeaCard.vue'
import DecisionObjectsPanel from '../components/DecisionObjectsPanel.vue'
import FollowUpCards from '../components/FollowUpCards.vue'
import FollowUpTimeline from '../components/FollowUpTimeline.vue'
import PhaseProgressBar from '../components/PhaseProgressBar.vue'
import ProposalCompare from '../components/ProposalCompare.vue'
import ReDeliberationBox from '../components/ReDeliberationBox.vue'

import ReportView from '../components/ReportView.vue'
import SeatRoleCard from '../components/SeatRoleCard.vue'
import VoteChanges from '../components/VoteChanges.vue'
import VoteDisplay from '../components/VoteDisplay.vue'
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

/* ── Digest row ── */
.digest-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  margin-bottom: 16px;
}

@media (max-width: 860px) {
  .digest-row {
    grid-template-columns: 1fr;
  }
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
  border: 1px solid rgba(15, 138, 161, 0.22);
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.58);
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
  background: #e2eef9;
  color: #1a5a8c;
}
.seat-tag.jingshi {
  background: #f0e6d3;
  color: #7a5a2e;
}
.seat-tag.chizheng {
  background: #f5e8e8;
  color: #8c3a3a;
}

.title-tag-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px;
}

.topic-tag {
  background: #e8f0fe;
  color: #1a5a8c;
  border: 1px solid #c6dafc;
}

.report-topic-tag {
  color: var(--color-text-muted);
  font-size: 13px;
}

.side-nav {
  position: fixed;
  right: 36px;
  top: 50%;
  transform: translateY(-50%);
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 10px 8px;
  z-index: 50;
  background: rgba(248, 250, 248, 0.5);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 14px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.06);
  width: auto;
  min-width: 28px;
  transition: min-width 250ms ease;
  overflow: hidden;
}
.side-nav:hover {
  min-width: 56px;
}
.side-nav a {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 6px;
  font-size: 11px;
  color: var(--color-text-muted);
  text-decoration: none;
  border-radius: 6px;
  line-height: 1.2;
  white-space: nowrap;
  transition: background 120ms, color 120ms;
}
.side-nav a:hover {
  background: rgba(255, 255, 255, 0.85);
  color: var(--color-text);
}
.nav-dot {
  flex: 0 0 6px;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--color-text-dim);
  opacity: 0.4;
  transition: opacity 150ms, background 150ms;
}
.side-nav a:hover .nav-dot {
  opacity: 0.8;
  background: var(--color-accent);
}
.workspace-main {
  min-width: 0;
}
section[id] {
  scroll-margin-top: 80px;
}
@media (max-width: 900px) {
  .side-nav {
    display: none;
  }
}

@media (prefers-reduced-motion: reduce) {
  .status-bar-dot {
    animation: none;
    opacity: 0.7;
  }
}
</style>
