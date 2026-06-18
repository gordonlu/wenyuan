export type SeatKind = 'mouyuan' | 'jingshi' | 'chizheng'
export type SessionPhase =
  | 'draft'
  | 'independent_deliberation'
  | 'cross_critique'
  | 'revision'
  | 'voting'
  | 'convergence'
  | 'completed'
  | 'failed'
  | 'cancelled'

export interface Proposal {
  id: string
  proposed_by: SeatKind
  title: string
  summary: string
  source_idea_ids?: string[]
  adopted_points?: string[]
  rejected_points?: string[]
  rejection_reasons?: string[]
  changes_from_initial?: string[]
  user_value: string
  implementation_path: string
  risks: string[]
  success_metrics: string[]
  confidence?: number
}

export interface Decision {
  status: 'majority_reached' | 'conditionally_adopted' | 'no_majority'
  selected_proposal?: Proposal | null
  vote_count: number
  majority_reasons: string[]
  minority_opinion: string[]
  adoption_conditions: string[]
  unresolved_questions: string[]
  next_steps: string[]
  self_vote_count: number
  minority_choices?: Array<{
    seat: SeatKind
    proposal_id: string
    reason: string
    reassessment_condition: string
    has_risk_warning: boolean
  }>
  reassessment_conditions?: string[]
  has_risk_blocker?: boolean
}

export interface SessionRecord {
  id: string
  title: string
  topic: string
  context: string
  mode: 'three_seat' | 'single_agent'
  phase: SessionPhase
  result?: Decision | null
  failure_reason?: string | null
}

export interface SessionSummary {
  id: string
  title: string
  mode: 'three_seat' | 'single_agent'
  phase: SessionPhase
  created_at: string
  updated_at: string
  has_majority: boolean
}

export interface SessionEvent {
  id: number
  event_type: string
  payload: unknown
  created_at: string
}

export type IdeaStatus = 'proposed' | 'expanded' | 'challenged' | 'merged' | 'shortlisted' | 'rejected' | 'adopted'

export type Critique = {
  reviewer: SeatKind
  target_seat: SeatKind
  strongest_point?: string
  weakest_point?: string
  hidden_assumption?: string
  challenge: string
  counterexample?: string
  suggested_improvement: string
  evidence_question?: string
}

export type IdeaCard = {
  id: string
  proposed_by: SeatKind
  source_seats?: SeatKind[]
  title: string
  summary: string
  value: string
  mechanism?: string
  unconventional?: boolean
  assumptions?: string[]
  risks?: string[]
  status?: IdeaStatus
  challenged_by?: string[]
  referenced_by_proposals?: string[]
  merged_into?: string | null
}

export type Vote = {
  voter: SeatKind
  proposal_id: string
  final_choice: boolean
  reason: string
  key_evidence?: string
  blocking_issue?: string
}

export interface DiscussionArtifacts {
  ideas: IdeaCard[]
  critiques: Critique[]
  proposals: Proposal[]
  votes: Vote[]
  seat_runs: SeatRunTrace[]
  decision?: Decision | null
  quality?: DiscussionQualityMetrics
  claims?: Array<{
    id: string
    proposed_by: SeatKind
    content: string
    context: string
    is_supported: boolean
    evidence_ids?: string[]
    assessment_ids?: string[]
  }>
  evidence?: Array<{
    id: string
    proposed_by: SeatKind
    kind: 'fact' | 'inference' | 'preference'
    content: string
    source: string
    source_fetched_at?: string | null
    source_hash?: string | null
    claim_ids?: string[]
  }>
  assessments?: Array<{
    id: string
    assessor: SeatKind
    evidence_id: string
    claim_id: string
    supports_claim: boolean
    reasoning: string
    confidence: number
  }>
  claim_evidence_links?: Array<{
    claim_id: string
    evidence_id: string
    link_type: string
  }>
  events: string[]
}

