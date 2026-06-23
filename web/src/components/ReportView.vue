<template>
  <section>
    <div v-if="currentDigest" class="report-cover">
      <h1 class="report-cover-title">{{ details.session.title }}</h1>
      <div class="report-cover-meta">
        <span>{{ phaseLabels[details.session.phase] }}</span>
        <span>·</span>
        <span>{{ modeLabels[details.session.mode] }}</span>
        <span v-if="details.artifacts.topic_type" class="report-topic-tag">· {{ topicTypeLabel(details.artifacts.topic_type) }}</span>
        <span>·</span>
        <span>{{ currentDigest.evidence_total }} 项来源</span>
        <span>·</span>
        <span>{{ currentDigest.vote_count }} 票</span>
      </div>
      <div v-if="currentDigest.has_decision" class="report-cover-decision">
        <span :class="['badge', currentDigest.status_class === 'ok' ? 'ok' : 'warn']">
          {{ currentDigest.status_label }}
        </span>
        <span v-if="currentDigest.selected_proposal_title" class="report-cover-proposal">
          {{ currentDigest.selected_proposal_title }}
        </span>
      </div>
      <div v-if="currentDigest.has_risk_blocker || currentDigest.has_untrusted_external || currentDigest.has_injection_risk" class="report-cover-flags">
        <span v-if="currentDigest.has_risk_blocker" class="report-flag report-flag-warn">存在风险阻塞</span>
        <span v-if="currentDigest.has_untrusted_external" class="report-flag report-flag-warn">含不可信外部来源</span>
        <span v-if="currentDigest.has_injection_risk" class="report-flag report-flag-danger">检测到疑似注入</span>
      </div>
    </div>

    <section id="report-topic" class="panel report-topic">
      <h2>议题</h2>
      <p>{{ reportText(details.session.topic) }}</p>
      <p v-if="reportText(details.session.context)" class="muted">{{ reportText(details.session.context) }}</p>
    </section>

    <DecisionSummary v-if="primaryDecision" :decision="primaryDecision" :vote-policy="details.session.vote_policy" :mode="details.session.mode" />

    <ProposalCompare id="report-proposals" :proposals="details.artifacts.proposals" />

    <section id="report-evidence" v-if="scribeMode === 'full' && externalEvidence.length" class="panel evidence-source-panel">
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
          <p>{{ reportText(ev.content) }}</p>
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

    <section id="report-tools" v-if="scribeMode === 'full' && toolRuns.length" class="panel tool-run-panel">
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
          <p>{{ reportText(run.input_summary) }}</p>
          <p class="muted">{{ (run.duration_ms / 1000).toFixed(1) }} 秒 · {{ run.evidence_ids?.length ?? 0 }} 条证据</p>
          <p v-if="run.error" class="muted tool-run-error">{{ run.error }}</p>
        </article>
      </div>
    </section>

    <section v-if="scribeMode === 'full'" class="role-card-row report-seat-row" aria-label="三席报告">
      <SeatRoleCard
        v-for="seat in seats"
        :key="seat"
        :seat="seat"
        :phase="details.session.phase"
        :events="details.events"
        :running="false"
        :runs="details.artifacts.seat_runs"
        :tool-runs="toolRuns"
        :provider-ref="seatProviderRef(seat)"
        report-mode
      />
    </section>

    <VoteDisplay id="report-votes" :votes="details.artifacts.votes" :proposals="details.artifacts.proposals" />

    <VoteChanges v-if="scribeMode === 'full'" id="report-vote-changes" :votes="details.artifacts.votes" :proposals="details.artifacts.proposals" />

    <section id="report-quality" class="panel">
      <h2>讨论质量</h2>
      <div class="stat-grid">
        <article v-for="metric in qualityMetricRows(details.artifacts.quality, hasTokenUsage)" :key="metric.label" class="stat">
          <span>{{ metric.label }}</span>
          <strong>{{ metric.value }}</strong>
        </article>
      </div>
    </section>

    <section id="report-claims" v-if="scribeMode === 'full' && details.artifacts.claims?.length" class="panel">
      <h2>证据与待核验判断</h2>
      <div class="item-grid">
        <article v-for="claim in details.artifacts.claims" :key="claim.id" class="item">
          <div class="item-head">
            <span :class="['seat-tag', claim.proposed_by]">{{ seatLabels[claim.proposed_by] }}</span>
            <span :class="['badge', claim.is_supported ? 'ok' : 'warn']">
              {{ claim.is_supported ? '已有依据' : '仍需核验' }}
            </span>
          </div>
          <p>{{ reportText(claim.content) }}</p>
          <p class="muted">来源：{{ reportText(claim.context) }}</p>
          <p v-if="detailEvidence(claim.evidence_ids)" class="muted">
            证据：{{ detailEvidence(claim.evidence_ids)?.map((ev) => evidenceKindLabels[ev.kind] + ': ' + reportText(ev.content)).join(' | ') }}
          </p>
        </article>
      </div>
    </section>
  </section>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import DecisionSummary from './DecisionSummary.vue'
import ProposalCompare from './ProposalCompare.vue'
import SeatRoleCard from './SeatRoleCard.vue'
import VoteChanges from './VoteChanges.vue'
import VoteDisplay from './VoteDisplay.vue'
import { cleanReportText, decisionDigest, evidenceSafetyLabels, evidenceSourceKindLabels, evidenceSummary, evidenceTrustLabels, evidenceKindLabels, modeLabels, phaseLabels, qualityMetricRows, seatLabels, type EvidenceItem, type SeatKind, type SessionDetails, type ToolRun } from '../domain/session'

interface ClaimItem {
  id: string
  proposed_by: SeatKind
  content: string
  context: string
  is_supported: boolean
  evidence_ids?: string[]
  assessment_ids?: string[]
}

const props = defineProps<{
  details: SessionDetails
  scribeMode: string
  externalEvidence: EvidenceItem[]
  toolRuns: ToolRun[]
  supportedClaims: ClaimItem[]
  unsupportedClaims: ClaimItem[]
}>()

const seats: SeatKind[] = ['mouyuan', 'jingshi', 'chizheng']

const currentEvidenceSummary = computed(() => props.details ? evidenceSummary(props.details) : null)
const currentDigest = computed(() => {
  if (!props.details) return null
  const evSum = currentEvidenceSummary.value ?? undefined
  return decisionDigest(props.details, evSum)
})
const primaryDecision = computed(() => props.details?.session.result ?? props.details?.artifacts.decision ?? null)
const hasTokenUsage = computed(() => (props.details?.artifacts.seat_runs ?? []).some((run) => typeof (run as any).total_tokens === 'number'))

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

function reportText(value?: string | null) {
  return cleanReportText(value)
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

function toolNameLabel(name: string) {
  const labels: Record<string, string> = {
    web_search: '网页搜索',
    document_parse: '文档解析',
    code_search: '代码搜索',
  }
  return labels[name] ?? name
}

function detailEvidence(evidenceIds?: string[]) {
  if (!evidenceIds?.length || !props.details?.artifacts.evidence?.length) return []
  return props.details.artifacts.evidence.filter((ev) => evidenceIds.includes(ev.id))
}

function seatProviderRef(seat: SeatKind) {
  return props.details?.seats?.find((item) => item.seat === seat)?.provider_ref ?? ''
}
</script>

<style scoped>
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

.report-topic-tag {
  color: var(--color-text-muted);
  font-size: 13px;
}

section[id] {
  scroll-margin-top: 80px;
}
</style>