export interface DiscussionQualityMetrics {
  idea_duplicate_rate: number
  seat_similarity: number
  high_similarity_detected?: boolean
  critique_effectiveness_rate: number
  revision_change_rate: number
  self_vote_rate: number
  vote_concentration: number
  minority_retention_rate: number
  average_tokens: number
  average_duration_ms: number
}

export interface RevisionDiff {
  seat: SeatKind
  ideaTitles: string[]
  proposalTitle?: string
  summaryChanged: boolean
  titleChanged: boolean
  adoptedIdeaCount: number
  initialSummary: string
  revisedSummary: string
  addedImplementationPath?: string
  addedSuccessMetrics: string[]
}

export interface SeatRunTrace {
  id: string
  session_id: string
  seat: SeatKind
  phase: SessionPhase
  status: 'completed' | 'failed'
  prompt_version: string
  repair_attempted: boolean
  raw_output?: string | null
  error?: string | null
  duration_ms: number | string
  prompt_tokens?: number | null
  completion_tokens?: number | null
  total_tokens?: number | null
  upstream_status?: number | null
}

export interface SessionDetails {
  session: SessionRecord
  artifacts: DiscussionArtifacts
  seats?: SeatRecord[]
  execution: ExecutionInfo
  events: SessionEvent[]
}

export interface SeatRecord {
  session_id: string
  seat: SeatKind
  status: string
  last_error?: string | null
  system_prompt: string
  conversation: Array<{ role: string; content: string }>
  provider_ref: string
}

export interface ExecutionInfo {
  running: boolean
  lease_expires_at?: string | null
  recovery_state: 'idle' | 'running' | 'completed' | 'failed' | 'cancelled' | 'retry_required' | string
}

export interface ConfigStatus {
  provider_configured: boolean
  provider_kind: string
  model: string
  seat_models: Record<string, string>
  database_url: string
  version: string
  available_models: Array<{ value: string; label: string }>
  seat_available_models: Record<string, Array<{ value: string; label: string }>>
}

export const seatLabels: Record<SeatKind, string> = {
  mouyuan: '谋远席',
  jingshi: '经世席',
  chizheng: '持正席',
}

export const ideaStatusLabels: Record<string, string> = {
  proposed: '已提出',
  expanded: '已扩展',
  challenged: '受质疑',
  merged: '已合并',
  shortlisted: '入围策案',
  rejected: '已拒绝',
  adopted: '已采纳',
}

export const evidenceKindLabels: Record<string, string> = {
  fact: '事实',
  inference: '推断',
  preference: '偏好',
}

export const phaseLabels: Record<SessionPhase, string> = {
  draft: '待陈策',
  independent_deliberation: '独议中',
  cross_critique: '批议中',
  revision: '复议中',
  voting: '阁议中',
  convergence: '合案复议',
  completed: '已完成',
  failed: '失败',
  cancelled: '已取消',
}

export const modeLabels: Record<string, string> = {
  three_seat: '三席合议',
  single_agent: '单 Agent',
}

export function seatStatus(phase: SessionPhase, seat: SeatKind, events: SessionEvent[] = [], running = false) {
  const latest = [...events]
    .reverse()
    .find((event) => ['seat_started', 'seat_completed', 'seat_failed'].includes(event.event_type) && eventPayloadIncludesSeat(event.payload, seat))
  if (latest?.event_type === 'seat_failed') return '失败'
  if (latest?.event_type === 'seat_started') return running ? '思考中' : '进行中'
  if (latest?.event_type === 'seat_completed') return '已完成'
  if (phase === 'cancelled') return '已取消'
  if (phase === 'draft') return '待陈策'
  if (running && ['independent_deliberation', 'cross_critique', 'revision', 'voting', 'convergence'].includes(phase)) return '等待'
  if (phase === 'independent_deliberation') return '陈策中'
  if (phase === 'cross_critique') return '批议中'
  if (phase === 'revision') return '已复议'
  if (phase === 'voting' || phase === 'convergence') return '已投策'
  if (phase === 'completed') return '已投策'
  return '失败'
}

export function seatStatusClass(phase: SessionPhase, seat: SeatKind, events: SessionEvent[] = [], running = false) {
  const status = seatStatus(phase, seat, events, running)
  if (status === '思考中') return 'active'
  if (status === '等待' || status.endsWith('中')) return 'pending'
  if (status === '已完成' || status === '已复议' || status === '已投策') return 'ok'
  if (status === '失败' || status === '已取消') return 'danger'
  return ''
}

function eventPayloadIncludesSeat(payload: unknown, seat: SeatKind) {
  return typeof payload === 'object' && payload !== null && 'seat' in payload && (payload as { seat?: unknown }).seat === seat
}

export function hasMajority(details: SessionDetails) {
  const status = details.session.result?.status ?? details.artifacts.decision?.status
  return status === 'majority_reached' || status === 'conditionally_adopted'
}

export function seatRunStats(runs: SeatRunTrace[]) {
  return (['mouyuan', 'jingshi', 'chizheng'] as SeatKind[]).map((seat) => {
    const scoped = runs.filter((run) => run.seat === seat)
    return {
      seat,
      calls: scoped.length,
      failed: scoped.filter((run) => run.status === 'failed').length,
      repaired: scoped.filter((run) => run.repair_attempted && run.status === 'completed').length,
      durationMs: scoped.reduce((sum, run) => sum + Number(run.duration_ms || 0), 0),
      tokens: scoped.reduce((sum, run) => sum + (run.total_tokens ?? 0), 0),
      hasUsage: scoped.some((run) => typeof run.total_tokens === 'number'),
      promptVersions: Array.from(new Set(scoped.map((run) => run.prompt_version))).join(', '),
    }
  })
}

export function revisionDiffs(details: SessionDetails): RevisionDiff[] {
  return (['mouyuan', 'jingshi', 'chizheng'] as SeatKind[]).map((seat) => {
    const seatIdeas = details.artifacts.ideas.filter((idea) => idea.proposed_by === seat)
    const proposal = details.artifacts.proposals.find((item) => item.proposed_by === seat)
    const referencedIdeas = proposal?.source_idea_ids?.length
      ? seatIdeas.filter((idea) => proposal.source_idea_ids?.includes(idea.id))
      : seatIdeas
    const initialSummary = referencedIdeas.map((idea) => idea.summary).filter(Boolean).join('\n') || seatIdeas.map((idea) => idea.summary).filter(Boolean).join('\n')
    const initialTitle = referencedIdeas.map((idea) => idea.title).filter(Boolean).join(' / ') || seatIdeas.map((idea) => idea.title).filter(Boolean).join(' / ')
    const revisedSummary = proposal?.summary ?? ''
    const proposalTitle = proposal?.title

    return {
      seat,
      ideaTitles: referencedIdeas.map((idea) => idea.title),
      proposalTitle,
      summaryChanged: normalizeText(initialSummary) !== normalizeText(revisedSummary) || Boolean(proposal?.changes_from_initial?.length),
      titleChanged: normalizeText(initialTitle) !== normalizeText(proposalTitle ?? ''),
      adoptedIdeaCount: referencedIdeas.length,
      initialSummary,
      revisedSummary,
      addedImplementationPath: proposal?.implementation_path,
      addedSuccessMetrics: proposal?.success_metrics ?? [],
    }
  })
}

export function qualityMetricRows(metrics?: DiscussionQualityMetrics, hasUsage = true) {
  const safe = metrics ?? {
    idea_duplicate_rate: 0,
    seat_similarity: 0,
    high_similarity_detected: false,
    critique_effectiveness_rate: 0,
    revision_change_rate: 0,
    self_vote_rate: 0,
    vote_concentration: 0,
    minority_retention_rate: 0,
    average_tokens: 0,
    average_duration_ms: 0,
  }
  return [
    { label: 'Idea 重复率', value: formatPercent(safe.idea_duplicate_rate) },
    { label: '三席相似度', value: formatPercent(safe.seat_similarity) },
    { label: '高度重复检测', value: safe.high_similarity_detected ? '是' : '否' },
    { label: '批议有效率', value: formatPercent(safe.critique_effectiveness_rate) },
    { label: '复议修改率', value: formatPercent(safe.revision_change_rate) },
    { label: '自投率', value: formatPercent(safe.self_vote_rate) },
    { label: '票数集中度', value: formatPercent(safe.vote_concentration) },
    { label: '少数留议率', value: formatPercent(safe.minority_retention_rate) },
    { label: '平均 token', value: hasUsage ? String(Math.round(safe.average_tokens)) : '暂无' },
    { label: '平均耗时', value: `${Math.round(safe.average_duration_ms)} ms` },
  ]
}

export function exportSessionMarkdown(details: SessionDetails, level: 'brief' | 'standard' | 'audit' = 'brief') {
  const decision = details.session.result ?? details.artifacts.decision
  const lines = [
    `# ${markdownText(details.session.title)}`,
    '',
    `- 阶段：${phaseLabels[details.session.phase]}`,
    `- 模式：${modeLabels[details.session.mode]}`,
    `- Session：${details.session.id}`,
    '',
  ]

  if (level === 'brief') {
    lines.push('## 议题', '', markdownText(details.session.topic))
    if (details.session.context.trim()) {
      lines.push('', '### 背景', '', markdownText(details.session.context))
    }
    lines.push('', '## 表决结果', '')
    if (decision) {
      lines.push(`- 状态：${decisionLabel(decision)}`)
      if (decision.has_risk_blocker) lines.push('- 存在风险阻塞')
      if (decision.selected_proposal) {
        lines.push(`- 多数策案：${seatLabels[decision.selected_proposal.proposed_by]} - ${markdownText(decision.selected_proposal.title)}`)
      }
      lines.push(`- 有效票数：${decision.vote_count}`)
      lines.push(`- 自投数：${decision.self_vote_count}`)
      lines.push('')
      if (decision.majority_reasons.length) {
        lines.push('### 多数理由', '')
        for (const r of decision.majority_reasons) lines.push(`- ${markdownText(r)}`)
        lines.push('')
      }
      if (decision.minority_opinion.length || decision.minority_choices?.length) {
        lines.push('### 少数留议', '')
        for (const choice of decision.minority_choices ?? []) {
          lines.push(`- ${seatLabels[choice.seat]}：${markdownText(choice.reason)}${choice.has_risk_warning ? ' ⚠️' : ''}`)
        }
        if (!decision.minority_choices?.length) {
          for (const o of decision.minority_opinion) lines.push(`- ${markdownText(o)}`)
        }
        lines.push('')
      }
      if (decision.minority_choices?.length) {
        lines.push('### 重新评估条件', '')
        for (const choice of decision.minority_choices) {
          if (choice.reassessment_condition) lines.push(`- ${seatLabels[choice.seat]}：${markdownText(choice.reassessment_condition)}`)
        }
        lines.push('')
      }
      if (decision.adoption_conditions.length) {
        appendList(lines, '采纳条件', decision.adoption_conditions)
      }
      if (decision.next_steps.length) {
        appendList(lines, '下一步', decision.next_steps)
      }
    } else {
      lines.push('暂无表决结果。')
    }
    lines.push('')
    if (details.artifacts.proposals.length) {
      lines.push('## 策案', '')
      for (const proposal of details.artifacts.proposals) {
        lines.push(`### ${seatLabels[proposal.proposed_by]}：${markdownText(proposal.title)}`)
        lines.push('', markdownText(proposal.summary), '')
        if (proposal.confidence !== undefined) lines.push(`- 置信度：${formatPercent(proposal.confidence)}`)
        lines.push('')
      }
    }
    lines.push('## 讨论质量', '')
    const hasUsage = details.artifacts.seat_runs.some((run) => typeof run.total_tokens === 'number')
    for (const metric of qualityMetricRows(details.artifacts.quality, hasUsage)) {
      if (metric.label === '平均 token' || metric.label === '平均耗时' || metric.label === '票数集中度' || metric.label === '少数留议率') continue
      lines.push(`- ${metric.label}：${metric.value}`)
    }
    return `${lines.join('\n').replace(/\n{3,}/g, '\n\n')}\n`
  }

  lines.push('## 原始议题', '', markdownText(details.session.topic))

  if (details.session.context.trim()) {
    lines.push('', '### 背景', '', markdownText(details.session.context))
  }

  lines.push('', '## 三席独议', '')
  for (const idea of details.artifacts.ideas) {
    lines.push(`### ${seatLabels[idea.proposed_by]}：${markdownText(idea.title)}`)
    lines.push('', markdownText(idea.summary))
    lines.push('', `- 价值：${markdownText(idea.value)}`)
    if (idea.mechanism) lines.push(`- 机制：${markdownText(idea.mechanism)}`)
    if (idea.unconventional) lines.push('- 非主流方向：是')
    appendList(lines, '假设', idea.assumptions)
    appendList(lines, '风险', idea.risks)
    lines.push('')
  }

  lines.push('## 批议摘要', '')
  for (const critique of details.artifacts.critiques) {
    lines.push(`### ${seatLabels[critique.reviewer]} → ${seatLabels[critique.target_seat]}`)
    if (critique.strongest_point) lines.push(`- 最强点：${markdownText(critique.strongest_point)}`)
    if (critique.weakest_point) lines.push(`- 最弱点：${markdownText(critique.weakest_point)}`)
    if (critique.hidden_assumption) lines.push(`- 隐含假设：${markdownText(critique.hidden_assumption)}`)
    lines.push(`- 挑战：${markdownText(critique.challenge)}`)
    if (critique.counterexample) lines.push(`- 反例或失败条件：${markdownText(critique.counterexample)}`)
    lines.push(`- 改进建议：${markdownText(critique.suggested_improvement)}`, '')
    if (critique.evidence_question) lines.push(`- 需要补证：${markdownText(critique.evidence_question)}`)
  }

  lines.push('## 三个策案', '')
  for (const proposal of details.artifacts.proposals) {
    lines.push(`### ${seatLabels[proposal.proposed_by]}：${markdownText(proposal.title)}`)
    lines.push('', markdownText(proposal.summary), '')
    lines.push(`- 用户价值：${markdownText(proposal.user_value)}`)
    lines.push(`- 落地路径：${markdownText(proposal.implementation_path)}`)
    appendList(lines, '采纳观点', proposal.adopted_points)
    appendList(lines, '拒绝观点', proposal.rejected_points)
    appendList(lines, '拒绝理由', proposal.rejection_reasons)
    appendList(lines, '相较独议修改', proposal.changes_from_initial)
    appendList(lines, '风险', proposal.risks)
    appendList(lines, '成功指标', proposal.success_metrics)
    if (proposal.confidence !== undefined) lines.push(`- 置信度：${formatPercent(proposal.confidence)}`)
    lines.push('')
  }

  lines.push('## 表决结果', '')
  if (decision) {
    lines.push(`- 状态：${decisionLabel(decision)}`)
    lines.push(`- 有效票数：${decision.vote_count}`)
    lines.push(`- 自投数：${decision.self_vote_count}`)
    if (decision.has_risk_blocker) lines.push('- 存在风险阻塞')
    if (decision.selected_proposal) {
      lines.push(`- 多数策案：${seatLabels[decision.selected_proposal.proposed_by]} - ${markdownText(decision.selected_proposal.title)}`)
    }
    appendList(lines, '多数理由', decision.majority_reasons)
    appendList(lines, '少数留议', decision.minority_opinion)
    if (decision.minority_choices?.length) {
      for (const choice of decision.minority_choices) {
        lines.push(`- ${seatLabels[choice.seat]}：${markdownText(choice.reason)}${choice.has_risk_warning ? ' ⚠️含风险提醒' : ''}`)
        lines.push(`  - 重新评估：${markdownText(choice.reassessment_condition)}`)
      }
    }
    appendList(lines, '重新评估条件', decision.reassessment_conditions)
    appendList(lines, '采纳条件', decision.adoption_conditions)
    appendList(lines, '未决问题', decision.unresolved_questions)
    appendList(lines, '下一步', decision.next_steps)
  } else {
    lines.push('暂无表决结果。')
  }

  if (details.artifacts.votes.length > 0) {
    lines.push('', '### 逐席投票', '')
    for (const vote of details.artifacts.votes) {
      const proposal = details.artifacts.proposals.find((item) => item.id === vote.proposal_id)
      lines.push(`- ${seatLabels[vote.voter]}：${vote.final_choice ? '支持' : '不支持'}${proposal ? ` ${markdownText(proposal.title)}` : ''}。${markdownText(vote.reason)}`)
    }
  }

  lines.push('', '## 讨论质量指标', '')
  const hasUsage = details.artifacts.seat_runs.some((run) => typeof run.total_tokens === 'number')
  for (const metric of qualityMetricRows(details.artifacts.quality, hasUsage)) {
    lines.push(`- ${metric.label}：${metric.value}`)
  }

  if (level === 'audit') {
    if (details.artifacts.claims?.length) {
      lines.push('', '## 主张池（Audit）', '')
      for (const claim of details.artifacts.claims) {
        lines.push(`- ${claim.proposed_by}：${markdownText(claim.content)}${claim.is_supported ? ' ✅' : ' ❌'}`)
      }
    }
    if (details.artifacts.evidence?.length) {
      lines.push('', '## 证据池（Audit）', '')
      for (const ev of details.artifacts.evidence) {
        lines.push(`- [${ev.kind}] ${markdownText(ev.content)}${ev.source ? ` (${ev.source})` : ''}`)
      }
    }
    if (details.artifacts.assessments?.length) {
      lines.push('', '## 评估记录（Audit）', '')
      for (const a of details.artifacts.assessments) {
        lines.push(`- ${a.assessor} → ${a.claim_id}：${a.supports_claim ? '支持' : '反对'}`)
      }
    }
    if (details.artifacts.seat_runs.length) {
      lines.push('', '## 执行轨迹（Audit）', '')
      for (const run of details.artifacts.seat_runs) {
        lines.push(`- ${seatLabels[run.seat]} [${run.phase}]：${run.status}`, '')
        if (run.raw_output) {
          lines.push('  ```', `  ${run.raw_output.slice(0, 500).replace(/\n/g, '\n  ')}`, '  ```', '')
        }
      }
    }
  }

  return `${lines.join('\n').replace(/\n{3,}/g, '\n\n')}\n`
}

function decisionLabel(decision: Decision) {
  if (decision.status === 'majority_reached') return '形成多数'
  if (decision.status === 'conditionally_adopted') return '有条件通过'
  return '未形成多数'
}

function appendList(lines: string[], label: string, values?: string[]) {
  const cleaned = values?.map((value) => markdownText(value)).filter(Boolean) ?? []
  if (cleaned.length === 0) return
  lines.push(`- ${label}：`)
  for (const value of cleaned) {
    lines.push(`  - ${value}`)
  }
}

function markdownText(value: string) {
  return value.replace(/\r\n/g, '\n').trim()
}

function normalizeText(value: string) {
  return value.replace(/\s+/g, '').toLowerCase()
}

function formatPercent(value: number) {
  return `${Math.round(value * 100)}%`
}
